use common::{Point3, Color};
use hit_record::HitRecord;
use hittable::Hittable;
use hittable_list::HittableList;
use image::{RgbImage, ImageBuffer};
use ray::Ray;
use cgmath::{InnerSpace, VectorSpace};
use rand::Rng;

use crate::{sphere::Sphere, common::to_pixel, camera::Camera};

mod common;
mod ray;
mod hit_record;
mod hittable;
mod sphere;
mod hittable_list;
mod camera;

/// Get colour of a ray
fn ray_color(ray: &Ray, world: &HittableList) -> Color {
    // check if ray hit any objects
    let mut hit_record = HitRecord::default();
    if world.hit(ray, 0.0, f32::INFINITY, &mut hit_record) {
        return 0.5 * (hit_record.normal + Color::new(1.0, 1.0, 1.0));
    };
    // if not return a sky gradient
    let unit_direction = ray.direction.normalize();
    let t = 0.5*(unit_direction.y + 1.0);
    Color::new(1.0, 1.0, 1.0).lerp(Color::new(0.5, 0.7, 1.0), t)
}

fn main() {
    // Image constants
    const ASPECT_RATIO: f32 = 16.0 / 9.0;
    const IMAGE_WIDTH: u32 = 400;
    const IMAGE_HEIGHT: u32 = (IMAGE_WIDTH as f32 /ASPECT_RATIO) as u32;
    const SAMPLES_PER_PIXEL: i32 = 100;

    // World
    let mut world = HittableList::new();
    world.add(Box::new(Sphere{ center: Point3::new(0.0, 0.0, -1.0), radius: 0.5 }));
    world.add(Box::new(Sphere{ center: Point3::new(0.0, -100.5, -1.0), radius: 100.0 }));

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
                pixel_color += ray_color(&ray, &world);
            }
            // print pixel
            img.put_pixel(i, IMAGE_HEIGHT-j-1, to_pixel(pixel_color, SAMPLES_PER_PIXEL));
        }
    }
    println!("");
    // save image
    img.save("out.png").expect("Faild to save image");
}
