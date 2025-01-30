use ash::{
    khr::{surface, swapchain},
    vk::{self, SurfaceKHR, SwapchainKHR},
};
use sdl3::{video::Window, Sdl, VideoSubsystem};

use crate::vulkan_helper::VulkanBase;

pub struct WindowManager {
    // SDL3 members
    pub sdl_context: Sdl,
    pub video_subsystem: VideoSubsystem,
    pub window: Window,
    // base vulkan members
    pub vulkan_base: VulkanBase,
    pub swapchain_loader: swapchain::Device,
    pub surface_loader: surface::Instance,
    // vulkan present objects
    pub surface: SurfaceKHR,
    pub swapchain: SwapchainKHR,
    pub swapchain_extent: vk::Extent2D,
    pub present_images: Vec<vk::Image>,
    //  vulkan sync objects
    pub image_acquire_semaphore: vk::Semaphore,
    pub rendering_completed_semaphore: vk::Semaphore,
}

impl WindowManager {
    pub unsafe fn new(window_width: u32, window_height: u32) -> Self {
        // init SDL3
        let sdl_context = sdl3::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window("Raytravcing in a weekend", window_width, window_height)
            .position_centered()
            .vulkan()
            .build()
            .unwrap();

        // init vulkan
        let sdl_extensions = window.vulkan_instance_extensions().unwrap();
        let vulkan_base = VulkanBase::new(&sdl_extensions);

        // extension loaders
        let surface_loader = surface::Instance::new(&vulkan_base.entry, &vulkan_base.instance);
        let swapchain_loader = swapchain::Device::new(&vulkan_base.instance, &vulkan_base.device);

        // surface init
        let surface = window
            .vulkan_create_surface(vulkan_base.instance.handle())
            .unwrap();

        // swapchain init
        let swapchain_extent = vk::Extent2D {
            width: window_width,
            height: window_height,
        };
        let swapchain = Self::create_swapchain(
            &vulkan_base.physical_device,
            &surface,
            &surface_loader,
            &swapchain_loader,
            &swapchain_extent,
        );
        let present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();

        // semaphore init
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let image_acquire_semaphore = vulkan_base
            .device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap();
        let rendering_completed_semaphore = vulkan_base
            .device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap();

        Self {
            sdl_context,
            video_subsystem,
            window,
            vulkan_base,
            swapchain_loader,
            surface_loader,
            surface,
            swapchain,
            swapchain_extent,
            image_acquire_semaphore,
            rendering_completed_semaphore,
            present_images,
        }
    }

    unsafe fn create_swapchain(
        physical_device: &vk::PhysicalDevice,
        surface: &SurfaceKHR,
        surface_loader: &surface::Instance,
        swapchain_loader: &swapchain::Device,
        swapchain_extent: &vk::Extent2D,
    ) -> SwapchainKHR {
        // get a UNORM format (guaranteed to be present by spec)
        let surface_formats = surface_loader
            .get_physical_device_surface_formats(*physical_device, *surface)
            .unwrap();
        let surface_format = surface_formats
            .iter()
            .filter(|surface_format| {
                surface_format.format == vk::Format::R8G8B8A8_UNORM
                    || surface_format.format == vk::Format::B8G8R8A8_UNORM
            })
            .collect::<Vec<_>>()[0];

        // get capabilities
        let surface_capabilities = surface_loader
            .get_physical_device_surface_capabilities(*physical_device, *surface)
            .unwrap();

        // try tripple buffering
        let desired_image_count = 3;
        let desired_image_count = if surface_capabilities.max_image_count == 0 {
            desired_image_count.max(surface_capabilities.min_image_count)
        } else {
            desired_image_count.clamp(
                surface_capabilities.min_image_count,
                surface_capabilities.max_image_count,
            )
        };

        //TODO: could this be a problem if it isn't identity??
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        // create swapchain
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(*surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(*swapchain_extent)
            .image_usage(vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .image_array_layers(1);

        swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap()
    }
}

impl Drop for WindowManager {
    fn drop(&mut self) {
        unsafe {
            // make sure nothing is happening
            self.vulkan_base.device.device_wait_idle().unwrap();

            self.vulkan_base
                .device
                .destroy_semaphore(self.rendering_completed_semaphore, None);
            self.vulkan_base
                .device
                .destroy_semaphore(self.image_acquire_semaphore, None);

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
