mod vulkan_base;
mod buffer;

use std::io::Cursor;

use ash::{util::read_spv, vk};
pub use vulkan_base::VulkanBase;
pub use buffer::Buffer;

pub unsafe fn load_shader(vulkan_base: &VulkanBase, shader_bytes: &[u8]) -> vk::ShaderModule{
	let mut shader_file = Cursor::new(shader_bytes);
	let shader_code = read_spv(&mut shader_file).unwrap();
	let shader_module_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);
    vulkan_base.device.create_shader_module(&shader_module_info, None).unwrap()
}
