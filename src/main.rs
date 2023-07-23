use common::{Point3, Color};
use hit_record::HitRecord;
use hittable_list::HittableList;
use image::{RgbImage, ImageBuffer};
use cgmath::{Vector3, InnerSpace, VectorSpace};
use ray::Ray;

use crate::{sphere::Sphere, common::to_pixel};

mod common;
mod ray;
mod hit_record;
mod hittable;
mod sphere;
mod hittable_list;
use hittable_list::HittableList;

/// Return a point at which a ray hit a spere, or -1.0 if missed
fn hit_sphere(center: Point3, radius: f32, r: &Ray) -> f32{
    let oc = r.origin - center;
    let a = r.direction.magnitude2();
    let half_b = oc.dot(r.direction);
    let c = oc.magnitude2() - radius*radius;
    let discriminant = half_b*half_b - a*c;
    if discriminant < 0.0 {
        -1.0
    } else {
        (-half_b - discriminant.sqrt()) / a
    }
}

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

    // World
    let mut world = HittableList::new();
    world.add(Box::new(Sphere{ center: Point3::new(0.0, 0.0, -1.0), radius: 0.5 }));
    world.add(Box::new(Sphere{ center: Point3::new(0.0, -100.5, -1.0), radius: 100.0 }));

    // camera
    let viewport_height = 2.0;
    let viewport_width = ASPECT_RATIO * viewport_height;
    let focal_length = 1.0;

    let origin = Vector3::new(0.0, 0.0, 0.0);
    let horizontal = Vector3::new(viewport_width, 0.0, 0.0);
    let vertical = Vector3::new(0.0, viewport_height, 0.0);
    let lower_left_corner = origin - horizontal/2.0 - vertical/2.0 - Vector3::new(0.0, 0.0, focal_length);

    // Allocate image buffer
    let mut img: RgbImage = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);

    // Render image
    for j in (0..IMAGE_HEIGHT).rev() {
        print!("\rScanlines remaining {:3}", j);
        for i in 0..IMAGE_WIDTH{
            // create color
            let u = i as f32 / (IMAGE_WIDTH - 1) as f32;
            let v = j as f32 / (IMAGE_HEIGHT - 1) as f32;
            let r = Ray{ origin,
                direction: lower_left_corner+ u*horizontal + v*vertical - origin
            };
            let color = ray_color(&r, &world);
            // print pixel
            img.put_pixel(i, IMAGE_HEIGHT-j-1, to_pixel(color));
        }
    }
    println!("");
    // save image
    img.save("out.png").expect("Faild to save image");
}
