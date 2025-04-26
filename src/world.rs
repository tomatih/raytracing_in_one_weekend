use ash::vk::{self, AabbPositionsKHR, AccelerationStructureBuildGeometryInfoKHR, AccelerationStructureBuildRangeInfoKHR, AccelerationStructureBuildSizesInfoKHR, AccelerationStructureBuildTypeKHR, AccelerationStructureCreateInfoKHR, AccelerationStructureDeviceAddressInfoKHR, AccelerationStructureGeometryAabbsDataKHR, AccelerationStructureGeometryDataKHR, AccelerationStructureGeometryInstancesDataKHR, AccelerationStructureGeometryKHR, AccelerationStructureInstanceKHR, AccelerationStructureKHR, AccelerationStructureTypeKHR, AccessFlags2, BufferCopy2, BufferMemoryBarrier2, BufferUsageFlags, BuildAccelerationStructureFlagsKHR, BuildAccelerationStructureModeKHR, CopyBufferInfo2, DependencyInfo, DescriptorType, DeviceSize, GeometryTypeKHR, Packed24_8, PipelineStageFlags2, WriteDescriptorSetAccelerationStructureKHR};
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

#[allow(unused)] // this holds GPU resources not necesserliy used by the CPU
pub struct WorldGpu<'a> {
    pub geometry: Buffer<'a, shaders::ray_trace_shader::Sphere>,
    pub materials: Buffer<'a, Vector4<f32>>,
    pub set_layout: vk::DescriptorSetLayout,
    pub descriptor_set: vk::DescriptorSet,

    pub acceleration_loader: ash::khr::acceleration_structure::Device,
    pub blas: AccelerationStructureKHR,
    pub blas_buffer: Buffer<'a, u8>,
    pub tlas: AccelerationStructureKHR,
    pub tlas_buffer: Buffer<'a, u8>,
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

        // Geometry setup
        let mut aabb_staging = Buffer::<AabbPositionsKHR>::new(
            allocator,
            BufferUsageFlags::TRANSFER_SRC,
            1,
            staging_buffers_allocation_info.clone(),
        );
        let aabb_data = vec![
            AabbPositionsKHR::default()
                .max_x(1.0)
                .max_y(1.0)
                .max_z(1.0)
                .min_x(-1.0)
                .min_y(-1.0)
                .min_z(-1.0)
        ];
        aabb_staging.fill_buffer(aabb_data);

        let aabb_buffer = Buffer::<AabbPositionsKHR>::new(
            allocator,
            BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::SHADER_DEVICE_ADDRESS_KHR
                | BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            1,
            main_buffers_allocation_info.clone(),
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

        let blas_memory_barriers = [BufferMemoryBarrier2::default()
            .buffer(aabb_buffer.handle)
            .size(aabb_buffer.size)
            .src_access_mask(AccessFlags2::TRANSFER_WRITE)
            .src_stage_mask(PipelineStageFlags2::TRANSFER)
            .dst_access_mask(AccessFlags2::SHADER_READ)
            .dst_stage_mask(PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR)];
        let blas_dependency_info =
            DependencyInfo::default().buffer_memory_barriers(&blas_memory_barriers);
        vulkan_base
            .device
            .cmd_pipeline_barrier2(command_buffer, &blas_dependency_info);

        let aabb_data = AccelerationStructureGeometryAabbsDataKHR::default()
            .data(get_buffer_const_address(&vulkan_base, &aabb_buffer))
            .stride(size_of::<AabbPositionsKHR>() as DeviceSize);

        let geometry = AccelerationStructureGeometryDataKHR { aabbs: aabb_data };

        let geometries = [AccelerationStructureGeometryKHR::default()
            .geometry_type(GeometryTypeKHR::AABBS)
            .geometry(geometry)];

        // BLAS info
        let mut blas_infos = [AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&geometries)];

        let blas_build_range = [AccelerationStructureBuildRangeInfoKHR::default()
            .primitive_offset(0)
            .primitive_count(1)
        ];

        // BLAS range
        let blas_build_range_infos = [blas_build_range.as_slice()];

        // get BLAS size
        let mut as_size = AccelerationStructureBuildSizesInfoKHR::default();
        acceleration_loder.get_acceleration_structure_build_sizes(
            AccelerationStructureBuildTypeKHR::DEVICE,
            &blas_infos[0],
            &[1],
            &mut as_size,
        );

        // make BLAS buffers
        let blas_scratch = Buffer::<u8>::new(
            allocator,
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::SHADER_DEVICE_ADDRESS_KHR,
            as_size.build_scratch_size as usize,
            main_buffers_allocation_info.clone(),
        );
        blas_infos[0].scratch_data = get_buffer_address(&vulkan_base, &blas_scratch);

        let blas_buffer = Buffer::<u8>::new(
            allocator,
            BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | BufferUsageFlags::SHADER_DEVICE_ADDRESS_KHR,
            as_size.acceleration_structure_size as usize,
            main_buffers_allocation_info.clone(),
        );

        let blas_create_info = AccelerationStructureCreateInfoKHR::default()
            .buffer(blas_buffer.handle)
            .size(as_size.acceleration_structure_size)
            .ty(AccelerationStructureTypeKHR::BOTTOM_LEVEL);
        let blas = acceleration_loder
            .create_acceleration_structure(&blas_create_info, None)
            .unwrap();
        blas_infos[0].dst_acceleration_structure = blas;

        acceleration_loder.cmd_build_acceleration_structures(
            command_buffer,
            &blas_infos,
            &blas_build_range_infos,
        );

        // bundle BLASes
        let blas_device_address_info =
            AccelerationStructureDeviceAddressInfoKHR::default().acceleration_structure(blas);
        let blas_handle =
            acceleration_loder.get_acceleration_structure_device_address(&blas_device_address_info);

        let blas_vec: Vec<AccelerationStructureInstanceKHR> = self.geometry.iter().map(
            |sphere|{
                AccelerationStructureInstanceKHR{
                    transform: vk::TransformMatrixKHR {
                        matrix: [
                            sphere.radius, 0.0, 0.0, sphere.center.x,
                            0.0, sphere.radius, 0.0, sphere.center.y,
                            0.0, 0.0, sphere.radius, sphere.center.z,
                        ],
                    },
                    instance_custom_index_and_mask: Packed24_8::new(0, 0xFF),
                    instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(0,0),
                    acceleration_structure_reference: vk::AccelerationStructureReferenceKHR{device_handle: blas_handle},
                }
            }
        ).collect();

        let mut blas_list_staging = Buffer::<AccelerationStructureInstanceKHR>::new(
            &allocator,
            BufferUsageFlags::TRANSFER_SRC,
            blas_vec.len(),
            staging_buffers_allocation_info.clone(),
        );
        blas_list_staging.fill_buffer(blas_vec);

        let blas_list = Buffer::<AccelerationStructureInstanceKHR>::new(
            &allocator,
            BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            blas_list_staging.count,
            main_buffers_allocation_info.clone(),
        );
        let blas_list_addres = get_buffer_const_address(&vulkan_base, &blas_list);
        let blas_list_copy_regions = [BufferCopy2::default().size(blas_list.size)];
        let blas_list_copy_info = CopyBufferInfo2::default()
            .src_buffer(blas_list_staging.handle)
            .dst_buffer(blas_list.handle)
            .regions(&blas_list_copy_regions);
        vulkan_base
            .device
            .cmd_copy_buffer2(command_buffer, &blas_list_copy_info);

        let tlas_memory_barriers = [BufferMemoryBarrier2::default()
            .buffer(blas_list.handle)
            .size(blas_list.size)
            .src_access_mask(AccessFlags2::TRANSFER_WRITE)
            .src_stage_mask(PipelineStageFlags2::TRANSFER)
            .dst_access_mask(AccessFlags2::SHADER_READ)
            .dst_stage_mask(PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR)];
        let tlas_dependency_info =
            DependencyInfo::default().buffer_memory_barriers(&tlas_memory_barriers);
        vulkan_base
            .device
            .cmd_pipeline_barrier2(command_buffer, &tlas_dependency_info);

        // setup build geometry info
        let tlas_geometries = [AccelerationStructureGeometryKHR::default()
            .geometry_type(GeometryTypeKHR::INSTANCES)
            .geometry(AccelerationStructureGeometryDataKHR {
                instances: AccelerationStructureGeometryInstancesDataKHR::default()
                    .data(blas_list_addres),
            })];

        let mut tlas_geometry_info = AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&tlas_geometries);

        // setup TLAS buffers`
        let mut tlas_sizes = AccelerationStructureBuildSizesInfoKHR::default();
        acceleration_loder.get_acceleration_structure_build_sizes(
            AccelerationStructureBuildTypeKHR::DEVICE,
            &tlas_geometry_info,
            &[self.geometry.len() as u32],
            &mut tlas_sizes,
        );

        let tlas_scratch = Buffer::<u8>::new(
            &allocator,
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            tlas_sizes.build_scratch_size as usize,
            main_buffers_allocation_info.clone(),
        );
        let tlas_scratch_address = get_buffer_address(&vulkan_base, &tlas_scratch);

        let tlas_buffer = Buffer::<u8>::new(
            allocator,
            BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | BufferUsageFlags::SHADER_DEVICE_ADDRESS_KHR,
            tlas_sizes.acceleration_structure_size as usize,
            main_buffers_allocation_info.clone(),
        );

        let tlas_creation_info = AccelerationStructureCreateInfoKHR::default()
            .ty(AccelerationStructureTypeKHR::TOP_LEVEL)
            .buffer(tlas_buffer.handle)
            .size(tlas_sizes.acceleration_structure_size as DeviceSize);

        let tlas = acceleration_loder
            .create_acceleration_structure(&tlas_creation_info, None)
            .unwrap();

        tlas_geometry_info.scratch_data = tlas_scratch_address;
        tlas_geometry_info.dst_acceleration_structure = tlas;

        acceleration_loder.cmd_build_acceleration_structures(
            command_buffer,
            &[tlas_geometry_info],
            &[[AccelerationStructureBuildRangeInfoKHR::default().primitive_count(self.geometry.len() as u32)].as_slice()],
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

        WorldGpu::new(geometry, materials, descriptor_pool, vulkan_base, acceleration_loder, blas, blas_buffer, tlas, tlas_buffer)
    }
}

impl<'a> WorldGpu<'a> {
    pub unsafe fn new(
        geo_buffer: Buffer<'a, shaders::ray_trace_shader::Sphere>,
        mat_buffer: Buffer<'a, Vector4<f32>>,
        descriptor_pool: vk::DescriptorPool,
        vulkan_base: &VulkanBase,
        acceleration_loader: ash::khr::acceleration_structure::Device,
        blas: AccelerationStructureKHR,
        blas_buffer: Buffer<'a, u8>,
        tlas: AccelerationStructureKHR,
        tlas_buffer: Buffer<'a, u8>,
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

        let as_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(2)
            .descriptor_count(1)
            .descriptor_type(DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .stage_flags(vk::ShaderStageFlags::COMPUTE);

        let descriptor_set_layout_bindings = [buffer_binding_0, buffer_binding_1, as_binding];
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

        let as_list = [tlas];
        let mut as_descriptor_info = WriteDescriptorSetAccelerationStructureKHR::default()
            .acceleration_structures(&as_list);
        let as_descriptor_write = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(2)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .push_next(&mut as_descriptor_info);

        let descriptor_writes = [
            world_descriptor_write_geometry,
            world_descriptor_write_material,
            as_descriptor_write,
        ];
        vulkan_base
            .device
            .update_descriptor_sets(&descriptor_writes, &[]);

        Self {
            geometry: geo_buffer,
            materials: mat_buffer,
            set_layout,
            descriptor_set,
            acceleration_loader,
            blas,
            blas_buffer,
            tlas,
            tlas_buffer
        }
    }

    pub unsafe fn cleanup(self, vulkan_base: &VulkanBase) {
        vulkan_base
            .device
            .destroy_descriptor_set_layout(self.set_layout, None);

        self.acceleration_loader.destroy_acceleration_structure(self.tlas, None);
        self.acceleration_loader.destroy_acceleration_structure(self.blas, None);
    }
}
