// #![allow(dead_code, unused_variables, unused_mut, unused_imports)]

// setup ffi
uniffi::setup_scaffolding!();

// project modules
mod common;
mod materials;
mod objects;
mod shaders;
mod vulkan_helper;
mod world;

use core::f32;
use std::io::Cursor;

use ash::vk::{self, DescriptorType};
use cgmath::{InnerSpace, Vector2, Vector4};
use common::Color;
// external imports
use image::{ImageBuffer, Rgba};
use itertools::Itertools;
use materials::Material;
use rand::{Rng, SeedableRng};

#[cfg(debug_assertions)]
use renderdoc::{RenderDoc, V130};

// Vulkan inports
use vk_mem::{Alloc, AllocationCreateInfo, AllocatorCreateFlags, MemoryUsage};
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
    // rand::rngs::StdRng::from_seed([0;32]);
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

    let image_init_barrier = [vk::ImageMemoryBarrier2::default()
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
    let image_init_depencency =
        vk::DependencyInfo::default().image_memory_barriers(&image_init_barrier);
    vulkan_base
        .device
        .cmd_pipeline_barrier2(command_buffer, &image_init_depencency);

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

unsafe fn render_sample(
    vulkan_base: &VulkanBase,
    pipeline: &vk::Pipeline,
    pipeline_layout: &vk::PipelineLayout,
    world_descriptor_set: &vk::DescriptorSet,
    work_descriptor_set: &vk::DescriptorSet,
    push_constant: &ray_trace_shader::PushConstantData,
    image_width: u32,
    image_height: u32,
) {
    let command_buffer = vulkan_base.start_command_buffer();

    vulkan_base
        .device
        .cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::COMPUTE, *pipeline);
    vulkan_base.device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        *pipeline_layout,
        0,
        &[*work_descriptor_set, *world_descriptor_set],
        &[],
    );

    vulkan_base.device.cmd_push_constants(
        command_buffer,
        *pipeline_layout,
        vk::ShaderStageFlags::COMPUTE,
        0,
        core::slice::from_raw_parts(
            (push_constant as *const ray_trace_shader::PushConstantData) as *const u8,
            core::mem::size_of::<ray_trace_shader::PushConstantData>(),
        ),
    );

    vulkan_base
        .device
        .cmd_dispatch(command_buffer, image_width / 8, image_height / 8, 1);

    vulkan_base.submit_command_buffer(command_buffer, None);
}

unsafe fn finalize_render(
    vulkan_base: &VulkanBase,
    pipeline: &vk::Pipeline,
    pipeline_layout: &vk::PipelineLayout,
    work_descriptor_set: &vk::DescriptorSet,
    final_descriptor_set: &vk::DescriptorSet,
    image: &vk::Image,
    output_buffer: &Buffer<u8>,
    sample_count: u32,
    image_width: u32,
    image_height: u32,
) {
    let command_buffer = vulkan_base.start_command_buffer();

    vulkan_base
        .device
        .cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::COMPUTE, *pipeline);
    vulkan_base.device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        *pipeline_layout,
        0,
        &[*work_descriptor_set, *final_descriptor_set],
        &[],
    );
    let final_push_constant = finalize_shader::PushConstantData { sample_count };
    vulkan_base.device.cmd_push_constants(
        command_buffer,
        *pipeline_layout,
        vk::ShaderStageFlags::COMPUTE,
        0,
        core::slice::from_raw_parts(
            (&final_push_constant as *const finalize_shader::PushConstantData) as *const u8,
            core::mem::size_of::<finalize_shader::PushConstantData>(),
        ),
    );
    vulkan_base
        .device
        .cmd_dispatch(command_buffer, image_width / 8, image_height / 8, 1);

    // get image back
    let final_image_barrier = [vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
        .src_access_mask(vk::AccessFlags2::SHADER_WRITE)
        .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
        .dst_access_mask(vk::AccessFlags2::TRANSFER_READ)
        .old_layout(vk::ImageLayout::GENERAL)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
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
    let final_image_depencency =
        vk::DependencyInfo::default().image_memory_barriers(&final_image_barrier);
    vulkan_base
        .device
        .cmd_pipeline_barrier2(command_buffer, &final_image_depencency);
    let image_copy_regions = [vk::BufferImageCopy2::default()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_offset(vk::Offset3D::default())
        .image_extent(vk::Extent3D {
            width: image_width,
            height: image_height,
            depth: 1,
        })
        .image_subresource(
            vk::ImageSubresourceLayers::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(0)
                .base_array_layer(0)
                .layer_count(1),
        )];
    let copy_image_to_buffer_info = vk::CopyImageToBufferInfo2::default()
        .src_image(*image)
        .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .dst_buffer(output_buffer.handle)
        .regions(&image_copy_regions);
    vulkan_base
        .device
        .cmd_copy_image_to_buffer2(command_buffer, &copy_image_to_buffer_info);

    vulkan_base.submit_command_buffer(command_buffer, None);
}

#[derive(uniffi::Record)]
pub struct RenderConfig {
    #[uniffi(default = 1200)]
    pub width: u32,

    #[uniffi(default = 1.5)]
    pub image_ratio: f32,

    #[uniffi(default = 500)]
    pub samples_per_pixel: i32,
}

#[uniffi::export]
pub fn render_image(config: RenderConfig) -> Vec<u8> {
    println!("Program start");
    // image data
    assert!(config.width % 8 == 0); // needed for shader
    let image_height: u32 = (config.width as f32 / config.image_ratio) as u32;

    // camera
    let look_from = Point3::new(13.0, 2.0, 3.0);

    // generate initial rays
    let mut rng = rand::thread_rng();

    // make the world
    println!("Generating world start");
    let world = randon_scene();
    println!("Generating world end");

    let buffer_content = unsafe {
        // init vulkan
        let vulkan_base = VulkanBase::new(&[]);

        // init renderdoc
        #[cfg(debug_assertions)]
        let mut rd: Option<RenderDoc<V130>> = RenderDoc::new().ok();
        #[cfg(debug_assertions)]
        if let Some(x) = rd.as_mut() {
            x.start_frame_capture(std::ptr::null(), std::ptr::null());
        }

        // load shaders
        let main_shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/ray_trace.comp.spv"));
        let main_shader_module = load_shader(&vulkan_base, main_shader_bytes);
        let final_shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/finalize.comp.spv"));
        let final_shader_module = load_shader(&vulkan_base, final_shader_bytes);

        //VMA setup
        let mut allocator_create_info = vk_mem::AllocatorCreateInfo::new(
            &vulkan_base.instance,
            &vulkan_base.device,
            vulkan_base.physical_device,
        );
        allocator_create_info.flags = AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;
        let allocator = vk_mem::Allocator::new(allocator_create_info).unwrap();

        // memory pools
        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(7),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(3),
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
            width: config.width,
            height: image_height,
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

        // output buffer
        let output_buffer_allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
            ..Default::default()
        };
        let mut output_buffer = Buffer::<u8>::new(
            &allocator,
            vk::BufferUsageFlags::TRANSFER_DST,
            (image_height * config.width * 4) as usize,
            output_buffer_allocation_info,
        );

        // working buffers
        let output_buffer_allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::AutoPreferDevice,
            flags: vk_mem::AllocationCreateFlags::DEDICATED_MEMORY,
            ..Default::default()
        };
        let working_buffer_1 = Buffer::<Vector4<f32>>::new(
            &allocator,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            (image_height * config.width) as usize,
            output_buffer_allocation_info.clone(),
        );
        let working_buffer_2 = Buffer::<Vector4<f32>>::new(
            &allocator,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            (image_height * config.width) as usize,
            output_buffer_allocation_info,
        );

        // gpu world
        let world_gpu = world.upload(&vulkan_base, descriptor_pool, &allocator);

        // descriptor set layouts
        let (main_descriptor_set_layout, final_descriptor_set_layout) =
            create_descriptor_set_layouts(&vulkan_base);

        // pipeline layouts
        let main_pipeline_layout = create_pipeline_layout::<ray_trace_shader::PushConstantData>(
            &vulkan_base,
            &[world_gpu.set_layout, main_descriptor_set_layout],
        );
        let final_pipeline_layout = create_pipeline_layout::<finalize_shader::PushConstantData>(
            &vulkan_base,
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
            &vulkan_base,
            &working_buffer_1,
            &working_buffer_2,
            &image_view,
            work_descriptor_set_1,
            work_descriptor_set_2,
            final_descriptor_set,
        );

        // initialize data
        initialize_gpu_resources(&vulkan_base, &working_buffer_1, &working_buffer_2, &image);

        // prepare push constant
        let mut push_constants = ray_trace_shader::PushConstantData {
            sphere_amount: (world_gpu.geometry.count as u32).into(),
            initial_seed: [0, 0, 0, 0].into(),
            camera: ray_trace_shader::Camera {
                look_from,
                aspect_ratio: config.image_ratio,
                look_angles: Vector2::new(2.0 * 0.710999, 2.0 * 0.113399),
            },
        };

        // record samples
        for i in 0..config.samples_per_pixel {
            for i in 0..4 {
                push_constants.initial_seed[i] = rng.gen_range(u32::MIN..u32::MAX);
            }

            render_sample(
                &vulkan_base,
                &main_pipeline,
                &main_pipeline_layout,
                &world_gpu.descriptor_set,
                if i % 2 == 0 {
                    &work_descriptor_set_1
                } else {
                    &work_descriptor_set_2
                },
                &push_constants,
                config.width,
                image_height,
            );
        }

        finalize_render(
            &vulkan_base,
            &final_pipeline,
            &final_pipeline_layout,
            if config.samples_per_pixel % 2 == 1 {
                &work_descriptor_set_1
            } else {
                &work_descriptor_set_2
            },
            &final_descriptor_set,
            &image,
            &output_buffer,
            config.samples_per_pixel as u32,
            config.width,
            image_height,
        );
        // wai on last submission
        vulkan_base
            .device
            .wait_for_fences(&[vulkan_base.fence], true, u64::MAX)
            .unwrap();

        let image_data = output_buffer.get_buffer_data();

        #[cfg(debug_assertions)]
        if let Some(x) = rd.as_mut() {
            x.end_frame_capture(std::ptr::null(), std::ptr::null());
        }

        // vulkan cleanup
        vulkan_base.device.device_wait_idle().unwrap();

        world_gpu.cleanup(&vulkan_base);

        allocator.destroy_image(image, &mut image_allocation);

        // vulkan_base.device.destroy_fence(fence, None);
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

        image_data
    };

    let image =
        ImageBuffer::<Rgba<u8>, _>::from_raw(config.width, image_height, &buffer_content[..])
            .unwrap();
    let mut output: Vec<u8> = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut output), image::ImageFormat::Bmp)
        .unwrap();
    output
}
