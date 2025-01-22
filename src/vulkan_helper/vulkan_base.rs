use ash::vk;

pub struct VulkanBase {
    pub _entry: ash::Entry, // base DLL/SO
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub queue: vk::Queue,
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
        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_create_info))
            .push_next(&mut features2);
        let device = instance
            .create_device(*physical_device, &device_create_info, None)
            .expect("Can't create device");
        let queue = device.get_device_queue(0, 0); // guaranteed by selector

        (device, queue)
    }

    pub unsafe fn new() -> Self {
        // Get DLL/SO
        let _entry = ash::Entry::load().unwrap();

        // get instance// Get Instance
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Ash Playground")
            .application_version(0)
            .engine_name(c"No engine")
            .api_version(vk::API_VERSION_1_3);
        let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);
        let instance = _entry.create_instance(&create_info, None).unwrap();

        // get physical device
        let physical_device = Self::get_physical_device(&instance);

        let (device, queue) = Self::get_device_and_queue(&instance, &physical_device);

        Self {
            _entry,
            instance,
            physical_device,
            device,
            queue,
        }
    }
}

impl Drop for VulkanBase {
    fn drop(&mut self) {
        unsafe {
            // make sure nothing is being used
            self.device.device_wait_idle().unwrap();

            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
