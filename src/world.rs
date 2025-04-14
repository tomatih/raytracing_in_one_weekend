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

pub struct WorldGpu<'a> {
    pub geometry: Buffer<'a, shaders::ray_trace_shader::Sphere>,
    pub materials: Buffer<'a, Vector4<f32>>,
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

        let fence_create_info =
            vk::FenceCreateInfo::default();
        let fence = vulkan_base.device.create_fence(&fence_create_info, None).unwrap();

        vulkan_base.submit_command_buffer(command_buffer, Some(fence));

        // wait so that staging buffers don't get dropped
        vulkan_base.device.wait_for_fences(&[fence], true, u64::MAX).unwrap();
        vulkan_base.device.destroy_fence(fence, None);

        WorldGpu {
            geometry,
            materials,
        }
    }
}
