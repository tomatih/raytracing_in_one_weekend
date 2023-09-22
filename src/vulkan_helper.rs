use std::sync::Arc;

use vulkano::{
    instance::{Instance, InstanceCreateInfo},
    VulkanLibrary,
    device::{
        physical::PhysicalDevice,Device, Queue, QueueFlags, DeviceCreateInfo, QueueCreateInfo
    }
};

pub fn get_vulkan_instance() -> Arc<Instance>{
    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    Instance::new(library, InstanceCreateInfo::default()).expect("failed to create instance")
}

pub fn get_physical_device(instance: Arc<Instance>) -> Arc<PhysicalDevice>{
    //TODO: add choice criteria
    instance
        .enumerate_physical_devices()
        .expect("Could not enumerate devices")
        .next()
        .expect("no devices avaiable")
}

pub fn get_logical_device(physical_device: Arc<PhysicalDevice>) -> (Arc<Device>, Arc<Queue>){
    // find suitable queues
    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .enumerate()
        .position(|(_queue_family_index, queue_family_properties)|{
            queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
        }).expect("couldn't find a graphical queue family") as u32;

    //create object
    // get logical device and queues
    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo{
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        }
    ).expect("failed to create device");

    (device, queues.next().unwrap())
}