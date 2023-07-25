// project modules
mod common;
mod ray;
mod camera;
mod objects;
mod materials;
mod hit_system;
// external imports
use std::rc::Rc;
use rand::Rng;
use image::{RgbImage, ImageBuffer};
use cgmath::{InnerSpace, VectorSpace};
// own imports
use common::{Point3, Color, to_pixel};
use ray::Ray;
use camera::Camera;
use objects::Sphere;
use materials::{Lambertian, Metal};
use hit_system::{HittableList, Hittable};

use crate::materials::Dielectric;


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

fn main() {
    // Image constants
    const ASPECT_RATIO: f32 = 16.0 / 9.0;
    const IMAGE_WIDTH: u32 = 400;
    const IMAGE_HEIGHT: u32 = (IMAGE_WIDTH as f32 /ASPECT_RATIO) as u32;
    const SAMPLES_PER_PIXEL: i32 = 100;
    const MAX_DEPTH: i32 = 50;

    // World
    let mut world = HittableList::new();
    // materials
    let material_ground = Rc ::new(Lambertian{ albedo: Color::new(0.8, 0.8, 0.0) });
    let material_center = Rc::new(Lambertian{ albedo: Color::new(0.1, 0.2, 0.5) });
    //let material_left = Rc::new(Metal::new(Color::new(0.8, 0.8, 0.8), 0.3));
    let material_left =  Rc::new(Dielectric{ir: 1.5});
    let material_right = Rc::new(Metal::new(Color::new(0.8, 0.6, 0.2), 0.0));
    //let material_right = Rc::new(TrueBlack);
    // objects
    world.add(Box::new(Sphere{ center: Point3::new(0.0, -100.5, -1.0), radius: 100.0, material: material_ground }));
    world.add(Box::new(Sphere{ center: Point3::new(0.0, 0.0, -1.0), radius: 0.5, material: material_center }));
    world.add(Box::new(Sphere{ center: Point3::new(-1.0, 0.0, -1.0), radius: 0.5, material: material_left }));
    world.add(Box::new(Sphere{ center: Point3::new(1.0, 0.0, -1.0), radius: 0.5, material: material_right }));


    // camera
    let camera = Camera::new();

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
