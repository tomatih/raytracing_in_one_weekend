// project modules
mod camera;
mod common;
mod compute_shader;
mod vulkan_helper;

use cgmath::Deg;
// external imports
use image::{ImageBuffer, Rgba};

// Vulkan inports
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage},
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
use crate::camera::Camera;
use crate::common::{Point3, Vec3};
use crate::vulkan_helper::{get_logical_device, get_physical_device, get_vulkan_instance};

fn main() {
    println!("Starting the renderer");
    // image data
    const ASPECT_RATIO: f32 = 16.0 / 9.0;
    const IMAGE_WIDTH: u32 = 400;
    const IMAGE_HEIGHT: u32 = (IMAGE_WIDTH as f32 / ASPECT_RATIO) as u32;

    // camera
    let look_from = Point3::new(13.0, 2.0, 3.0);
    let look_at = Point3::new(0.0, 0.0, 0.0);
    let up = Vec3::unit_y();
    let distance_to_focus = 10.0;
    let aperture = 0.1;
    let camera = Camera::new(
        look_from,
        look_at,
        up,
        Deg(20.0),
        ASPECT_RATIO,
        aperture,
        distance_to_focus,
    );

    // init vulkan
    let instance = get_vulkan_instance();
    let physical_device = get_physical_device(instance);
    println!("Chosen {}", physical_device.properties().device_name);
    let (device, queue) = get_logical_device(physical_device);

    // load shader
    let shader = compute_shader::load(device.clone()).expect("Failed to load shader module");

    // create a memory allocator
    let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

    // camera push constant
    let push_constant = camera.to_push_constant();

    // image
    let output_image = StorageImage::new(
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

    // data buffer
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

    let view = ImageView::new_default(output_image.clone()).unwrap();

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
    let descriptor_set = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::image_view(0, view.clone())],
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

    builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            compute_pipeline.layout().clone(),
            0,
            descriptor_set,
        )
        .push_constants(pipeline_layout.clone(), 0, push_constant)
        .dispatch([IMAGE_WIDTH / 8, IMAGE_HEIGHT / 8, 1])
        .unwrap()
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
            output_image.clone(),
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
