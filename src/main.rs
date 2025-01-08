#![allow(dead_code, unused_variables, unused_mut)]

// project modules
mod common;
mod compute_shader;
mod vulkan_helper;
mod objects;
mod materials;

use core::f32;

// external imports
use image::{ImageBuffer, Rgba};
use materials::Material;
use rand::Rng;

// Vulkan inports
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{
        allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
        AutoCommandBufferBuilder, CommandBufferUsage, CopyImageToBufferInfo,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    format::Format,
    image::{view::ImageView, ImageDimensions, StorageImage},
    memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator},
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    sync::{self, GpuFuture},
};
// own imports
use crate::objects::Sphere;
use crate::common::{Point3, Vec3};
use crate::vulkan_helper::{get_logical_device, get_physical_device, get_vulkan_instance};


fn main() {
    println!("Starting the renderer");
    // image data
    const ASPECT_RATIO: f32 = 16.0 / 9.0;
    const IMAGE_WIDTH: u32 = 400;
    assert!(IMAGE_WIDTH%8 == 0); // needed for shader
    const IMAGE_HEIGHT: u32 = (IMAGE_WIDTH as f32 / ASPECT_RATIO) as u32;
    const SAMPLES_PER_PIXEL: i32 = 10;

    // camera
    // let look_from = Point3::new(13.0, 2.0, 3.0);
    let look_from = Point3::new(10.0, 0.0, 0.0);
    let look_at = Point3::new(0.0, 0.0, 0.0);
    let up = Vec3::unit_y();
    let distance_to_focus = 10.0;
    let aperture = 0.1;

    // generate initial rays
    let mut rng = rand::thread_rng();

    // make the world
    let materials: Vec<[f32;4]> = vec![
        Material::Lambertian { albedo: [1.0, 0.0, 0.0].into() }.into(),
        Material::Dielectric { ir: 1.5 }.into(),
        Material::Metal { albedo: [0.0, 0.0, 1.0].into(), fuzziness: 1.0 }.into(),
    ];

    let spheres: Vec<compute_shader::Sphere> = vec![
        Sphere::new([0.0, 0.0, 1.2].into(), 0.5, 0, 0).into(),
        Sphere::new([0.0, 0.0, -1.2].into(), 0.5, 1, 1).into(),
        Sphere::new([0.0, 0.0, 0.0].into(), 0.5, 2, 2 ).into(),
    ];
    let sphere_amount = spheres.len() as u32;

    // init vulkan
    let instance = get_vulkan_instance();
    let physical_device = get_physical_device(instance);
    println!("Chosen {}", physical_device.properties().device_name);
    let (device, queue) = get_logical_device(physical_device);

    // load shader
    let shader = compute_shader::load(device.clone()).expect("Failed to load shader module");

    // create a memory allocator
    let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

    // output image
    let output_image_1 = StorageImage::new(
        &memory_allocator,
        ImageDimensions::Dim2d {
            width: IMAGE_WIDTH,
            height: IMAGE_HEIGHT,
            array_layers: 1,
        },
        Format::R8G8B8A8_UNORM,
        Some(queue.queue_family_index()),
    )
    .unwrap();
    let output_image_2 =  StorageImage::new(
        &memory_allocator,
        ImageDimensions::Dim2d {
            width: IMAGE_WIDTH,
            height: IMAGE_HEIGHT,
            array_layers: 1,
        },
        Format::R8G8B8A8_UNORM,
        Some(queue.queue_family_index()),
    )
    .unwrap();

    // output data buffer
    let output_buff = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            usage: MemoryUsage::Download,
            ..Default::default()
        },
        (0..IMAGE_WIDTH * IMAGE_HEIGHT * 4).map(|_| 0u8),
    )
    .expect("failed to create buffer");
    let view_1 = ImageView::new_default(output_image_1.clone()).unwrap();
    let view_2 = ImageView::new_default(output_image_2.clone()).unwrap();

    // Sphere buffer
    let sphere_buffer = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo{
            usage: BufferUsage::STORAGE_BUFFER, 
            ..Default::default()
        },
        AllocationCreateInfo{
            usage: MemoryUsage::Upload,
            ..Default::default()
        }, 
        spheres.into_iter()
    ).unwrap();

    // Material buffer
    let material_buffer = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo{
            usage: BufferUsage::STORAGE_BUFFER, 
            ..Default::default()
        },
        AllocationCreateInfo{
            usage: MemoryUsage::Upload,
            ..Default::default()
        }, 
        materials.into_iter()
    ).unwrap();

    // create pipeline
    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        shader.entry_point("main").unwrap(),
        &(),
        None,
        |_| {},
    )
    .expect("failed to create compute pipeline");

    // descriptor sets
    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
    let pipeline_layout = compute_pipeline.layout();
    let descriptor_set_layouts = pipeline_layout.set_layouts();

    let descriptor_set_layout = descriptor_set_layouts.get(0).unwrap();
    let descriptor_set_1 = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        [
            WriteDescriptorSet::image_view(0, view_1.clone()),
            WriteDescriptorSet::image_view(1, view_2.clone()),
            WriteDescriptorSet::buffer(2, sphere_buffer.clone()),
            WriteDescriptorSet::buffer(3, material_buffer.clone()),
        ],
    )
    .unwrap();
    let descriptor_set_2 = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        [
            WriteDescriptorSet::image_view(0, view_2.clone()),
            WriteDescriptorSet::image_view(1, view_1.clone()),
            WriteDescriptorSet::buffer(2, sphere_buffer.clone()),
            WriteDescriptorSet::buffer(3, material_buffer.clone()),
        ],
    )
    .unwrap();

    // command buffer allocator
    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );

    // command buffer builder
    let mut builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    // init pipeline
    builder
        .bind_pipeline_compute(compute_pipeline.clone());
        


    // record samples
    for i in 0..SAMPLES_PER_PIXEL{
        let limits = compute_shader::PushConstantData{
            sphere_amount: sphere_amount.into(),
            initial_seed: [
                rng.gen_range(u32::min_value()..u32::max_value()),
                rng.gen_range(u32::min_value()..u32::max_value()),
                rng.gen_range(u32::min_value()..u32::max_value()),
                rng.gen_range(u32::min_value()..u32::max_value()),
            ],
            camera: compute_shader::Camera{
                look_from: look_from.into(),
                look_at: look_at.into(),
                up: up.into(),
                vfov: 20.0 * f32::consts::PI / 180.0,
                aspect_ratio: ASPECT_RATIO,
                apeture: aperture,
                focus_distance: distance_to_focus,
            },

        };
        builder
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                compute_pipeline.layout().clone(),
                0,
                if i%2 == 0 {
                    descriptor_set_1.clone()
                }
                else{
                    descriptor_set_2.clone()
                }
            )
            .push_constants(compute_pipeline.layout().clone(), 0, limits)
            .dispatch([IMAGE_WIDTH / 8, IMAGE_HEIGHT / 8, 1])
            .unwrap();
    }
    
    // get image back
    builder
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
            output_image_1.clone(),
            output_buff.clone(),
        ))
        .unwrap();


    let command_buffer = builder.build().unwrap();

    // submit work
    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    // save image on disk
    let buffer_content = output_buff.read().unwrap();
    let image =
        ImageBuffer::<Rgba<u8>, _>::from_raw(IMAGE_WIDTH, IMAGE_HEIGHT, &buffer_content[..])
            .unwrap();
    image.save("out.png").unwrap();

    println!("Everything worked!");
}
