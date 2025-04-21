use ash::vk::{self, AabbPositionsKHR, AccelerationStructureBuildGeometryInfoKHR, AccelerationStructureBuildRangeInfoKHR, AccelerationStructureBuildSizesInfoKHR, AccelerationStructureBuildTypeKHR, AccelerationStructureCreateFlagsKHR, AccelerationStructureCreateInfoKHR, AccelerationStructureGeometryAabbsDataKHR, AccelerationStructureGeometryDataKHR, AccelerationStructureGeometryKHR, AccelerationStructureKHR, AccelerationStructureTypeKHR, AccessFlags2, BufferDeviceAddressInfo, BufferMemoryBarrier2, BufferUsageFlags, BuildAccelerationStructureFlagsKHR, BuildAccelerationStructureModeKHR, DependencyInfo, DeviceOrHostAddressConstKHR, DeviceOrHostAddressKHR, DeviceSize, GeometryFlagsKHR, GeometryTypeKHR, PipelineStageFlags2};
use cgmath::Vector4;
use vk_mem::{AllocationCreateInfo, Allocator};

use crate::{
    materials::Material,
    objects::Sphere,
    shaders,
    vulkan_helper::{get_buffer_address, get_buffer_const_address, Buffer, VulkanBase},
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

        // buffer memory types
        let staging_buffers_allocation_info = AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let main_buffers_allocation_info = AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        // RT extension
        let acceleration_loder = ash::khr::acceleration_structure::Device::new(
            &vulkan_base.instance,
            &vulkan_base.device,
        );

        // AS section
        // will have a single TLAS and a single BLAS as there is only static geometry
        // TODO: BLAS will have a node for each sphere wan an AABB around it
        // start with 1 geomety with all spheres in it

        // Geometry setup
        let mut aabb_staging = Buffer::<AabbPositionsKHR>::new(
            allocator,
            BufferUsageFlags::TRANSFER_SRC,
            self.geometry.len(),
            staging_buffers_allocation_info.clone()
        );
        let aabb_data = self.geometry.iter().map(|sphere| {
            AabbPositionsKHR::default()
                .max_x(sphere.center.x + sphere.radius)
                .max_y(sphere.center.y + sphere.radius)
                .max_z(sphere.center.z + sphere.radius)
                .min_x(sphere.center.x - sphere.radius)
                .min_y(sphere.center.y - sphere.radius)
                .min_z(sphere.center.z - sphere.radius)
        }).collect();
        aabb_staging.fill_buffer(aabb_data);

        let aabb_buffer = Buffer::<AabbPositionsKHR>::new(
            allocator,
            BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::SHADER_DEVICE_ADDRESS_KHR | BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            self.geometry.len(),
            main_buffers_allocation_info.clone()
        );

        let aabb_copy_regions = [vk::BufferCopy2::default()
            .src_offset(0)
            .dst_offset(0)
            .size(aabb_staging.size)];
        let geometry_copy_info = vk::CopyBufferInfo2::default()
            .src_buffer(aabb_staging.handle)
            .dst_buffer(aabb_buffer.handle)
            .regions(&aabb_copy_regions);
        vulkan_base
            .device
            .cmd_copy_buffer2(command_buffer, &geometry_copy_info);

        let blas_memory_barriers = [
            BufferMemoryBarrier2::default()
                .buffer(aabb_buffer.handle)
                .size(aabb_buffer.size)
                .src_access_mask(AccessFlags2::TRANSFER_WRITE)
                .src_stage_mask(PipelineStageFlags2::TRANSFER)
                .dst_access_mask(AccessFlags2::SHADER_READ)
                .dst_stage_mask(PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR)
        ];
        let blas_dependency_info = DependencyInfo::default()
            .buffer_memory_barriers(&blas_memory_barriers);
        vulkan_base.device.cmd_pipeline_barrier2(command_buffer,&blas_dependency_info);

        let aabb_data = AccelerationStructureGeometryAabbsDataKHR::default()
            .data(get_buffer_const_address(&vulkan_base, &aabb_buffer))
            .stride(size_of::<AabbPositionsKHR>() as DeviceSize);

        let geometry = AccelerationStructureGeometryDataKHR{
            aabbs: aabb_data
        };

        let geometries = [
            AccelerationStructureGeometryKHR::default()
                .geometry_type(GeometryTypeKHR::AABBS)
                .geometry(geometry),
        ];

        // BLAS info
        let mut blas_infos = [
            AccelerationStructureBuildGeometryInfoKHR::default()
                .ty(AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                .flags(BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
                .mode(BuildAccelerationStructureModeKHR::BUILD)
                .geometries(&geometries)
        ];

        let blas_build_range = [
            AccelerationStructureBuildRangeInfoKHR::default()
                .primitive_offset(0)
                .primitive_count(self.geometry.len() as u32)
        ];

        // BLAS range
        let blas_build_range_infos = [
            blas_build_range.as_slice()
        ];

        // get BLAS size
        let mut as_size = AccelerationStructureBuildSizesInfoKHR::default();
        acceleration_loder.get_acceleration_structure_build_sizes(
            AccelerationStructureBuildTypeKHR::DEVICE,
            &blas_infos[0],
            &[self.geometry.len() as u32],
            &mut as_size
        );

        // make BLAS buffers
        let blas_scratch = Buffer::<u8>::new(
            allocator,
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::SHADER_DEVICE_ADDRESS_KHR,
            as_size.build_scratch_size as usize,
            main_buffers_allocation_info.clone()
        );
        blas_infos[0].scratch_data = get_buffer_address(&vulkan_base, &blas_scratch);

        let blas_buffer = Buffer::<u8>::new(
            allocator,
            BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | BufferUsageFlags::SHADER_DEVICE_ADDRESS_KHR,
            as_size.acceleration_structure_size as usize,
            main_buffers_allocation_info.clone()
        );
        
        let blas_create_info = AccelerationStructureCreateInfoKHR::default()
            .buffer(blas_buffer.handle)
            .size(as_size.acceleration_structure_size)
            .ty(AccelerationStructureTypeKHR::BOTTOM_LEVEL);
        let blas = acceleration_loder.create_acceleration_structure(&blas_create_info, None).unwrap();
        blas_infos[0].dst_acceleration_structure = blas;

        acceleration_loder.cmd_build_acceleration_structures(
            command_buffer,
            &blas_infos,
            &blas_build_range_infos,
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
