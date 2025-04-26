// #![allow(dead_code, unused_variables, unused_mut, unused_imports)]

// project modules
mod common;
mod materials;
mod objects;
mod shaders;
mod vulkan_helper;
mod windowing_manager;
mod world;

use core::{f32, f64};

use ash::vk;
use ash::vk::DescriptorType;
use cgmath::{InnerSpace, Vector2, Vector4};
use common::Color;
// external imports
use itertools::Itertools;
use materials::Material;
use rand::{Rng, SeedableRng};

use sdl3::event::Event;
use sdl3::keyboard::{Keycode, Scancode};
use sdl3::mouse::MouseButton;

// Vulkan inports
use vk_mem::{Alloc, AllocationCreateInfo, AllocatorCreateFlags, MemoryUsage};
use vulkan_helper::{load_shader, Buffer};
use windowing_manager::WindowManager;
// own imports
use crate::common::{Point3, Vec3};
use crate::objects::Sphere;
use crate::shaders::{finalize_shader, ray_trace_shader};
use crate::vulkan_helper::VulkanBase;
use crate::world::WorldCpu;

// Generate a scene fileld with random spheres
fn randon_scene() -> WorldCpu {
    let mut out = WorldCpu::new();

    // the ground
    out.add_material(Material::Lambertian {
        albedo: Color::new(0.5, 0.5, 0.5),
    });
    out.add_geometry(Sphere {
        center: Vec3::new(0.0, -1000.0, 0.0),
        radius: 1000.0,
        material: 0,
        material_type: 0,
    });

    // the reandom speres
    let mut rng = //rand::thread_rng();
    rand::rngs::StdRng::from_seed([0;32]);
    for a in -11..11 {
        for b in -11..11 {
            let material_choice = rng.gen::<f32>();

            out.add_material(if material_choice < 0.8 {
                Material::Lambertian {
                    albedo: Color::new(
                        rng.gen::<f32>() * rng.gen::<f32>(),
                        rng.gen::<f32>() * rng.gen::<f32>(),
                        rng.gen::<f32>() * rng.gen::<f32>(),
                    ),
                }
            } else if material_choice < 0.95 {
                Material::Metal {
                    albedo: Color::new(rng.gen(), rng.gen(), rng.gen()),
                    fuzziness: rng.gen_range(0.0..0.5),
                }
            } else {
                Material::Dielectric { ir: 1.5 }
            });

            let center = Point3::new(
                (a as f32) + 0.9 * rng.gen::<f32>(),
                0.2,
                (b as f32) + 0.9 * rng.gen::<f32>(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).magnitude() > 0.9 {
                out.add_geometry(Sphere::new(
                    center,
                    0.2,
                    out.get_last_material_index(),
                    out.get_material_type(out.get_last_material_index()),
                ));
            }
        }
    }

    out.add_material(Material::Dielectric { ir: 1.5 });
    out.add_geometry(Sphere {
        center: Vec3::new(0.0, 1.0, 0.0),
        radius: 1.0,
        material: out.get_last_material_index(),
        material_type: 1,
    });

    out.add_material(Material::Lambertian {
        albedo: Color::new(0.4, 0.2, 0.1),
    });
    out.add_geometry(Sphere {
        center: Vec3::new(-4.0, 1.0, 0.0),
        radius: 1.0,
        material: out.get_last_material_index(),
        material_type: 0,
    });

    out.add_material(Material::Metal {
        albedo: Color::new(0.7, 0.6, 0.7),
        fuzziness: 0.0,
    });
    out.add_geometry(Sphere {
        center: Vec3::new(4.0, 1.0, 0.0),
        radius: 1.0,
        material: out.get_last_material_index(),
        material_type: 2,
    });

    out
}

const FULL_SUBRESOURCE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

unsafe fn setup_descriptor_pool(vulkan_base: &VulkanBase) -> vk::DescriptorPool {
    let descriptor_pool_sizes = [
        vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(7),
        vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1),
    ];
    let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
        .max_sets(4)
        .pool_sizes(&descriptor_pool_sizes);
    vulkan_base
        .device
        .create_descriptor_pool(&descriptor_pool_create_info, None)
        .unwrap()
}

unsafe fn create_render_target(
    width: u32,
    height: u32,
    vulkan_base: &VulkanBase,
    allocator: &vk_mem::Allocator,
) -> (vk::Image, vk_mem::Allocation, vk::ImageView) {
    let image_create_info = vk::ImageCreateInfo {
        image_type: vk::ImageType::TYPE_2D,
        format: vk::Format::R8G8B8A8_UNORM,
        extent: vk::Extent3D {
            width,
            height,
            depth: 1,
        },
        usage: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC,
        mip_levels: 1,
        array_layers: 1,
        samples: vk::SampleCountFlags::TYPE_1,
        initial_layout: vk::ImageLayout::UNDEFINED,
        tiling: vk::ImageTiling::OPTIMAL,
        ..Default::default()
    };
    let image_allocation_info = AllocationCreateInfo {
        usage: MemoryUsage::AutoPreferDevice,
        ..Default::default()
    };
    let (image, image_allocation) = allocator
        .create_image(&image_create_info, &image_allocation_info)
        .unwrap();
    let image_view_create_info = vk::ImageViewCreateInfo {
        image,
        subresource_range: FULL_SUBRESOURCE,
        format: vk::Format::R8G8B8A8_UNORM,
        view_type: vk::ImageViewType::TYPE_2D,
        ..Default::default()
    };
    let image_view = vulkan_base
        .device
        .create_image_view(&image_view_create_info, None)
        .unwrap();

    (image, image_allocation, image_view)
}

unsafe fn create_descriptor_set_layouts(
    vulkan_base: &VulkanBase,
) -> (vk::DescriptorSetLayout, vk::DescriptorSetLayout) {
    let buffer_binding_0 = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .stage_flags(vk::ShaderStageFlags::COMPUTE);
    let buffer_binding_1 = vk::DescriptorSetLayoutBinding::default()
        .binding(1)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .stage_flags(vk::ShaderStageFlags::COMPUTE);

    let as_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(2)
        .descriptor_count(1)
        .descriptor_type(DescriptorType::ACCELERATION_STRUCTURE_KHR)
        .stage_flags(vk::ShaderStageFlags::COMPUTE);

    let main_descriptor_set_layout_bindings = [buffer_binding_0, buffer_binding_1, as_binding];
    let main_descriptor_set_layout =
        vk::DescriptorSetLayoutCreateInfo::default().bindings(&main_descriptor_set_layout_bindings);
    let main_descriptor_set_layout = vulkan_base
        .device
        .create_descriptor_set_layout(&main_descriptor_set_layout, None)
        .unwrap();

    let image_binding_0 = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
        .stage_flags(vk::ShaderStageFlags::COMPUTE);
    let final_descriptor_set_layout_bindings = [image_binding_0];
    let final_descriptor_set_layout = vk::DescriptorSetLayoutCreateInfo::default()
        .bindings(&final_descriptor_set_layout_bindings);
    let final_descriptor_set_layout = vulkan_base
        .device
        .create_descriptor_set_layout(&final_descriptor_set_layout, None)
        .unwrap();
    (main_descriptor_set_layout, final_descriptor_set_layout)
}

unsafe fn update_descriptor_sets(
    vulkan_base: &VulkanBase,
    working_buffer_1: &Buffer<Vector4<f32>>,
    working_buffer_2: &Buffer<Vector4<f32>>,
    image_view: &vk::ImageView,
    work_descriptor_set_1: vk::DescriptorSet,
    work_descriptor_set_2: vk::DescriptorSet,
    final_descriptor_set: vk::DescriptorSet,
) {
    let work_buffer_1_descriptor_info = [vk::DescriptorBufferInfo::default()
        .buffer(working_buffer_1.handle)
        .range(working_buffer_1.size)];
    let work_descriptor_1_write_input = vk::WriteDescriptorSet::default()
        .dst_set(work_descriptor_set_1)
        .dst_binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .buffer_info(&work_buffer_1_descriptor_info);
    let work_descriptor_2_write_output = vk::WriteDescriptorSet::default()
        .dst_set(work_descriptor_set_2)
        .dst_binding(1)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .buffer_info(&work_buffer_1_descriptor_info);

    let work_buffer_2_descriptor_info = [vk::DescriptorBufferInfo::default()
        .buffer(working_buffer_2.handle)
        .range(working_buffer_2.size)];
    let work_descriptor_1_write_output = vk::WriteDescriptorSet::default()
        .dst_set(work_descriptor_set_1)
        .dst_binding(1)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .buffer_info(&work_buffer_2_descriptor_info);
    let work_descriptor_2_write_input = vk::WriteDescriptorSet::default()
        .dst_set(work_descriptor_set_2)
        .dst_binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .buffer_info(&work_buffer_2_descriptor_info);

    let output_image_descriptor_info = [vk::DescriptorImageInfo::default()
        .image_view(*image_view)
        .image_layout(vk::ImageLayout::GENERAL)];
    let final_descriptor_image_write = vk::WriteDescriptorSet::default()
        .dst_set(final_descriptor_set)
        .dst_binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
        .image_info(&output_image_descriptor_info);

    let descriptor_writes = [
        work_descriptor_1_write_input,
        work_descriptor_1_write_output,
        work_descriptor_2_write_input,
        work_descriptor_2_write_output,
        final_descriptor_image_write,
    ];
    vulkan_base
        .device
        .update_descriptor_sets(&descriptor_writes, &[]);
}

unsafe fn initialize_gpu_resources(
    vulkan_base: &VulkanBase,
    working_buffer_1: &Buffer<Vector4<f32>>,
    working_buffer_2: &Buffer<Vector4<f32>>,
    image: &vk::Image,
    present_images: &[vk::Image],
) {
    // start the command buffer
    let command_buffer = vulkan_base.start_command_buffer();

    vulkan_base.device.cmd_fill_buffer(
        command_buffer,
        working_buffer_1.handle,
        0,
        working_buffer_1.size,
        0,
    );
    vulkan_base.device.cmd_fill_buffer(
        command_buffer,
        working_buffer_2.handle,
        0,
        working_buffer_2.size,
        0,
    );

    let mut image_init_barriers = vec![vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::NONE)
        .src_access_mask(vk::AccessFlags2::NONE)
        .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
        .dst_access_mask(vk::AccessFlags2::SHADER_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::GENERAL)
        .src_queue_family_index(0)
        .dst_queue_family_index(0)
        .image(*image)
        .subresource_range(FULL_SUBRESOURCE)];

    present_images.iter().for_each(|present_image| {
        image_init_barriers.push(
            vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::NONE)
                .src_access_mask(vk::AccessFlags2::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(vk::AccessFlags2::NONE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .src_queue_family_index(0)
                .dst_queue_family_index(0)
                .image(*present_image)
                .subresource_range(FULL_SUBRESOURCE),
        )
    });
    submit_dependency(
        vulkan_base,
        &command_buffer,
        &[],
        image_init_barriers.as_slice(),
    );

    vulkan_base.submit_command_buffer(command_buffer, None);
}

unsafe fn create_pipeline_layout<T>(
    vulkan_base: &VulkanBase,
    set_layouts: &[vk::DescriptorSetLayout],
) -> vk::PipelineLayout {
    // let main_descriptor_set_layouts = [world_gpu.set_layout, main_descriptor_set_layout];
    let push_constant_ranges = [vk::PushConstantRange::default()
        .size(std::mem::size_of::<T>() as u32)
        .stage_flags(vk::ShaderStageFlags::COMPUTE)];
    let pipeline_layout_cerate_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(set_layouts)
        .push_constant_ranges(&push_constant_ranges);
    vulkan_base
        .device
        .create_pipeline_layout(&pipeline_layout_cerate_info, None)
        .unwrap()
}

unsafe fn dispatch_pipeline<T>(
    vulkan_base: &VulkanBase,
    command_buffer: &vk::CommandBuffer,
    pipeline: &vk::Pipeline,
    pipeline_layout: &vk::PipelineLayout,
    descriptor_sets: &[vk::DescriptorSet],
    push_constant: &T,
    render_size: vk::Extent2D,
) {
    vulkan_base.device.cmd_bind_pipeline(
        *command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        *pipeline,
    );
    vulkan_base.device.cmd_bind_descriptor_sets(
        *command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        *pipeline_layout,
        0,
        descriptor_sets,
        &[],
    );
    vulkan_base.device.cmd_push_constants(
        *command_buffer,
        *pipeline_layout,
        vk::ShaderStageFlags::COMPUTE,
        0,
        core::slice::from_raw_parts(
            (push_constant as *const T) as *const u8,
            core::mem::size_of::<T>(),
        ),
    );
    vulkan_base.device.cmd_dispatch(
        *command_buffer,
        render_size.width / 8,
        render_size.height / 8,
        1,
    );
}

unsafe fn submit_dependency(
    vulkan_base: &VulkanBase,
    command_buffer: &vk::CommandBuffer,
    buffer_barriers: &[vk::BufferMemoryBarrier2],
    image_barriers: &[vk::ImageMemoryBarrier2],
) {
    let dependency = vk::DependencyInfo::default()
        .image_memory_barriers(image_barriers)
        .buffer_memory_barriers(buffer_barriers);
    vulkan_base
        .device
        .cmd_pipeline_barrier2(*command_buffer, &dependency);
}

fn main() {
    println!("Program start");
    // image data
    const ASPECT_RATIO: f32 = 3.0 / 2.0;
    const IMAGE_WIDTH: u32 = 1200;
    #[allow(clippy::assertions_on_constants)]
    {
        assert!(IMAGE_WIDTH % 8 == 0); // needed for shader
    }
    const IMAGE_HEIGHT: u32 = (IMAGE_WIDTH as f32 / ASPECT_RATIO) as u32;
    const SAMPLES_PER_PIXEL: u32 = 500;

    // camera
    let mut camera = ray_trace_shader::Camera {
        look_from: Point3::new(13.0, 2.0, 3.0),
        look_angles: Vector2::new(f32::consts::FRAC_PI_2, 0.0),
        aspect_ratio: ASPECT_RATIO,
    };
    let movement_speed = 2.5;
    let mouse_sensitivity = 2.5;

    // generate initial rays
    let mut rng = rand::thread_rng();

    // make the world
    println!("Generating world start");
    let world = randon_scene();
    println!("Generating world end");

    unsafe {
        let windowing_manager = WindowManager::new(IMAGE_WIDTH, IMAGE_HEIGHT);
        let vulkan_base = &windowing_manager.vulkan_base;

        // load shaders
        let main_shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/ray_trace.comp.spv"));
        let main_shader_module = load_shader(vulkan_base, main_shader_bytes);
        let final_shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/finalize.comp.spv"));
        let final_shader_module = load_shader(vulkan_base, final_shader_bytes);

        //VMA setup
        let mut allocator_create_info = vk_mem::AllocatorCreateInfo::new(
            &vulkan_base.instance,
            &vulkan_base.device,
            vulkan_base.physical_device,
        );
        allocator_create_info.flags = AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;
        let allocator = vk_mem::Allocator::new(allocator_create_info).unwrap();

        // memory pools
        let descriptor_pool = setup_descriptor_pool(vulkan_base);

        // output image
        let (image, mut image_allocation, image_view) =
            create_render_target(IMAGE_WIDTH, IMAGE_HEIGHT, vulkan_base, &allocator);

        // working buffers
        let output_buffer_allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::AutoPreferDevice,
            flags: vk_mem::AllocationCreateFlags::DEDICATED_MEMORY,
            ..Default::default()
        };
        let working_buffer_1 = Buffer::<Vector4<f32>>::new(
            &allocator,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            (IMAGE_HEIGHT * IMAGE_WIDTH) as usize,
            output_buffer_allocation_info.clone(),
        );
        let working_buffer_2 = Buffer::<Vector4<f32>>::new(
            &allocator,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            (IMAGE_HEIGHT * IMAGE_WIDTH) as usize,
            output_buffer_allocation_info,
        );

        // gpu world
        let world_gpu = world.upload(vulkan_base, descriptor_pool, &allocator);

        // descriptor set layouts
        let (main_descriptor_set_layout, final_descriptor_set_layout) =
            create_descriptor_set_layouts(vulkan_base);

        // pipeline layouts
        let main_pipeline_layout = create_pipeline_layout::<ray_trace_shader::PushConstantData>(
            vulkan_base,
            &[world_gpu.set_layout, main_descriptor_set_layout],
        );
        let final_pipeline_layout = create_pipeline_layout::<finalize_shader::PushConstantData>(
            vulkan_base,
            &[world_gpu.set_layout, final_descriptor_set_layout],
        );

        // create pipelines
        let pipeline_cache_create_into = vk::PipelineCacheCreateInfo::default()
            .flags(vk::PipelineCacheCreateFlags::EXTERNALLY_SYNCHRONIZED)
            .initial_data(&[]);
        let pipeline_cache = vulkan_base
            .device
            .create_pipeline_cache(&pipeline_cache_create_into, None)
            .unwrap();

        let main_shader_pipeline_stage = vk::PipelineShaderStageCreateInfo::default()
            .module(main_shader_module)
            .name(c"main")
            .stage(vk::ShaderStageFlags::COMPUTE);
        let main_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
            .stage(main_shader_pipeline_stage)
            .layout(main_pipeline_layout);
        let final_shader_pipeline_stage = vk::PipelineShaderStageCreateInfo::default()
            .module(final_shader_module)
            .name(c"main")
            .stage(vk::ShaderStageFlags::COMPUTE);
        let final_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
            .stage(final_shader_pipeline_stage)
            .layout(final_pipeline_layout);
        let (main_pipeline, final_pipeline) = vulkan_base
            .device
            .create_compute_pipelines(
                pipeline_cache,
                &[main_pipeline_create_info, final_pipeline_create_info],
                None,
            )
            .unwrap()
            .into_iter()
            .collect_tuple()
            .unwrap();

        // create descriptor sets
        let desctiptor_sets_layouts = [
            main_descriptor_set_layout,
            main_descriptor_set_layout,
            final_descriptor_set_layout,
        ];
        let descriptor_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&desctiptor_sets_layouts);
        let (work_descriptor_set_1, work_descriptor_set_2, final_descriptor_set) = vulkan_base
            .device
            .allocate_descriptor_sets(&descriptor_allocate_info)
            .unwrap()
            .into_iter()
            .collect_tuple()
            .unwrap();

        // update descriptor sets
        update_descriptor_sets(
            vulkan_base,
            &working_buffer_1,
            &working_buffer_2,
            &image_view,
            work_descriptor_set_1,
            work_descriptor_set_2,
            final_descriptor_set,
        );

        // initialize data
        initialize_gpu_resources(
            vulkan_base,
            &working_buffer_1,
            &working_buffer_2,
            &image,
            &windowing_manager.present_images,
        );

        // prepare push constant
        let mut push_constants = ray_trace_shader::PushConstantData {
            sphere_amount: (world_gpu.geometry.count as u32).into(),
            initial_seed: [0, 0, 0, 0].into(),
            camera,
        };
        let mut final_push_constant = finalize_shader::PushConstantData { sample_count: 0 };

        let mut event_pump = windowing_manager.sdl_context.event_pump().unwrap();
        let mut last_fame_time = std::time::Instant::now();
        let mut last_mouse_position = None;
        let mut current_sample: u32 = 0;
        'running: loop {
            let mut reconstruct = false;
            // handle events
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => {
                        break 'running;
                    }
                    Event::MouseButtonDown {
                        mouse_btn: MouseButton::Left,
                        x,
                        y,
                        ..
                    } => {
                        last_mouse_position = Some(Vector2::new(x, y));
                    }
                    Event::MouseButtonUp {
                        mouse_btn: MouseButton::Left,
                        ..
                    } => {
                        last_mouse_position = None;
                    }
                    _ => {}
                }
            }
            // handle keyboar movement
            let mut to_move = Vec3::new(0.0, 0.0, 0.0);
            let keyboard_state = event_pump.keyboard_state();

            if keyboard_state.is_scancode_pressed(Scancode::A) {
                to_move += Vec3::new(0.0, 0.0, 1.0);
            }
            if keyboard_state.is_scancode_pressed(Scancode::D) {
                to_move -= Vec3::new(0.0, 0.0, 1.0);
            }
            if keyboard_state.is_scancode_pressed(Scancode::W) {
                to_move -= Vec3::new(1.0, 0.0, 0.0);
            }
            if keyboard_state.is_scancode_pressed(Scancode::S) {
                to_move += Vec3::new(1.0, 0.0, 0.0);
            }
            if keyboard_state.is_scancode_pressed(Scancode::Space) {
                to_move += Vec3::new(0.0, 1.0, 0.0);
            }
            if keyboard_state.is_scancode_pressed(Scancode::LShift) {
                to_move -= Vec3::new(0.0, 1.0, 0.0);
            }

            // calculate dt
            let dt = last_fame_time.elapsed().as_secs_f32();
            last_fame_time = std::time::Instant::now();

            if let Some(last_pos) = last_mouse_position {
                let mouse_status = event_pump.mouse_state();
                let delta_x = (last_pos.x - mouse_status.x()) / IMAGE_WIDTH as f32;
                let delta_y = (last_pos.y - mouse_status.y()) / IMAGE_HEIGHT as f32;

                camera.look_angles.y -= delta_x * mouse_sensitivity * dt;
                camera.look_angles.x += delta_y * mouse_sensitivity * dt;

                // restric vertical rotation
                camera.look_angles.x = camera
                    .look_angles
                    .x
                    .clamp(0.0001, f64::consts::PI as f32 - 0.0001);

                camera.look_angles.x %= f64::consts::TAU as f32;

                if delta_x != 0.0 && delta_y != 0.0 {
                    reconstruct = true;
                }
            }

            if to_move.magnitude2() != 0.0 {
                let to_move = to_move.normalize() * movement_speed * dt;
                let to_move = Vec3::new(
                    to_move.x * camera.look_angles.y.cos() - to_move.z * camera.look_angles.y.sin(),
                    to_move.y,
                    to_move.x * camera.look_angles.y.sin() + to_move.z * camera.look_angles.y.cos(),
                );
                camera.look_from += to_move;
                reconstruct = true;
            }

            // update push constants
            for i in 0..4 {
                push_constants.initial_seed[i] = rng.gen_range(u32::MIN..u32::MAX);
            }
            push_constants.camera = camera;

            // start new frame
            let (command_buffer, present_image, image_index) = windowing_manager.start_frame();

            // reset samples
            if reconstruct {
                current_sample = 0;

                vulkan_base.device.cmd_fill_buffer(
                    command_buffer,
                    working_buffer_1.handle,
                    0,
                    working_buffer_1.size,
                    0,
                );

                submit_dependency(
                    vulkan_base,
                    &command_buffer,
                    &[vk::BufferMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                        .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .dst_access_mask(vk::AccessFlags2::SHADER_READ)
                        .buffer(working_buffer_1.handle)
                        .offset(0)
                        .size(working_buffer_1.size)],
                    &[],
                );
            }

            // choose buffers
            let (current_work_set, dest_buffer) = if current_sample % 2 == 0 {
                (work_descriptor_set_1, &working_buffer_2)
            } else {
                (work_descriptor_set_2, &working_buffer_1)
            };
            final_push_constant.sample_count = (current_sample + 1).min(SAMPLES_PER_PIXEL);

            // render next sample
            if current_sample < SAMPLES_PER_PIXEL {
                dispatch_pipeline(
                    vulkan_base,
                    &command_buffer,
                    &main_pipeline,
                    &main_pipeline_layout,
                    &[current_work_set, world_gpu.descriptor_set],
                    &push_constants,
                    windowing_manager.swapchain_extent,
                );

                // make sure it finishes
                submit_dependency(
                    vulkan_base,
                    &command_buffer,
                    &[vk::BufferMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .src_access_mask(vk::AccessFlags2::SHADER_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .dst_access_mask(vk::AccessFlags2::SHADER_READ)
                        .buffer(dest_buffer.handle)
                        .offset(0)
                        .size(dest_buffer.size)],
                    &[],
                );

                current_sample += 1;
            }

            // generate image from current samples
            dispatch_pipeline(
                vulkan_base,
                &command_buffer,
                &final_pipeline,
                &final_pipeline_layout,
                &[current_work_set, final_descriptor_set],
                &final_push_constant,
                windowing_manager.swapchain_extent,
            );

            // prepare for image blit
            submit_dependency(
                vulkan_base,
                &command_buffer,
                &[],
                &[
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .src_access_mask(vk::AccessFlags2::SHADER_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::BLIT)
                        .dst_access_mask(vk::AccessFlags2::TRANSFER_READ)
                        .old_layout(vk::ImageLayout::GENERAL)
                        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .src_queue_family_index(0)
                        .dst_queue_family_index(0)
                        .image(image)
                        .subresource_range(FULL_SUBRESOURCE),
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .src_access_mask(vk::AccessFlags2::NONE)
                        .dst_stage_mask(vk::PipelineStageFlags2::BLIT)
                        .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                        .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .src_queue_family_index(0)
                        .dst_queue_family_index(0)
                        .image(present_image)
                        .subresource_range(FULL_SUBRESOURCE),
                ],
            );

            // copy to output
            let full_subresourcelayers = vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            };
            let full_offsets = [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: IMAGE_WIDTH as i32,
                    y: IMAGE_HEIGHT as i32,
                    z: 1,
                },
            ];
            let copy_regions = [vk::ImageBlit2::default()
                .src_subresource(full_subresourcelayers)
                .src_offsets(full_offsets)
                .dst_subresource(full_subresourcelayers)
                .dst_offsets(full_offsets)];
            let blit_image_info = vk::BlitImageInfo2::default()
                .src_image(image)
                .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .dst_image(present_image)
                .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .regions(&copy_regions)
                .filter(vk::Filter::NEAREST);
            vulkan_base
                .device
                .cmd_blit_image2(command_buffer, &blit_image_info);

            // prepare for present
            submit_dependency(
                vulkan_base,
                &command_buffer,
                &[],
                &[
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::BLIT)
                        .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .dst_access_mask(vk::AccessFlags2::NONE)
                        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .src_queue_family_index(0)
                        .dst_queue_family_index(0)
                        .image(present_image)
                        .subresource_range(FULL_SUBRESOURCE),
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::BLIT)
                        .src_access_mask(vk::AccessFlags2::TRANSFER_READ)
                        .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .dst_access_mask(vk::AccessFlags2::SHADER_WRITE)
                        .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .new_layout(vk::ImageLayout::GENERAL)
                        .src_queue_family_index(0)
                        .dst_queue_family_index(0)
                        .image(image)
                        .subresource_range(FULL_SUBRESOURCE),
                ],
            );

            // finish frame
            windowing_manager.finish_frame(command_buffer, image_index);
        }

        // vulkan cleanup
        vulkan_base.device.device_wait_idle().unwrap();
        world_gpu.cleanup(vulkan_base);

        allocator.destroy_image(image, &mut image_allocation);

        vulkan_base
            .device
            .destroy_descriptor_set_layout(final_descriptor_set_layout, None);
        vulkan_base
            .device
            .destroy_descriptor_set_layout(main_descriptor_set_layout, None);
        vulkan_base.device.destroy_pipeline(final_pipeline, None);
        vulkan_base.device.destroy_pipeline(main_pipeline, None);
        vulkan_base
            .device
            .destroy_pipeline_cache(pipeline_cache, None);
        vulkan_base
            .device
            .destroy_pipeline_layout(final_pipeline_layout, None);
        vulkan_base
            .device
            .destroy_pipeline_layout(main_pipeline_layout, None);
        vulkan_base.device.destroy_image_view(image_view, None);
        vulkan_base
            .device
            .destroy_descriptor_pool(descriptor_pool, None);
        vulkan_base
            .device
            .destroy_shader_module(final_shader_module, None);
        vulkan_base
            .device
            .destroy_shader_module(main_shader_module, None);
    }
}
