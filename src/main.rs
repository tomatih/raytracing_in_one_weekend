// project modules
mod vulkan_helper;
mod compute_shader;

// external imports
use image::{Rgba, ImageBuffer};

// Vulkan inports
use vulkano::{
    memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage},
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{
        allocator::{
            StandardCommandBufferAllocator,
            StandardCommandBufferAllocatorCreateInfo
        },
        AutoCommandBufferBuilder,
        CommandBufferUsage,
        CopyImageToBufferInfo
    },
    sync::{self, GpuFuture},
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator,
        PersistentDescriptorSet,
        WriteDescriptorSet
    },
    image::{ImageDimensions, StorageImage, view::ImageView},
    format::Format
};

// own imports
use crate::vulkan_helper::{get_vulkan_instance, get_physical_device, get_logical_device};

fn main (){
    // image data
    let image_size = 1024;
    // init vulkan
    let instance = get_vulkan_instance();
    let physical_device = get_physical_device(instance);
    let (device, queue) = get_logical_device(physical_device);

    // load shader
    let shader = compute_shader::load(device.clone()).expect("Failed to load shader module");

    // create a memory allocator
    let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

    // data buffer
    let buf = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo{
            usage: BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo{
            usage: MemoryUsage::Download,
            ..Default::default()
        },
        (0..image_size * image_size * 4).map(|_| 0u8)
    ).expect("failed to create buffer");

    // image
    let image = StorageImage::new(
        &memory_allocator,
        ImageDimensions::Dim2d {
            width: image_size,
            height: image_size,
            array_layers: 1
        },
        Format::R8G8B8A8_UNORM,
        Some(queue.queue_family_index())
    ).unwrap();

    let view = ImageView::new_default(image.clone()).unwrap();

    // create pipeline
    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        shader.entry_point("main").unwrap(),
        &(),
        None,
        |_| {},
    ).expect("failed to create compute pipeline");

    // descriptor sets
    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
    let pipeline_layout = compute_pipeline.layout();
    let descriptor_set_layouts = pipeline_layout.set_layouts();

    let descriptor_set_layout = descriptor_set_layouts.get(0).unwrap();
    let descriptor_set = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::image_view(0, view.clone())]
    ).unwrap();

    // command buffer allocator
    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(), 
        StandardCommandBufferAllocatorCreateInfo::default()
    );

    // command buffer builder
    let mut builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit
    ).unwrap();

    builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            compute_pipeline.layout().clone(),
            0,
            descriptor_set
        )
        .dispatch([image_size / 8, image_size / 8, 1])
        .unwrap()
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(image.clone(), buf.clone()))
        .unwrap();

    let command_buffer = builder.build().unwrap();

    // submit work
    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>,_>::from_raw(image_size, image_size, &buffer_content[..]).unwrap();
    image.save("out.png").unwrap();

    println!("Everything worked!");
}