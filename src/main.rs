// #![allow(dead_code, unused_variables, unused_mut, unused_imports)]

// project modules
mod common;
mod materials;
mod objects;
mod shaders;
mod vulkan_helper;
mod world;

use core::f32;

use ash::vk;
use cgmath::InnerSpace;
use common::Color;
// external imports
use image::{ImageBuffer, Rgba};
use materials::Material;
use rand::{Rng, SeedableRng};

#[cfg(debug_assertions)]
use renderdoc::{RenderDoc, V130};

// own imports
use crate::common::{Point3, Vec3};
use crate::objects::Sphere;
use crate::shaders::{finalize_shader, ray_trace_shader};
use crate::vulkan_helper::{RenderResources, VulkanBase};
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
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
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
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);

    // make the world
    println!("Generating world start");
    let world = randon_scene();
    println!("Generating world end");

    let buffer_content = unsafe {
        // init vulkan
        let vulkan_base = VulkanBase::new();

        // init renderdoc
        #[cfg(debug_assertions)]
        let mut rd: Option<RenderDoc<V130>> = RenderDoc::new().ok();
        #[cfg(debug_assertions)]
        if let Some(x) = rd.as_mut() {
            x.start_frame_capture(std::ptr::null(), std::ptr::null());
        }

        // create render resouurces
        let mut render_resources = RenderResources::init(
            &vulkan_base,
            IMAGE_WIDTH,
            IMAGE_HEIGHT,
            SAMPLES_PER_PIXEL,
            world,
        );

        // start command buffer
        let command_buffer_allocation_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(1)
            .command_pool(render_resources.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = vulkan_base
            .device
            .allocate_command_buffers(&command_buffer_allocation_info)
            .unwrap()[0];

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        vulkan_base
            .device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .unwrap();

        // initialize the pipeline
        vulkan_base.device.cmd_fill_buffer(
            command_buffer,
            render_resources.working_buffer_1.handle,
            0,
            render_resources.working_buffer_1.size,
            0,
        );
        vulkan_base.device.cmd_fill_buffer(
            command_buffer,
            render_resources.working_buffer_2.handle,
            0,
            render_resources.working_buffer_2.size,
            0,
        );
        let initial_buffer_memory_barriers = [
            vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
                .src_queue_family_index(0)
                .dst_queue_family_index(0)
                .buffer(render_resources.working_buffer_1.handle)
                .offset(0)
                .size(render_resources.working_buffer_1.size),
            vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
                .src_queue_family_index(0)
                .dst_queue_family_index(0)
                .buffer(render_resources.working_buffer_2.handle)
                .offset(0)
                .size(render_resources.working_buffer_2.size),
        ];
        let initial_dependency_info =
            vk::DependencyInfo::default().buffer_memory_barriers(&initial_buffer_memory_barriers);
        vulkan_base
            .device
            .cmd_pipeline_barrier2(command_buffer, &initial_dependency_info);
        vulkan_base.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            render_resources.render_pipeline,
        );
        vulkan_base.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            render_resources.render_pipeline_layout,
            1,
            &[render_resources.world_descriptor_set],
            &[],
        );

        // prepare push constant
        let mut push_constants = ray_trace_shader::PushConstantData {
            sphere_amount: (render_resources.world_gpu.geometry.count as u32).into(),
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

        // prepare sample barriers
        let buffer_1_write_wait = vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(render_resources.working_buffer_1.handle)
            .offset(0)
            .size(render_resources.working_buffer_1.size);
        let buffer_1_read_wait = vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(render_resources.working_buffer_1.handle)
            .offset(0)
            .size(render_resources.working_buffer_1.size);
        let buffer_2_write_wait = vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(render_resources.working_buffer_2.handle)
            .offset(0)
            .size(render_resources.working_buffer_2.size);
        let buffer_2_read_wait = vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(render_resources.working_buffer_2.handle)
            .offset(0)
            .size(render_resources.working_buffer_2.size);

        let barriers_1_to_2 = [buffer_1_read_wait, buffer_2_write_wait];
        let barriers_2_to_1 = [buffer_1_write_wait, buffer_2_read_wait];

        let dependency_1_to_2 =
            vk::DependencyInfo::default().buffer_memory_barriers(&barriers_1_to_2);
        let dependency_2_to_1 =
            vk::DependencyInfo::default().buffer_memory_barriers(&barriers_2_to_1);

        // record samples
        for i in 0..SAMPLES_PER_PIXEL {
            for i in 0..4 {
                push_constants.initial_seed[i] = rng.gen_range(u32::MIN..u32::MAX);
            }

            vulkan_base.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                render_resources.render_pipeline_layout,
                0,
                &[if i % 2 == 0 {
                    render_resources.work_descriptor_set_1
                } else {
                    render_resources.work_descriptor_set_2
                }],
                &[],
            );

            vulkan_base.device.cmd_push_constants(
                command_buffer,
                render_resources.render_pipeline_layout,
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

            vulkan_base.device.cmd_pipeline_barrier2(
                command_buffer,
                if i % 2 == 0 {
                    &dependency_1_to_2
                } else {
                    &dependency_2_to_1
                },
            );
        }

        // prepare image
        let image_init_barrier = [vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::NONE)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::GENERAL)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .image(render_resources.output_image)
            .subresource_range(render_resources.output_image_subresource)];
        let image_init_depencency =
            vk::DependencyInfo::default().image_memory_barriers(&image_init_barrier);
        vulkan_base
            .device
            .cmd_pipeline_barrier2(command_buffer, &image_init_depencency);

        // final processing
        vulkan_base.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            render_resources.finalize_pipeline,
        );
        vulkan_base.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            render_resources.finalize_pipeline_layout,
            0,
            &[render_resources.final_descriptor_set],
            &[],
        );
        let final_push_constant = finalize_shader::PushConstantData {
            sample_count: SAMPLES_PER_PIXEL as u32,
        };
        vulkan_base.device.cmd_push_constants(
            command_buffer,
            render_resources.finalize_pipeline_layout,
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
            .image(render_resources.output_image)
            .subresource_range(render_resources.output_image_subresource)];
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
            .image_extent(render_resources.output_image_extent)
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1),
            )];
        let copy_image_to_buffer_info = vk::CopyImageToBufferInfo2::default()
            .src_image(render_resources.output_image)
            .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .dst_buffer(render_resources.output_buffer.handle)
            .regions(&image_copy_regions);
        vulkan_base
            .device
            .cmd_copy_image_to_buffer2(command_buffer, &copy_image_to_buffer_info);

        // create fence
        let fence_create_info = vk::FenceCreateInfo::default();
        let fence = vulkan_base
            .device
            .create_fence(&fence_create_info, None)
            .unwrap();

        // submit command buffer
        vulkan_base
            .device
            .end_command_buffer(command_buffer)
            .unwrap();
        let submit_infos = [vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer)];
        let to_submit = [vk::SubmitInfo2::default().command_buffer_infos(&submit_infos)];
        println!("Render started");
        vulkan_base
            .device
            .queue_submit2(vulkan_base.queue, &to_submit, fence)
            .unwrap();
        vulkan_base
            .device
            .wait_for_fences(&[fence], true, u64::MAX)
            .unwrap();
        println!("Render finished");

        let image_data = render_resources.output_buffer.get_buffer_data();

        #[cfg(debug_assertions)]
        if let Some(x) = rd.as_mut() {
            x.end_frame_capture(std::ptr::null(), std::ptr::null());
        }

        // vulkan cleanup
        vulkan_base.device.device_wait_idle().unwrap();

        vulkan_base.device.destroy_fence(fence, None);

        render_resources.cleanup(&vulkan_base);

        vulkan_base.cleanup();

        image_data
    };

    let image =
        ImageBuffer::<Rgba<u8>, _>::from_raw(IMAGE_WIDTH, IMAGE_HEIGHT, &buffer_content[..])
            .unwrap();
    image.save("out.png").unwrap();
    println!("Everything worked!");
}
