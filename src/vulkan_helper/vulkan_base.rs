use ash::{khr::{swapchain, surface}, vk};
use renderdoc::{RenderDoc, V130};

pub struct VulkanBase {
    pub _entry: ash::Entry, // base DLL/SO
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub queue: vk::Queue,
    pub command_pool: vk::CommandPool,
    pub fence: vk::Fence,
    pub swapchain_loader: swapchain::Device,
    pub surface_loader: surface::Instance,
    #[cfg(debug_assertions)]
    rd: Option<RenderDoc<V130>>,
}

impl VulkanBase {
    unsafe fn get_physical_device(instance: &ash::Instance) -> vk::PhysicalDevice {
        let physical_devices = instance.enumerate_physical_devices().unwrap();

        let physical_device = physical_devices
            .iter()
            .find_map(|physical_device| {
                // make sure supports vulkan 1.3
                let properties = instance.get_physical_device_properties(*physical_device);
                let modern_enough = properties.api_version >= vk::API_VERSION_1_3;

                // make sure first queue supports compute
                let supports_compute = instance
                    .get_physical_device_queue_family_properties(*physical_device)[0]
                    .queue_flags
                    .contains(vk::QueueFlags::COMPUTE);

                if supports_compute && modern_enough {
                    Some(*physical_device)
                } else {
                    None
                }
            })
            .expect("Can't find suitable device");
        let properties = instance.get_physical_device_properties(physical_device);
        println!(
            "Chosen: {}",
            properties.device_name_as_c_str().unwrap().to_str().unwrap()
        );

        physical_device
    }

    unsafe fn get_device_and_queue(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
    ) -> (ash::Device, vk::Queue) {
        // queue setup
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(0) // guaranteed to work by selector
            .queue_priorities(&[1.0]);

        // get support for everything up to 1.3 (guaranteed by selector)
        let mut features_1_3 = vk::PhysicalDeviceVulkan13Features::default();
        let mut features_1_2 = vk::PhysicalDeviceVulkan12Features::default();
        let mut features_1_1 = vk::PhysicalDeviceVulkan11Features::default();
        let features = instance.get_physical_device_features(*physical_device);
        let mut features2 = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut features_1_1)
            .push_next(&mut features_1_2)
            .push_next(&mut features_1_3)
            .features(features);
        instance.get_physical_device_features2(*physical_device, &mut features2);

        // enable swapchain extension
         let device_extensions = [
            swapchain::NAME.as_ptr()
        ];

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_create_info))
            .enabled_extension_names(&device_extensions)
            .push_next(&mut features2);
        let device = instance
            .create_device(*physical_device, &device_create_info, None)
            .expect("Can't create device");
        let queue = device.get_device_queue(0, 0); // guaranteed by selector

        (device, queue)
    }

    pub unsafe fn new(instance_extensions: &Vec<String>) -> Self {
        // Get DLL/SO
        let _entry = ash::Entry::load().unwrap();

        // get instance// Get Instance
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Ash Playground")
            .application_version(0)
            .engine_name(c"No engine")
            .api_version(vk::API_VERSION_1_3);
        let instance_extensions: Vec<_> = instance_extensions.iter().map(|f| f.as_ptr() as *const i8).collect();
        let create_info = vk::InstanceCreateInfo::default().application_info(&app_info).enabled_extension_names(&instance_extensions.as_slice());
        let instance = _entry.create_instance(&create_info, None).unwrap();

        // get physical device
        let physical_device = Self::get_physical_device(&instance);

        let (device, queue) = Self::get_device_and_queue(&instance, &physical_device);

        let command_pool_create_info = vk::CommandPoolCreateInfo::default().queue_family_index(0);
        let command_pool = device
            .create_command_pool(&command_pool_create_info, None)
            .unwrap();

        let fence_create_info =
            vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);
        let fence = device.create_fence(&fence_create_info, None).unwrap();

        // extension loaders
        let surface_loader = surface::Instance::new(&_entry, &instance);
        let swapchain_loader = swapchain::Device::new(&instance, &device);

        // init renderdoc
        #[cfg(debug_assertions)]
        let mut rd: Option<RenderDoc<V130>> = RenderDoc::new().ok();
        #[cfg(debug_assertions)]
        if let Some(x) = rd.as_mut() {
            x.start_frame_capture(std::ptr::null(), std::ptr::null());
        }

        Self {
            _entry,
            instance,
            physical_device,
            device,
            queue,
            command_pool,
            fence,
            swapchain_loader,
            surface_loader,
            #[cfg(debug_assertions)]
            rd

        }
    }

    pub unsafe fn start_command_buffer(&self) -> vk::CommandBuffer {
        let command_buffer_allocation_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(1)
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = self
            .device
            .allocate_command_buffers(&command_buffer_allocation_info)
            .unwrap()[0];

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        self.device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .unwrap();

        command_buffer
    }

    pub unsafe fn submit_command_buffer(&self, command_buffer: vk::CommandBuffer, fence: Option<vk::Fence>) {
        self.device.end_command_buffer(command_buffer).unwrap();

        // submit prepare
        let submit_infos = [vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer)];
        let to_submit = [vk::SubmitInfo2::default().command_buffer_infos(&submit_infos)];

        // make sure the previous one has finished
        if fence.is_none(){
            self.device
                .wait_for_fences(&[self.fence], true, u64::MAX)
                .unwrap();
            self.device.reset_fences(&[self.fence]).unwrap();
        }

        // start new submission
        self.device
            .queue_submit2(self.queue, &to_submit, fence.unwrap_or(self.fence))
            .unwrap();
    }
}

impl Drop for VulkanBase {
    fn drop(&mut self) {

        #[cfg(debug_assertions)]
        if let Some(x) = self.rd.as_mut() {
            x.end_frame_capture(std::ptr::null(), std::ptr::null());
        }

        unsafe {
            // make sure nothing is being used
            self.device.device_wait_idle().unwrap();

            self.device.destroy_fence(self.fence, None);
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
