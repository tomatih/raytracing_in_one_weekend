use ash::vk;
use cgmath::Vector4;
use itertools::Itertools;
use vk_mem::{Alloc, AllocationCreateInfo, MemoryUsage};

use crate::{
    shaders::{finalize_shader, ray_trace_shader},
    vulkan_helper::{load_shader, Buffer, VulkanBase},
    world::{WorldCpu, WorldGpu},
};

pub struct RenderResources<'a> {
    pub render_shader: vk::ShaderModule,
    pub final_shader: vk::ShaderModule,

    pub command_pool: vk::CommandPool,
    pub descriptor_pool: vk::DescriptorPool,

    pub output_image_extent: vk::Extent3D,
    pub output_image: vk::Image,
    output_image_memory: vk_mem::Allocation,
    pub output_image_subresource: vk::ImageSubresourceRange,
    pub output_image_view: vk::ImageView,

    pub output_buffer: Buffer<'a, u8>,
    pub working_buffer_1: Buffer<'a, Vector4<f32>>,
    pub working_buffer_2: Buffer<'a, Vector4<f32>>,

    pub world_gpu: WorldGpu<'a>,

    pub main_descriptor_set_layout: vk::DescriptorSetLayout,
    pub final_descriptor_set_layout: vk::DescriptorSetLayout,

    pub world_descriptor_set: vk::DescriptorSet,
    pub work_descriptor_set_1: vk::DescriptorSet,
    pub work_descriptor_set_2: vk::DescriptorSet,
    pub final_descriptor_set: vk::DescriptorSet,

    pipeline_cache: vk::PipelineCache,
    pub render_pipeline_layout: vk::PipelineLayout,
    pub render_pipeline: vk::Pipeline,
    pub finalize_pipeline_layout: vk::PipelineLayout,
    pub finalize_pipeline: vk::Pipeline,

    pub barriers_1_to_2: [vk::BufferMemoryBarrier2<'a>; 2],
    pub barriers_2_to_1: [vk::BufferMemoryBarrier2<'a>; 2],
    pub render_fence: vk::Fence,
}

impl<'a> RenderResources<'a> {
    unsafe fn _create_shaders(vulkan_base: &VulkanBase) -> (vk::ShaderModule, vk::ShaderModule) {
        let main_shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/ray_trace.comp.spv"));
        let main_shader_module = load_shader(vulkan_base, main_shader_bytes);
        let final_shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/finalize.comp.spv"));
        let final_shader_module = load_shader(vulkan_base, final_shader_bytes);

        (main_shader_module, final_shader_module)
    }

    unsafe fn _create_memory_pools(
        vulkan_base: &VulkanBase,
    ) -> (vk::CommandPool, vk::DescriptorPool) {
        let command_pool_create_info = vk::CommandPoolCreateInfo::default().queue_family_index(0);
        let command_pool = vulkan_base
            .device
            .create_command_pool(&command_pool_create_info, None)
            .unwrap();

        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(7),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1),
        ];
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(4)
            .pool_sizes(&descriptor_pool_sizes);
        let descriptor_pool = vulkan_base
            .device
            .create_descriptor_pool(&descriptor_pool_create_info, None)
            .unwrap();

        (command_pool, descriptor_pool)
    }

    unsafe fn _create_image(
        vulkan_base: &'a VulkanBase,
        image_width: u32,
        image_height: u32,
    ) -> (
        vk::Extent3D,
        vk::Image,
        vk_mem::Allocation,
        vk::ImageSubresourceRange,
        vk::ImageView,
    ) {
        // output image
        let image_extent = vk::Extent3D {
            width: image_width,
            height: image_height,
            depth: 1,
        };
        let image_create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_UNORM,
            extent: image_extent,
            usage: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC,
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            initial_layout: vk::ImageLayout::UNDEFINED,
            tiling: vk::ImageTiling::OPTIMAL,
            ..Default::default()
        };
        let image_allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        let (image, image_allocation) = vulkan_base
            .allocator
            .create_image(&image_create_info, &image_allocation_info)
            .unwrap();
        let image_subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };
        let image_view_create_info = vk::ImageViewCreateInfo {
            image,
            subresource_range: image_subresource_range,
            format: vk::Format::R8G8B8A8_UNORM,
            view_type: vk::ImageViewType::TYPE_2D,
            ..Default::default()
        };
        let image_view = vulkan_base
            .device
            .create_image_view(&image_view_create_info, None)
            .unwrap();

        (
            image_extent,
            image,
            image_allocation,
            image_subresource_range,
            image_view,
        )
    }

    unsafe fn _create_buffers(
        allocator: &'a vk_mem::Allocator,
        image_height: u32,
        image_width: u32,
    ) -> (
        Buffer<'a, u8>,
        Buffer<'a, Vector4<f32>>,
        Buffer<'a, Vector4<f32>>,
    ) {
        // output buffer
        let output_buffer_allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
            ..Default::default()
        };
        let output_buffer = Buffer::<u8>::new(
            &allocator,
            vk::BufferUsageFlags::TRANSFER_DST,
            (image_height * image_width * 4) as usize,
            output_buffer_allocation_info,
        );

        // working buffers
        let output_buffer_allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::AutoPreferDevice,
            flags: vk_mem::AllocationCreateFlags::DEDICATED_MEMORY,
            ..Default::default()
        };
        let working_buffer_1 = Buffer::<Vector4<f32>>::new(
            &allocator,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            (image_height * image_width) as usize,
            output_buffer_allocation_info.clone(),
        );
        let working_buffer_2 = Buffer::<Vector4<f32>>::new(
            &allocator,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            (image_height * image_width) as usize,
            output_buffer_allocation_info,
        );

        (output_buffer, working_buffer_1, working_buffer_2)
    }

    unsafe fn _create_descriptor_layouts(
        vulkan_base: &'a VulkanBase,
    ) -> (vk::DescriptorSetLayout, vk::DescriptorSetLayout) {
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
        let main_descriptor_set_layout_bindings = [buffer_binding_0, buffer_binding_1];
        let main_descriptor_set_layout = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&main_descriptor_set_layout_bindings);
        let main_descriptor_set_layout = vulkan_base
            .device
            .create_descriptor_set_layout(&main_descriptor_set_layout, None)
            .unwrap();

        let image_binding_1 = vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .stage_flags(vk::ShaderStageFlags::COMPUTE);
        let final_descriptor_set_layout_bindings = [buffer_binding_0, image_binding_1];
        let final_descriptor_set_layout = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&final_descriptor_set_layout_bindings);
        let final_descriptor_set_layout = vulkan_base
            .device
            .create_descriptor_set_layout(&final_descriptor_set_layout, None)
            .unwrap();

        (main_descriptor_set_layout, final_descriptor_set_layout)
    }

    unsafe fn _create_pipelines(
        vulkan_base: &'a VulkanBase,
        main_descriptor_set_layout: vk::DescriptorSetLayout,
        final_descriptor_set_layout: vk::DescriptorSetLayout,
        main_shader_module: vk::ShaderModule,
        final_shader_module: vk::ShaderModule,
    ) -> (
        vk::PipelineCache,
        vk::PipelineLayout,
        vk::Pipeline,
        vk::PipelineLayout,
        vk::Pipeline,
    ) {
        // pipeline layouts
        let main_descriptor_set_layouts = [main_descriptor_set_layout, main_descriptor_set_layout];
        let main_push_constant_ranges = [vk::PushConstantRange::default()
            .size(std::mem::size_of::<ray_trace_shader::PushConstantData>() as u32)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)];
        let main_pipeline_layout_cerate_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&main_descriptor_set_layouts)
            .push_constant_ranges(&main_push_constant_ranges);
        let main_pipeline_layout = vulkan_base
            .device
            .create_pipeline_layout(&main_pipeline_layout_cerate_info, None)
            .unwrap();

        let final_descriptor_set_layouts = [final_descriptor_set_layout];
        let final_push_constant_ranges = [vk::PushConstantRange::default()
            .size(std::mem::size_of::<finalize_shader::PushConstantData>() as u32)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)];
        let final_pipeline_layout_cerate_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&final_descriptor_set_layouts)
            .push_constant_ranges(&final_push_constant_ranges);
        let final_pipeline_layout = vulkan_base
            .device
            .create_pipeline_layout(&final_pipeline_layout_cerate_info, None)
            .unwrap();

        // create pipelines
        let pipeline_cache_create_into = vk::PipelineCacheCreateInfo::default()
            .flags(vk::PipelineCacheCreateFlags::EXTERNALLY_SYNCHRONIZED)
            .initial_data(&[]);
        let pipeline_cache = vulkan_base
            .device
            .create_pipeline_cache(&pipeline_cache_create_into, None)
            .unwrap();

        let main_shader_pipeline_stage = vk::PipelineShaderStageCreateInfo::default()
            .module(main_shader_module)
            .name(c"main")
            .stage(vk::ShaderStageFlags::COMPUTE);
        let main_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
            .stage(main_shader_pipeline_stage)
            .layout(main_pipeline_layout);
        let final_shader_pipeline_stage = vk::PipelineShaderStageCreateInfo::default()
            .module(final_shader_module)
            .name(c"main")
            .stage(vk::ShaderStageFlags::COMPUTE);
        let final_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
            .stage(final_shader_pipeline_stage)
            .layout(final_pipeline_layout);
        let (main_pipeline, final_pipeline) = vulkan_base
            .device
            .create_compute_pipelines(
                pipeline_cache,
                &[main_pipeline_create_info, final_pipeline_create_info],
                None,
            )
            .unwrap()
            .into_iter()
            .collect_tuple()
            .unwrap();

        (
            pipeline_cache,
            main_pipeline_layout,
            main_pipeline,
            final_pipeline_layout,
            final_pipeline,
        )
    }

    unsafe fn _create_descriptor_sets(
        vulkan_base: &'a VulkanBase,
        descriptor_pool: vk::DescriptorPool,
        main_descriptor_set_layout: vk::DescriptorSetLayout,
        final_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (
        vk::DescriptorSet,
        vk::DescriptorSet,
        vk::DescriptorSet,
        vk::DescriptorSet,
    ) {
        // create descriptor sets
        let desctiptor_sets_layouts = [
            main_descriptor_set_layout,
            main_descriptor_set_layout,
            main_descriptor_set_layout,
            final_descriptor_set_layout,
        ];
        let descriptor_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&desctiptor_sets_layouts);

        vulkan_base
            .device
            .allocate_descriptor_sets(&descriptor_allocate_info)
            .unwrap()
            .into_iter()
            .collect_tuple()
            .unwrap()
    }

    unsafe fn _update_descriptor_sets(
        vulkan_base: &'a VulkanBase,
        world_gpu: &WorldGpu,
        working_buffer_1: &Buffer<Vector4<f32>>,
        working_buffer_2: &Buffer<Vector4<f32>>,
        image_view: vk::ImageView,
        samples_per_image: i32,
        world_descriptor_set: vk::DescriptorSet,
        work_descriptor_set_1: vk::DescriptorSet,
        work_descriptor_set_2: vk::DescriptorSet,
        final_descriptor_set: vk::DescriptorSet,
    ) -> () {
        // update descriptor sets
        let geometry_buffer_descriptor_info = [vk::DescriptorBufferInfo::default()
            .buffer(world_gpu.geometry.handle)
            .range(world_gpu.geometry.size)];
        let world_descriptor_write_geometry = vk::WriteDescriptorSet::default()
            .dst_set(world_descriptor_set)
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&geometry_buffer_descriptor_info);
        let material_buffer_descriptor_info = [vk::DescriptorBufferInfo::default()
            .buffer(world_gpu.materials.handle)
            .range(world_gpu.materials.size)];
        let world_descriptor_write_material = vk::WriteDescriptorSet::default()
            .dst_set(world_descriptor_set)
            .dst_binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&material_buffer_descriptor_info);

        let work_buffer_1_descriptor_info = [vk::DescriptorBufferInfo::default()
            .buffer(working_buffer_1.handle)
            .range(working_buffer_1.size)];
        let work_descriptor_1_write_input = vk::WriteDescriptorSet::default()
            .dst_set(work_descriptor_set_1)
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&work_buffer_1_descriptor_info);
        let work_descriptor_2_write_output = vk::WriteDescriptorSet::default()
            .dst_set(work_descriptor_set_2)
            .dst_binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&work_buffer_1_descriptor_info);

        let work_buffer_2_descriptor_info = [vk::DescriptorBufferInfo::default()
            .buffer(working_buffer_2.handle)
            .range(working_buffer_2.size)];
        let work_descriptor_1_write_output = vk::WriteDescriptorSet::default()
            .dst_set(work_descriptor_set_1)
            .dst_binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&work_buffer_2_descriptor_info);
        let work_descriptor_2_write_input = vk::WriteDescriptorSet::default()
            .dst_set(work_descriptor_set_2)
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&work_buffer_2_descriptor_info);

        let output_image_descriptor_info = [vk::DescriptorImageInfo::default()
            .image_view(image_view)
            .image_layout(vk::ImageLayout::GENERAL)];
        let final_descriptor_image_write = vk::WriteDescriptorSet::default()
            .dst_set(final_descriptor_set)
            .dst_binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&output_image_descriptor_info);

        let final_descriptor_source_write = vk::WriteDescriptorSet::default()
            .dst_set(final_descriptor_set)
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(if samples_per_image % 2 == 0 {
                &work_buffer_1_descriptor_info
            } else {
                &work_buffer_2_descriptor_info
            });

        let descriptor_writes = [
            world_descriptor_write_material,
            world_descriptor_write_geometry,
            work_descriptor_1_write_input,
            work_descriptor_1_write_output,
            work_descriptor_2_write_input,
            work_descriptor_2_write_output,
            final_descriptor_image_write,
            final_descriptor_source_write,
        ];
        vulkan_base
            .device
            .update_descriptor_sets(&descriptor_writes, &[]);
    }

    unsafe fn _create_barriers(
        working_buffer_1: &Buffer<'a, Vector4<f32>>,
        working_buffer_2: &Buffer<'a, Vector4<f32>>,
    ) -> (
        [vk::BufferMemoryBarrier2<'a>; 2],
        [vk::BufferMemoryBarrier2<'a>; 2],
    ) {
        // prepare sample barriers
        let buffer_1_write_wait = vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(working_buffer_1.handle)
            .offset(0)
            .size(working_buffer_1.size);
        let buffer_1_read_wait = vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(working_buffer_1.handle)
            .offset(0)
            .size(working_buffer_1.size);
        let buffer_2_write_wait = vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(working_buffer_2.handle)
            .offset(0)
            .size(working_buffer_2.size);
        let buffer_2_read_wait = vk::BufferMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(working_buffer_2.handle)
            .offset(0)
            .size(working_buffer_2.size);

        let barriers_1_to_2 = [buffer_1_read_wait, buffer_2_write_wait];
        let barriers_2_to_1 = [buffer_1_write_wait, buffer_2_read_wait];

        (barriers_1_to_2, barriers_2_to_1)
    }

    pub unsafe fn init(
        vulkan_base: &'a VulkanBase,
        image_width: u32,
        image_height: u32,
        samples_per_image: i32,
        world: WorldCpu,
    ) -> Self {
        let (render_shader, final_shader) = Self::_create_shaders(vulkan_base);

        let (command_pool, descriptor_pool) = Self::_create_memory_pools(vulkan_base);

        let (
            output_image_extent,
            output_image,
            output_image_memory,
            output_image_subresource,
            output_image_view,
        ) = Self::_create_image(vulkan_base, image_width, image_height);

        let (output_buffer, working_buffer_1, working_buffer_2) =
            Self::_create_buffers(&vulkan_base.allocator, image_height, image_width);

        let world_gpu = world.upload(&vulkan_base, &command_pool);

        let (main_descriptor_set_layout, final_descriptor_set_layout) =
            Self::_create_descriptor_layouts(vulkan_base);

        let (
            pipeline_cache,
            render_pipeline_layout,
            render_pipeline,
            finalize_pipeline_layout,
            finalize_pipeline,
        ) = Self::_create_pipelines(
            vulkan_base,
            main_descriptor_set_layout,
            final_descriptor_set_layout,
            render_shader,
            final_shader,
        );

        let (
            world_descriptor_set,
            work_descriptor_set_1,
            work_descriptor_set_2,
            final_descriptor_set,
        ) = Self::_create_descriptor_sets(
            vulkan_base,
            descriptor_pool,
            main_descriptor_set_layout,
            final_descriptor_set_layout,
        );

        Self::_update_descriptor_sets(
            vulkan_base,
            &world_gpu,
            &working_buffer_1,
            &working_buffer_2,
            output_image_view,
            samples_per_image,
            world_descriptor_set,
            work_descriptor_set_1,
            work_descriptor_set_2,
            final_descriptor_set,
        );

        let (barriers_1_to_2, barriers_2_to_1) =
            Self::_create_barriers(&working_buffer_1, &working_buffer_2);

        let fence_create_info =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let render_fence = vulkan_base
            .device
            .create_fence(&fence_create_info, None)
            .unwrap();

        Self {
            render_shader,
            final_shader,
            command_pool,
            descriptor_pool,
            output_image_extent,
            output_image,
            output_image_memory,
            output_image_subresource,
            output_image_view,
            output_buffer,
            working_buffer_1,
            working_buffer_2,
            world_gpu,
            main_descriptor_set_layout,
            final_descriptor_set_layout,
            pipeline_cache,
            render_pipeline_layout,
            render_pipeline,
            finalize_pipeline_layout,
            finalize_pipeline,
            world_descriptor_set,
            work_descriptor_set_1,
            work_descriptor_set_2,
            final_descriptor_set,
            barriers_1_to_2,
            barriers_2_to_1,
            render_fence,
        }
    }

    pub unsafe fn clear_buffers(
        &self,
        command_buffer: vk::CommandBuffer,
        vulkan_base: &VulkanBase,
    ) {
        //TODO: technically only one needs to be filled as other isn't read before writing
        // fill buffers with 0s
        vulkan_base.device.cmd_fill_buffer(
            command_buffer,
            self.working_buffer_1.handle,
            0,
            self.working_buffer_1.size,
            0,
        );
        vulkan_base.device.cmd_fill_buffer(
            command_buffer,
            self.working_buffer_2.handle,
            0,
            self.working_buffer_2.size,
            0,
        );

        // add synchronisation objects
        let initial_buffer_memory_barriers = [
            vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_READ)
                .src_queue_family_index(0)
                .dst_queue_family_index(0)
                .buffer(self.working_buffer_1.handle)
                .offset(0)
                .size(self.working_buffer_1.size),
            vk::BufferMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE)
                .src_queue_family_index(0)
                .dst_queue_family_index(0)
                .buffer(self.working_buffer_2.handle)
                .offset(0)
                .size(self.working_buffer_2.size),
        ];
        let initial_dependency_info =
            vk::DependencyInfo::default().buffer_memory_barriers(&initial_buffer_memory_barriers);
        vulkan_base
            .device
            .cmd_pipeline_barrier2(command_buffer, &initial_dependency_info);
    }

    pub unsafe fn wait_on_render_fence(&self, vulkan_base: &'a VulkanBase) {
        vulkan_base
            .device
            .wait_for_fences(&[self.render_fence], true, u64::MAX)
            .unwrap();
        vulkan_base
            .device
            .reset_fences(&[self.render_fence])
            .unwrap();
    }

    pub unsafe fn cleanup(mut self, vulkan_base: &'a VulkanBase) {
        vulkan_base.device.device_wait_idle().unwrap();

        vulkan_base
            .allocator
            .destroy_image(self.output_image, &mut self.output_image_memory);

        vulkan_base.device.destroy_fence(self.render_fence, None);

        vulkan_base
            .device
            .destroy_descriptor_set_layout(self.final_descriptor_set_layout, None);
        vulkan_base
            .device
            .destroy_descriptor_set_layout(self.main_descriptor_set_layout, None);
        vulkan_base
            .device
            .destroy_pipeline(self.finalize_pipeline, None);
        vulkan_base
            .device
            .destroy_pipeline(self.render_pipeline, None);
        vulkan_base
            .device
            .destroy_pipeline_cache(self.pipeline_cache, None);
        vulkan_base
            .device
            .destroy_pipeline_layout(self.finalize_pipeline_layout, None);
        vulkan_base
            .device
            .destroy_pipeline_layout(self.render_pipeline_layout, None);
        vulkan_base
            .device
            .destroy_image_view(self.output_image_view, None);
        vulkan_base
            .device
            .destroy_descriptor_pool(self.descriptor_pool, None);
        vulkan_base
            .device
            .destroy_command_pool(self.command_pool, None);
        vulkan_base
            .device
            .destroy_shader_module(self.final_shader, None);
        vulkan_base
            .device
            .destroy_shader_module(self.render_shader, None);
    }
}
