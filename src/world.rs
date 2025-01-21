use std::sync::Arc;

use cgmath::Vector4;
use vulkano::{
	buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer}, 
	command_buffer::{
		allocator::StandardCommandBufferAllocator, 
		AutoCommandBufferBuilder,
		CommandBufferUsage, 
		CopyBufferInfo
	}, 
	device::{Device, Queue}, 
	memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter}, 
	sync::{self, GpuFuture}, 
	DeviceSize
};

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

	pub fn get_last_material_index(&self) -> usize{
		self.materials.len() - 1 
	}

	pub fn get_material_type(&self, index: usize) -> u32{
		self.materials[index].get_type()
	}

	pub fn upload(self, memory_allocator: Arc<dyn MemoryAllocator>, command_buffer_allocator: Arc<StandardCommandBufferAllocator>, queue: Arc<Queue>, device: Arc<Device>) -> WorldGpu{
		// make buffers
		let geometry = Buffer::new_slice::<shaders::ray_trace_shader::Sphere>(
			memory_allocator.clone(), 
			BufferCreateInfo{
				usage: BufferUsage::TRANSFER_DST | BufferUsage::STORAGE_BUFFER,
				..Default::default()
			}, 
			AllocationCreateInfo{
				memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
				..Default::default()
			}, 
			self.geometry.len() as DeviceSize
		).expect("can't create deive geometry buffer");
		let materials = Buffer::new_slice::<[f32;4]>(
			memory_allocator.clone(), 
			BufferCreateInfo{
				usage: BufferUsage::TRANSFER_DST | BufferUsage::STORAGE_BUFFER,
				..Default::default()
			}, 
			AllocationCreateInfo{
				memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
				..Default::default()
			}, 
			self.materials.len() as DeviceSize
		).expect("can't create device material buffer");

		// make staging buffers
		let geometry_staging: Subbuffer<[shaders::ray_trace_shader::Sphere]> = Buffer::from_iter(
			memory_allocator.clone(), 
			BufferCreateInfo{
				usage: BufferUsage::TRANSFER_SRC,
				..Default::default()
			}, 
			AllocationCreateInfo{
				memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
				..Default::default()
			}, 
			self.geometry.into_iter().map(|s| s.into())
		).unwrap();
		let material_staging: Subbuffer<[Vector4<f32>]> = Buffer::from_iter(
			memory_allocator.clone(), 
			BufferCreateInfo{
				usage: BufferUsage::TRANSFER_SRC,
				..Default::default()
			}, 
			AllocationCreateInfo{
				memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
				..Default::default()
			}, 
			self.materials.into_iter().map(|m| m.into())
		).unwrap();


		// upload data
	    let mut builder = AutoCommandBufferBuilder::primary(
	        &command_buffer_allocator,
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