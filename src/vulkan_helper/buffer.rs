use std::marker::PhantomData;

use ash::vk;
use vk_mem::Alloc;

pub struct Buffer<'a, T> {
    _phantom_data: PhantomData<T>,
    allocator: &'a vk_mem::Allocator,
    pub handle: vk::Buffer,
    pub memory: vk_mem::Allocation,
    pub count: usize,
    pub size: vk::DeviceSize,
}

impl<'a, T> Buffer<'a, T> {
    pub unsafe fn new(
        allocator: &'a vk_mem::Allocator,
        usage: vk::BufferUsageFlags,
        count: usize,
        allocation_info: vk_mem::AllocationCreateInfo,
    ) -> Self {
        let size = (std::mem::size_of::<T>() * count) as vk::DeviceSize;
        let create_info = vk::BufferCreateInfo::default().size(size).usage(usage);

        let (handle, memory) = allocator
            .create_buffer(&create_info, &allocation_info)
            .unwrap();
        Self {
            allocator,
            handle,
            memory,
            _phantom_data: PhantomData,
            count,
            size,
        }
    }

    pub unsafe fn fill_buffer(&mut self, data: Vec<T>) {
        assert!(data.len() <= self.count);

        let buff_mem_map = self.allocator.map_memory(&mut self.memory).unwrap();
        std::ptr::copy_nonoverlapping(data.as_ptr(), buff_mem_map as *mut T, data.len());
        self.allocator.unmap_memory(&mut self.memory);
    }
}

impl<T> Drop for Buffer<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.allocator.destroy_buffer(self.handle, &mut self.memory);
        }
    }
}
