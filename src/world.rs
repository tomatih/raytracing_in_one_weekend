use ash::vk::{
    self, AabbPositionsKHR, AccelerationStructureBuildGeometryInfoKHR, AccelerationStructureBuildRangeInfoKHR, AccelerationStructureCreateFlagsKHR, AccelerationStructureGeometryAabbsDataKHR, AccelerationStructureGeometryDataKHR, AccelerationStructureGeometryKHR, AccelerationStructureTypeKHR, BufferUsageFlags, BuildAccelerationStructureFlagsKHR, BuildAccelerationStructureModeKHR, DeviceSize, GeometryFlagsKHR, GeometryTypeKHR
};
use cgmath::Vector4;
use vk_mem::{AllocationCreateInfo, Allocator};

use crate::{
    materials::Material,
    objects::Sphere,
    shaders,
    vulkan_helper::{Buffer, VulkanBase},
};

pub struct WorldCpu {
    geometry: Vec<Sphere>,
    materials: Vec<Material>,
}

#[allow(unused)] // this holds GPU resources not necesserliy used by the CPU
pub struct WorldGpu<'a> {
    pub geometry: Buffer<'a, shaders::ray_trace_shader::Sphere>,
    pub materials: Buffer<'a, Vector4<f32>>,
    pub set_layout: vk::DescriptorSetLayout,
    pub descriptor_set: vk::DescriptorSet,
}

impl<'a> WorldCpu {
    // add code here
    pub fn new() -> Self {
        Self {
            geometry: Vec::new(),
            materials: Vec::new(),
        }
    }

    pub fn add_material(&mut self, material: Material) {
        self.materials.push(material);
    }

    pub fn add_geometry(&mut self, geometry: Sphere) {
        self.geometry.push(geometry);
    }

    pub fn get_last_material_index(&self) -> usize {
        self.materials.len() - 1
    }

    pub fn get_material_type(&self, index: usize) -> u32 {
        self.materials[index].get_type()
    }

    pub unsafe fn upload(
        self,
        vulkan_base: &VulkanBase,
        descriptor_pool: vk::DescriptorPool,
        allocator: &'a Allocator,
    ) -> WorldGpu<'a> {
        // start the command buffer
        let command_buffer = vulkan_base.start_command_buffer();

        // RT
        let acceleration_loder = ash::khr::acceleration_structure::Device::new(
            &vulkan_base.instance,
            &vulkan_base.device,
        );


        let aabb_data = AccelerationStructureGeometryAabbsDataKHR::default()
            .data(data)
            .stride(size_of::<AabbPositionsKHR>() as DeviceSize);

        let geometry = AccelerationStructureGeometryDataKHR{
            aabbs: aabb_data
        };

        let geometries = [
            AccelerationStructureGeometryKHR::default()
                .geometry_type(GeometryTypeKHR::AABBS)
                .geometry(geometry),
        ];

        let infos = [
            AccelerationStructureBuildGeometryInfoKHR::default()
                .ty(AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                .flags(BuildAccelerationStructureFlagsKHR::PREFER_FAST_BUILD)
                .mode(BuildAccelerationStructureModeKHR::BUILD)
                .dst_acceleration_structure(dst_acceleration_structure)
                .geometries(&geometries)
                .scratch_data(scratch_data),
        ];

        let build_range = [
            AccelerationStructureBuildRangeInfoKHR::default()
        ];

        let build_range_infos = [
            &build_range
        ];

        acceleration_loder.cmd_build_acceleration_structures(
            command_buffer,
            &infos,
            &build_range_infos,
        );

        // main buffers
        let main_buffers_allocation_info = AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        let geometry = Buffer::<shaders::ray_trace_shader::Sphere>::new(
            allocator,
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::TRANSFER_DST,
            self.geometry.len(),
            main_buffers_allocation_info.clone(),
        );
        let materials = Buffer::<Vector4<f32>>::new(
            allocator,
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::TRANSFER_DST,
            self.materials.len(),
            main_buffers_allocation_info,
        );

        // staging buffers
        let staging_buffers_allocation_info = AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let mut geometry_staging = Buffer::<shaders::ray_trace_shader::Sphere>::new(
            allocator,
            BufferUsageFlags::TRANSFER_SRC,
            self.geometry.len(),
            staging_buffers_allocation_info.clone(),
        );
        let mut materials_staging = Buffer::<Vector4<f32>>::new(
            allocator,
            BufferUsageFlags::TRANSFER_SRC,
            self.materials.len(),
            staging_buffers_allocation_info,
        );

        geometry_staging.fill_buffer(self.geometry.into_iter().map(|s| s.into()).collect());
        materials_staging.fill_buffer(self.materials.into_iter().map(|m| m.into()).collect());

        // record upload command
        // let command_buffer = vulkan_base.start_command_buffer();

        // copy geometry
        let geometry_copy_regions = [vk::BufferCopy2::default()
            .src_offset(0)
            .dst_offset(0)
            .size(geometry.size)];
        let geometry_copy_info = vk::CopyBufferInfo2::default()
            .src_buffer(geometry_staging.handle)
            .dst_buffer(geometry.handle)
            .regions(&geometry_copy_regions);
        vulkan_base
            .device
            .cmd_copy_buffer2(command_buffer, &geometry_copy_info);

        // copy materials
        let material_copy_regions = [vk::BufferCopy2::default()
            .src_offset(0)
            .dst_offset(0)
            .size(materials.size)];
        let material_copy_info = vk::CopyBufferInfo2::default()
            .src_buffer(materials_staging.handle)
            .dst_buffer(materials.handle)
            .regions(&material_copy_regions);
        vulkan_base
            .device
            .cmd_copy_buffer2(command_buffer, &material_copy_info);

        let fence_create_info = vk::FenceCreateInfo::default();
        let fence = vulkan_base
            .device
            .create_fence(&fence_create_info, None)
            .unwrap();

        vulkan_base.submit_command_buffer(command_buffer, Some(fence));

        // wait so that staging buffers don't get dropped
        vulkan_base
            .device
            .wait_for_fences(&[fence], true, u64::MAX)
            .unwrap();
        vulkan_base.device.destroy_fence(fence, None);

        WorldGpu::new(geometry, materials, descriptor_pool, vulkan_base)
    }
}

impl<'a> WorldGpu<'a> {
    pub unsafe fn new(
        geo_buffer: Buffer<'a, shaders::ray_trace_shader::Sphere>,
        mat_buffer: Buffer<'a, Vector4<f32>>,
        descriptor_pool: vk::DescriptorPool,
        vulkan_base: &VulkanBase,
    ) -> Self {
        // create descriptor layout
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
        let descriptor_set_layout_bindings = [buffer_binding_0, buffer_binding_1];
        let descriptor_set_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_set_layout_bindings);
        let set_layout = vulkan_base
            .device
            .create_descriptor_set_layout(&descriptor_set_layout_info, None)
            .unwrap();

        // create descriptor set
        let set_layouts = [set_layout];
        let descriptor_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = vulkan_base
            .device
            .allocate_descriptor_sets(&descriptor_allocate_info)
            .unwrap()[0];

        // update descriptor set
        let geometry_buffer_descriptor_info = [vk::DescriptorBufferInfo::default()
            .buffer(geo_buffer.handle)
            .range(geo_buffer.size)];
        let world_descriptor_write_geometry = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&geometry_buffer_descriptor_info);
        let material_buffer_descriptor_info = [vk::DescriptorBufferInfo::default()
            .buffer(mat_buffer.handle)
            .range(mat_buffer.size)];
        let world_descriptor_write_material = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&material_buffer_descriptor_info);

        let descriptor_writes = [
            world_descriptor_write_geometry,
            world_descriptor_write_material,
        ];
        vulkan_base
            .device
            .update_descriptor_sets(&descriptor_writes, &[]);

        Self {
            geometry: geo_buffer,
            materials: mat_buffer,
            set_layout,
            descriptor_set,
        }
    }

    pub unsafe fn cleanup(self, vulkan_base: &VulkanBase) {
        vulkan_base
            .device
            .destroy_descriptor_set_layout(self.set_layout, None);
    }
}
