#![allow(dead_code, unused_variables, unused_mut, unused_imports)]

// project modules
mod common;
mod materials;
mod objects;
mod shaders;
mod vulkan_helper;
mod world;

use core::f32;

use ash::vk;
use cgmath::{InnerSpace, Vector4};
use common::Color;
// external imports
use itertools::Itertools;
use materials::Material;
use rand::Rng;

#[cfg(debug_assertions)]
use renderdoc::{RenderDoc, V130};

use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::surface;
// Vulkan inports
use vk_mem::{Alloc, AllocationCreateInfo, MemoryUsage};
use vulkan_helper::{load_shader, Buffer};
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
    let mut rng = rand::thread_rng();
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

unsafe fn initialize_gpu_resources(
    vulkan_base: &VulkanBase,
    working_buffer_1: &Buffer<Vector4<f32>>,
    working_buffer_2: &Buffer<Vector4<f32>>,
    image: &vk::Image,
    present_images: &Vec<vk::Image>,
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
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        })];

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
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }),
        )
    });

    let image_init_depencency =
        vk::DependencyInfo::default().image_memory_barriers(image_init_barriers.as_slice());
    vulkan_base
        .device
        .cmd_pipeline_barrier2(command_buffer, &image_init_depencency);

    vulkan_base.submit_command_buffer(command_buffer, None);
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
    const SAMPLES_PER_PIXEL: i32 = 500;

    // camera
    let look_from = Point3::new(13.0, 2.0, 3.0);
    // let look_from = Point3::new(10.0, 0.0, 0.0);
    let look_at = Point3::new(0.0, 0.0, 0.0);
    let up = Vec3::unit_y();
    let distance_to_focus = 10.0;
    let aperture = 0.1;

    // generate initial rays
    let mut rng = rand::thread_rng();

    // make the world
    println!("Generating world start");
    let world = randon_scene();
    println!("Generating world end");

    // SDL3 init
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Raytravcing in a weekend", IMAGE_WIDTH, IMAGE_HEIGHT)
        .position_centered()
        .vulkan()
        .build()
        .unwrap();

    unsafe {
        // init vulkan
        let sdl_extensions = window.vulkan_instance_extensions().unwrap();
        let vulkan_base = VulkanBase::new(&sdl_extensions);

        // init renderdoc
        #[cfg(debug_assertions)]
        let mut rd: Option<RenderDoc<V130>> = RenderDoc::new().ok();
        #[cfg(debug_assertions)]
        if let Some(x) = rd.as_mut() {
            x.start_frame_capture(std::ptr::null(), std::ptr::null());
        }

        // create surface
        let surface = window
            .vulkan_create_surface(vulkan_base.instance.handle())
            .unwrap();

        // get surface information for swapchain creation
        let surface_format = vulkan_base
            .surface_loader
            .get_physical_device_surface_formats(vulkan_base.physical_device, surface)
            .unwrap()[0]; // TODO: make some sort of optimal format finder
        let surface_capabilities = vulkan_base
            .surface_loader
            .get_physical_device_surface_capabilities(vulkan_base.physical_device, surface)
            .unwrap();

        let desired_image_count = 3;
        let desired_image_count = if surface_capabilities.max_image_count == 0 {
            desired_image_count.max(surface_capabilities.min_image_count)
        } else {
            desired_image_count.clamp(
                surface_capabilities.min_image_count,
                surface_capabilities.max_image_count,
            )
        };
        let surface_resolution = vk::Extent2D {
            width: IMAGE_WIDTH,
            height: IMAGE_HEIGHT,
        };
        //TODO: could this be a problem if it isn't identity??
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let supported_present_modes = vulkan_base
            .surface_loader
            .get_physical_device_surface_present_modes(vulkan_base.physical_device, surface)
            .unwrap();
        let present_mode = supported_present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        // create swapchain
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);
        let swapchain = vulkan_base
            .swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        // load shaders
        let main_shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/ray_trace.comp.spv"));
        let main_shader_module = load_shader(&vulkan_base, main_shader_bytes);
        let final_shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/finalize.comp.spv"));
        let final_shader_module = load_shader(&vulkan_base, final_shader_bytes);

        //VMA setup
        let allocator_create_info = vk_mem::AllocatorCreateInfo::new(
            &vulkan_base.instance,
            &vulkan_base.device,
            vulkan_base.physical_device,
        );
        let allocator = vk_mem::Allocator::new(allocator_create_info).unwrap();

        // memory pools
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
        let descriptor_pool = vulkan_base
            .device
            .create_descriptor_pool(&descriptor_pool_create_info, None)
            .unwrap();

        // output image
        let image_extent = vk::Extent3D {
            width: IMAGE_WIDTH,
            height: IMAGE_HEIGHT,
            depth: 1,
        };
        let image_create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_UNORM,
            extent: image_extent,
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
        let (image, mut image_allocation) = allocator
            .create_image(&image_create_info, &image_allocation_info)
            .unwrap();
        let image_subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };
        let image_view_create_info = vk::ImageViewCreateInfo {
            image,
            subresource_range: image_subresource_range,
            format: vk::Format::R8G8B8A8_UNORM,
            view_type: vk::ImageViewType::TYPE_2D,
            ..Default::default()
        };
        let image_view = vulkan_base
            .device
            .create_image_view(&image_view_create_info, None)
            .unwrap();

        // present images
        let present_images = vulkan_base
            .swapchain_loader
            .get_swapchain_images(swapchain)
            .unwrap();

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
        let world_gpu = world.upload(&vulkan_base, &allocator);

        // descriptor set layouts
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
        let main_descriptor_set_layout_bindings = [buffer_binding_0, buffer_binding_1];
        let main_descriptor_set_layout = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&main_descriptor_set_layout_bindings);
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

        // pipeline layouts
        let main_descriptor_set_layouts = [main_descriptor_set_layout, main_descriptor_set_layout];
        let main_push_constant_ranges = [vk::PushConstantRange::default()
            .size(std::mem::size_of::<ray_trace_shader::PushConstantData>() as u32)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)];
        let main_pipeline_layout_cerate_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&main_descriptor_set_layouts)
            .push_constant_ranges(&main_push_constant_ranges);
        let main_pipeline_layout = vulkan_base
            .device
            .create_pipeline_layout(&main_pipeline_layout_cerate_info, None)
            .unwrap();

        let final_descriptor_set_layouts =
            [main_descriptor_set_layout, final_descriptor_set_layout];
        let final_push_constant_ranges = [vk::PushConstantRange::default()
            .size(std::mem::size_of::<finalize_shader::PushConstantData>() as u32)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)];
        let final_pipeline_layout_cerate_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&final_descriptor_set_layouts)
            .push_constant_ranges(&final_push_constant_ranges);
        let final_pipeline_layout = vulkan_base
            .device
            .create_pipeline_layout(&final_pipeline_layout_cerate_info, None)
            .unwrap();

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
            main_descriptor_set_layout,
            final_descriptor_set_layout,
        ];
        let descriptor_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&desctiptor_sets_layouts);
        let (
            world_descriptor_set,
            work_descriptor_set_1,
            work_descriptor_set_2,
            final_descriptor_set,
        ) = vulkan_base
            .device
            .allocate_descriptor_sets(&descriptor_allocate_info)
            .unwrap()
            .into_iter()
            .collect_tuple()
            .unwrap();

        // update descriptor sets
        let geometry_buffer_descriptor_info = [vk::DescriptorBufferInfo::default()
            .buffer(world_gpu.geometry.handle)
            .range(world_gpu.geometry.size)];
        let world_descriptor_write_geometry = vk::WriteDescriptorSet::default()
            .dst_set(world_descriptor_set)
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&geometry_buffer_descriptor_info);
        let material_buffer_descriptor_info = [vk::DescriptorBufferInfo::default()
            .buffer(world_gpu.materials.handle)
            .range(world_gpu.materials.size)];
        let world_descriptor_write_material = vk::WriteDescriptorSet::default()
            .dst_set(world_descriptor_set)
            .dst_binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&material_buffer_descriptor_info);

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
            .image_view(image_view)
            .image_layout(vk::ImageLayout::GENERAL)];
        let final_descriptor_image_write = vk::WriteDescriptorSet::default()
            .dst_set(final_descriptor_set)
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&output_image_descriptor_info);

        let descriptor_writes = [
            world_descriptor_write_material,
            world_descriptor_write_geometry,
            work_descriptor_1_write_input,
            work_descriptor_1_write_output,
            work_descriptor_2_write_input,
            work_descriptor_2_write_output,
            final_descriptor_image_write,
        ];
        vulkan_base
            .device
            .update_descriptor_sets(&descriptor_writes, &[]);

        // initialize data
        initialize_gpu_resources(
            &vulkan_base,
            &working_buffer_1,
            &working_buffer_2,
            &image,
            &present_images,
        );

        // prepare push constant
        let mut push_constants = ray_trace_shader::PushConstantData {
            sphere_amount: (world_gpu.geometry.count as u32).into(),
            initial_seed: [0, 0, 0, 0].into(),
            camera: ray_trace_shader::Camera {
                look_from,
                look_at,
                up,
                vfov: 20.0 * f32::consts::PI / 180.0,
                aspect_ratio: ASPECT_RATIO,
                apeture: aperture,
                focus_distance: distance_to_focus,
            },
        };
        let final_push_constant = finalize_shader::PushConstantData {
            sample_count: 1, // SAMPLES_PER_PIXEL as u32,
        };

        // setup semaphores
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let image_acquire_semaphore = vulkan_base
            .device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap();
        let rendering_completed_semaphore = vulkan_base
            .device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap();

        let mut event_pump = sdl_context.event_pump().unwrap();
        'running: loop {
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
                    _ => {}
                }
            }

            // update random seeds
            for i in 0..4 {
                push_constants.initial_seed[i] = rng.gen_range(u32::MIN..u32::MAX);
            }

            // wait on last command to finish
            vulkan_base
                .device
                .wait_for_fences(&[vulkan_base.fence], true, u64::MAX)
                .unwrap();
            vulkan_base
                .device
                .reset_fences(&[vulkan_base.fence])
                .unwrap();

            // get image
            let (image_index, _) = vulkan_base
                .swapchain_loader
                .acquire_next_image(
                    swapchain,
                    u64::MAX,
                    image_acquire_semaphore,
                    vk::Fence::null(),
                )
                .unwrap();

            // start command buffer
            let command_buffer = vulkan_base.start_command_buffer();

            // render sample
            vulkan_base.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                main_pipeline,
            );
            vulkan_base.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                main_pipeline_layout,
                0,
                &[work_descriptor_set_1, world_descriptor_set],
                &[],
            );
            vulkan_base.device.cmd_push_constants(
                command_buffer,
                main_pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                core::slice::from_raw_parts(
                    (&push_constants as *const ray_trace_shader::PushConstantData) as *const u8,
                    core::mem::size_of::<ray_trace_shader::PushConstantData>(),
                ),
            );
            vulkan_base
                .device
                .cmd_dispatch(command_buffer, IMAGE_WIDTH / 8, IMAGE_HEIGHT / 8, 1);

            // make sure it finishes
            let inter_shader_buffer_barriers = [vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .src_access_mask(vk::AccessFlags2::SHADER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_READ)
                .buffer(working_buffer_2.handle)
                .offset(0)
                .size(working_buffer_2.size)];
            let inter_shader_dependency =
                vk::DependencyInfo::default().buffer_memory_barriers(&inter_shader_buffer_barriers);
            vulkan_base
                .device
                .cmd_pipeline_barrier2(command_buffer, &inter_shader_dependency);

            // generate image from current sample
            vulkan_base.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                final_pipeline,
            );
            vulkan_base.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                final_pipeline_layout,
                0,
                &[work_descriptor_set_1, final_descriptor_set],
                &[],
            );
            vulkan_base.device.cmd_push_constants(
                command_buffer,
                final_pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                core::slice::from_raw_parts(
                    (&final_push_constant as *const finalize_shader::PushConstantData) as *const u8,
                    core::mem::size_of::<finalize_shader::PushConstantData>(),
                ),
            );
            vulkan_base
                .device
                .cmd_dispatch(command_buffer, IMAGE_WIDTH / 8, IMAGE_HEIGHT / 8, 1);

            // prepare for image blit
            let image_after_barrier = [
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
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    ),
                vk::ImageMemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                    .src_access_mask(vk::AccessFlags2::NONE)
                    .dst_stage_mask(vk::PipelineStageFlags2::BLIT)
                    .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_queue_family_index(0)
                    .dst_queue_family_index(0)
                    .image(present_images[image_index as usize])
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    ),
            ];
            let after_dependency =
                vk::DependencyInfo::default().image_memory_barriers(&image_after_barrier);
            vulkan_base
                .device
                .cmd_pipeline_barrier2(command_buffer, &after_dependency);

            // copy to output
            let copy_regions = [vk::ImageBlit2::default()
                .src_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: IMAGE_WIDTH as i32,
                        y: IMAGE_HEIGHT as i32,
                        z: 1,
                    },
                ])
                .dst_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: IMAGE_WIDTH as i32,
                        y: IMAGE_HEIGHT as i32,
                        z: 1,
                    },
                ])];
            let blit_image_info = vk::BlitImageInfo2::default()
                .src_image(image)
                .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .dst_image(present_images[image_index as usize])
                .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .regions(&copy_regions)
                .filter(vk::Filter::NEAREST);
            vulkan_base
                .device
                .cmd_blit_image2(command_buffer, &blit_image_info);

            // prepare for present
            let image_present_barriers = [
                vk::ImageMemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::BLIT)
                    .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                    .dst_access_mask(vk::AccessFlags2::NONE)
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .src_queue_family_index(0)
                    .dst_queue_family_index(0)
                    .image(present_images[image_index as usize])
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    ),
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
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    ),
            ];
            let present_dependency =
                vk::DependencyInfo::default().image_memory_barriers(&image_present_barriers);
            vulkan_base
                .device
                .cmd_pipeline_barrier2(command_buffer, &present_dependency);

            // submit command buffer
            vulkan_base
                .device
                .end_command_buffer(command_buffer)
                .unwrap();

            let submit_infos =
                [vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer)];
            let wait_semaphore_infos = [vk::SemaphoreSubmitInfo::default()
                .semaphore(image_acquire_semaphore)
                .stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)];
            let signal_semaphore_infos = [vk::SemaphoreSubmitInfo::default()
                .semaphore(rendering_completed_semaphore)
                .stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)];
            let to_submit = [vk::SubmitInfo2::default()
                .command_buffer_infos(&submit_infos)
                .signal_semaphore_infos(&signal_semaphore_infos)
                .wait_semaphore_infos(&wait_semaphore_infos)];
            vulkan_base
                .device
                .queue_submit2(vulkan_base.queue, &to_submit, vulkan_base.fence)
                .unwrap();

            // present image
            let wait_semaphores = [rendering_completed_semaphore];
            let swapchains = [swapchain];
            let imaage_indices = [image_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&wait_semaphores)
                .swapchains(&swapchains)
                .image_indices(&imaage_indices);
            vulkan_base
                .swapchain_loader
                .queue_present(vulkan_base.queue, &present_info)
                .unwrap();
        }

        #[cfg(debug_assertions)]
        if let Some(x) = rd.as_mut() {
            x.end_frame_capture(std::ptr::null(), std::ptr::null());
        }

        // vulkan cleanup
        vulkan_base.device.device_wait_idle().unwrap();

        allocator.destroy_image(image, &mut image_allocation);

        vulkan_base
            .device
            .destroy_semaphore(rendering_completed_semaphore, None);
        vulkan_base
            .device
            .destroy_semaphore(image_acquire_semaphore, None);
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
        vulkan_base
            .swapchain_loader
            .destroy_swapchain(swapchain, None);
        vulkan_base.surface_loader.destroy_surface(surface, None);
    }
}
