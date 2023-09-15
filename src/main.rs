// project modules
/*
mod common;
mod ray;
mod camera;
mod objects;
mod materials;
mod hit_system;
*/
// external imports
/*
use std::rc::Rc;
use rand::Rng;
use image::{RgbImage, ImageBuffer};
use cgmath::{InnerSpace, VectorSpace, Deg};
*/
// Vulkan inports
use vulkano::{
    VulkanLibrary,
    instance::{Instance, InstanceCreateInfo},
    device::{QueueFlags, Device, DeviceCreateInfo, QueueCreateInfo},
    memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage},
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{
        allocator::{
            StandardCommandBufferAllocator,
            StandardCommandBufferAllocatorCreateInfo
        },
        AutoCommandBufferBuilder,
        CommandBufferUsage,
        CopyBufferInfo
    },
    sync::{self, GpuFuture}
};
// own imports
/*
use common::{Point3, Color, to_pixel};
use ray::Ray;
use camera::Camera;
use objects::Sphere;
use materials::{Lambertian, Metal};
use hit_system::{HittableList, Hittable};

use crate::{materials::Dielectric, common::Vec3};
*/

/*
/// Get colour of a ray
fn ray_color(ray: Ray, world: &HittableList, depth: i32) -> Color {
    // Exceded ray bounce limit
    if depth <= 0 {
        return Color::new(0.0, 0.0, 0.0);
    }
    // check if ray hit any objects
    if let Some(hit_record) =  world.hit(&ray, 0.001, f32::INFINITY){
        // if the ray scatters further
        if let Some((attenuation, scattered)) = hit_record.mat_ptr.scatter(ray, &hit_record){
            // bounce new ray
            let result =  ray_color(scattered, world, depth-1);
            Color::new(
                result.x * attenuation.x,
                result.y * attenuation.y,
                result.z * attenuation.z
            )
        }
        // ray got absorbed
        else{
            Color::new(0.0, 0.0, 0.0)
        }
    }
    // if not return a sky gradient
    else{
        let unit_direction = ray.direction.normalize();
        let t = 0.5*(unit_direction.y + 1.0);
        Color::new(1.0, 1.0, 1.0).lerp(Color::new(0.5, 0.7, 1.0), t)
    }
}

/// Generate a scene fileld with random spheres
fn randon_scene() -> HittableList{
    let mut out = HittableList::new();

    // the ground
    let ground_material = Rc::new(Lambertian{ albedo: Color::new(0.5, 0.5, 0.5)});
    out.add(Box::new(Sphere{ center: Vec3::new(0.0, -1000.0, 0.0), radius: 1000.0, material: ground_material}));

    // the reandom speres
    let mut rng = rand::thread_rng();
    for a in -11..11 {
        for b in -11..11{
            let material_coice = rng.gen::<f32>();
            let center = Point3::new((a as f32)+0.9*rng.gen::<f32>(), 0.2, (b as f32)+0.9*rng.gen::<f32>());

            if (center - Vec3::new(4.0, 0.2, 0.0)).magnitude() > 0.9{
                out.add(Box::new(Sphere{ center, radius: 0.2,
                    material: if material_coice  < 0.8{
                        Rc::new(Lambertian{albedo: Color::new(
                            rng.gen::<f32>() * rng.gen::<f32>(),
                            rng.gen::<f32>() * rng.gen::<f32>(),
                            rng.gen::<f32>() * rng.gen::<f32>()
                        )})
                    }
                    else if material_coice < 0.95{
                        Rc::new(Metal::new(Color::new(rng.gen(), rng.gen(), rng.gen()), rng.gen_range(0.0..0.5)))
                    }
                    else{
                        Rc::new(Dielectric{ ir: 1.5 })
                    }
                }));
            }

        }
    }

    let material1 = Rc::new(Dielectric{ir:1.5});
    out.add(Box::new(Sphere{ center: Vec3::new(0.0, 1.0, 0.0), radius: 1.0, material: material1 }));

    let material2 = Rc::new(Lambertian{albedo: Color::new(0.4, 0.2, 0.1)});
    out.add(Box::new(Sphere{ center: Vec3::new(-4.0, 1.0, 0.0), radius: 1.0, material: material2 }));

    let material3 = Rc::new(Metal::new(Color::new(0.7, 0.6, 0.7), 0.0));
    out.add(Box::new(Sphere{ center: Vec3::new(4.0, 1.0, 0.0), radius: 1.0, material: material3 }));

    out
}


fn old_main() {
    // Image constants
    const ASPECT_RATIO: f32 = 3.0 / 2.0;
    const IMAGE_WIDTH: u32 = 1200;
    const IMAGE_HEIGHT: u32 = (IMAGE_WIDTH as f32 /ASPECT_RATIO) as u32;
    const SAMPLES_PER_PIXEL: i32 = 500;
    const MAX_DEPTH: i32 = 50;

    // World
    let world = randon_scene();

    // camera
    let look_from = Point3::new(13.0, 2.0, 3.0);
    let look_at = Point3::new(0.0, 0.0, 0.0);
    let up = Vec3::unit_y();
    let distance_to_focus = 10.0;
    let apeture = 0.1;
    let camera = Camera::new(look_from,look_at,up,Deg(20.0), ASPECT_RATIO, apeture, distance_to_focus);

    // Allocate image buffer
    let mut img: RgbImage = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);

    // Render image
    let mut rng = rand::thread_rng();
    for j in (0..IMAGE_HEIGHT).rev() {
        print!("\rScanlines remaining {:3}", j);
        for i in 0..IMAGE_WIDTH{
            let mut pixel_color = Color::new(0.0, 0.0, 0.0);
            for _ in 0..SAMPLES_PER_PIXEL{
                let u = (i as f32 + rng.gen::<f32>()) / (IMAGE_WIDTH - 1) as f32;
                let v = (j as f32 + rng.gen::<f32>()) / (IMAGE_HEIGHT - 1) as f32;
                let ray = camera.get_ray(u, v);
                pixel_color += ray_color(ray, &world, MAX_DEPTH);
            }
            // print pixel
            img.put_pixel(i, IMAGE_HEIGHT-j-1, to_pixel(pixel_color, SAMPLES_PER_PIXEL));
        }
    }
    println!("");
    // save image
    img.save("out.png").expect("Faild to save image");
}
*/


fn main (){
    // init vulkan
    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let instance = Instance::new(library, InstanceCreateInfo::default()).expect("failed to create instance");
    // get physical device
    let physical_device = instance
        .enumerate_physical_devices()
        .expect("Could not enumerate devices")
        .next()
        .expect("no devices avaiable");

    // find suitable queues
    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .enumerate()
        .position(|(_queue_family_index, queue_family_properties)|{
            queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
        }).expect("couldn't find a graphical queue family") as u32;

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

    // get the queue
    let queue = queues.next().unwrap();
    // create a memory allocator
    let memory_allocator = StandardMemoryAllocator::new_default(device.clone());
    // create a source buffer
    let source_content: Vec<i32> = (0..64).collect();
    let source = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo{
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo{
            usage: MemoryUsage::Upload,
            ..Default::default()
        },
        source_content
    ).expect("failed to create source buffer");

    // create destination buffer
    let destination_content: Vec<i32> = (0..64).map(|_| 0).collect();
    let destination = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo{
            usage: BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
    AllocationCreateInfo{
        usage: MemoryUsage::Download,
        ..Default::default()
    },
    destination_content
    ).expect("failed to create destination buffer");

    // command buffer allocator
    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(), 
        StandardCommandBufferAllocatorCreateInfo::default()
    );
    
    // command buffer builder
    let mut builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue_family_index,
        CommandBufferUsage::OneTimeSubmit
    ).unwrap();

    builder
        .copy_buffer(CopyBufferInfo::buffers(source.clone(), destination.clone()))
        .unwrap();

    let command_buffer = builder.build().unwrap();

    // submit work
    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    let src_content = source.read().unwrap();
    let dest_content = destination.read().unwrap();
    assert_eq!(&*src_content, &*dest_content);

    println!("Everything worked!");
}