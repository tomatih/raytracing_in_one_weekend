use std::sync::Arc;

use vulkano::{buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::{allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferInfo}, device::{Device, Queue}, memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryUsage}, sync::{self, GpuFuture}, DeviceSize};

use crate::{materials::Material, objects::Sphere, shaders};

pub struct WorldCpu{
	geometry: Vec<Sphere>,
	materials: Vec<Material>
}

pub struct WorldGpu{
	pub geometry: Subbuffer<[shaders::ray_trace_shader::Sphere]>,
	pub materials: Subbuffer<[[f32;4]]>
}

impl WorldCpu {
	// add code here
	pub fn new() -> Self{
		Self { geometry: Vec::new(), materials: Vec::new() }
	}

	pub fn add_material(&mut self, material: Material){
		self.materials.push(material);
	}

	pub fn add_geometry(&mut self, geometry: Sphere){
		self.geometry.push(geometry);
	}

	pub fn upload(self, memory_allocator: &(impl MemoryAllocator + ?Sized), command_buffer_allocator: &StandardCommandBufferAllocator, queue: Arc<Queue>, device: Arc<Device>) -> WorldGpu{
		// make buffers
		let geometry = Buffer::new_slice::<shaders::ray_trace_shader::Sphere>(
			memory_allocator, 
			BufferCreateInfo{
				usage: BufferUsage::TRANSFER_DST | BufferUsage::STORAGE_BUFFER,
				..Default::default()
			}, 
			AllocationCreateInfo{
				usage: MemoryUsage::DeviceOnly,
				..Default::default()
			}, 
			self.geometry.len() as DeviceSize
		).expect("can't create deive geometry buffer");
		let materials = Buffer::new_slice::<[f32;4]>(
			memory_allocator, 
			BufferCreateInfo{
				usage: BufferUsage::TRANSFER_DST | BufferUsage::STORAGE_BUFFER,
				..Default::default()
			}, 
			AllocationCreateInfo{
				usage: MemoryUsage::DeviceOnly,
				..Default::default()
			}, 
			self.materials.len() as DeviceSize
		).expect("can't create device material buffer");

		// make staging buffers
		let geometry_staging: Subbuffer<[shaders::ray_trace_shader::Sphere]> = Buffer::from_iter(
			memory_allocator, 
			BufferCreateInfo{
				usage: BufferUsage::TRANSFER_SRC,
				..Default::default()
			}, 
			AllocationCreateInfo{
				usage: MemoryUsage::Upload,
				..Default::default()
			},
			self.geometry.into_iter().map(|s| s.into())
		).unwrap();
		let material_staging: Subbuffer<[[f32;4]]> = Buffer::from_iter(
			memory_allocator, 
			BufferCreateInfo{
				usage: BufferUsage::TRANSFER_SRC,
				..Default::default()
			}, 
			AllocationCreateInfo{
				usage: MemoryUsage::Upload,
				..Default::default() 
			},
			self.materials.into_iter().map(|m| m.into())
		).unwrap();


		// upload data
	    let mut builder = AutoCommandBufferBuilder::primary(
	        command_buffer_allocator,
	        queue.queue_family_index(),
	        CommandBufferUsage::OneTimeSubmit,
	    )
	    .unwrap();

	    builder
	    	.copy_buffer(CopyBufferInfo::buffers(material_staging, materials.clone()))
	    	.unwrap()
	    	.copy_buffer(CopyBufferInfo::buffers(geometry_staging, geometry.clone()))
	    	.unwrap();

	     let command_buffer = builder.build().unwrap();

	    // submit work
	    let future = sync::now(device.clone())
	        .then_execute(queue.clone(), command_buffer)
	        .unwrap()
	        .then_signal_fence_and_flush()
	        .unwrap();

	    future.wait(None).unwrap();


		WorldGpu { geometry, materials }
	}

}