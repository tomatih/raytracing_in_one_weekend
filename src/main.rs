// project modules
mod camera;
mod common;
mod hit_system;
mod materials;
mod objects;
mod ray;
// external imports
use cgmath::{Deg, InnerSpace, VectorSpace};
use image::{ImageBuffer, RgbImage};
use rand::Rng;
use std::rc::Rc;
// own imports
use camera::Camera;
use common::{to_pixel, Color, Point3};
use hit_system::{Hittable, HittableList};
use materials::{Lambertian, Metal};
use objects::Sphere;
use ray::Ray;

use crate::{common::Vec3, materials::Dielectric};

/// Get colour of a ray
fn ray_color(ray: Ray, world: &HittableList, depth: i32) -> Color {
    let mut output_color = Color::new(1.0, 1.0, 1.0);
    let mut current_ray = ray;

    for _ in 0..depth {
        if let Some(hit_record) = world.hit(&current_ray, 0.001, f32::INFINITY) {
            let new_color = if let Some((attenuation, scattered)) = world
                .get_material(hit_record.mat_index)
                .scatter(current_ray, &hit_record)
            {
                // bounce new ray
                current_ray = scattered;
                attenuation
            } else {
                return Color::new(0.0, 0.0, 0.0);
            };

            output_color = Color::new(
                output_color.x * new_color.x,
                output_color.y * new_color.y,
                output_color.z * new_color.z,
            );
        } else {
            let unit_direction = current_ray.direction.normalize();
            let t = 0.5 * (unit_direction.y + 1.0);
            let sky_color = Color::new(1.0, 1.0, 1.0).lerp(Color::new(0.5, 0.7, 1.0), t);
            output_color = Color::new(
                output_color.x * sky_color.x,
                output_color.y * sky_color.y,
                output_color.z * sky_color.z,
            );
            break;
        };
    }

    output_color
}

/// Generate a scene fileld with random spheres
fn randon_scene() -> HittableList {
    let mut out = HittableList::new();

    // the ground
    out.add_material(Box::new(Lambertian {
        albedo: Color::new(0.5, 0.5, 0.5),
    }));
    out.add_object(Box::new(Sphere {
        center: Vec3::new(0.0, -1000.0, 0.0),
        radius: 1000.0,
        material: 0,
    }));

    // the reandom speres
    let mut rng = rand::thread_rng();
    for a in -11..11 {
        for b in -11..11 {
            let material_choice = rng.gen::<f32>();

            out.add_material(if material_choice < 0.8 {
                Box::new(Lambertian {
                    albedo: Color::new(
                        rng.gen::<f32>() * rng.gen::<f32>(),
                        rng.gen::<f32>() * rng.gen::<f32>(),
                        rng.gen::<f32>() * rng.gen::<f32>(),
                    ),
                })
            } else if material_choice < 0.95 {
                Box::new(Metal::new(
                    Color::new(rng.gen(), rng.gen(), rng.gen()),
                    rng.gen_range(0.0..0.5),
                ))
            } else {
                Box::new(Dielectric { ir: 1.5 })
            });

            let center = Point3::new(
                (a as f32) + 0.9 * rng.gen::<f32>(),
                0.2,
                (b as f32) + 0.9 * rng.gen::<f32>(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).magnitude() > 0.9 {
                out.add_object(Box::new(Sphere {
                    center,
                    radius: 0.2,
                    material: out.get_last_material_index(),
                }));
            }
        }
    }

    out.add_material(Box::new(Dielectric { ir: 1.5 }));
    out.add_object(Box::new(Sphere {
        center: Vec3::new(0.0, 1.0, 0.0),
        radius: 1.0,
        material: out.get_last_material_index(),
    }));

    out.add_material(Box::new(Lambertian {
        albedo: Color::new(0.4, 0.2, 0.1),
    }));
    out.add_object(Box::new(Sphere {
        center: Vec3::new(-4.0, 1.0, 0.0),
        radius: 1.0,
        material: out.get_last_material_index(),
    }));

    out.add_material(Box::new(Metal::new(Color::new(0.7, 0.6, 0.7), 0.0)));
    out.add_object(Box::new(Sphere {
        center: Vec3::new(4.0, 1.0, 0.0),
        radius: 1.0,
        material: out.get_last_material_index(),
    }));

    out
}

fn main() {
    // Image constants
    const ASPECT_RATIO: f32 = 3.0 / 2.0;
    const IMAGE_WIDTH: u32 = 1200;
    const IMAGE_HEIGHT: u32 = (IMAGE_WIDTH as f32 / ASPECT_RATIO) as u32;
    const SAMPLES_PER_PIXEL: i32 = 5;
    const MAX_DEPTH: i32 = 5;

    // World
    let world = randon_scene();

    // camera
    let look_from = Point3::new(13.0, 2.0, 3.0);
    let look_at = Point3::new(0.0, 0.0, 0.0);
    let up = Vec3::unit_y();
    let distance_to_focus = 10.0;
    let apeture = 0.1;
    let camera = Camera::new(
        look_from,
        look_at,
        up,
        Deg(20.0),
        ASPECT_RATIO,
        apeture,
        distance_to_focus,
    );

    // Allocate image buffer
    let mut img: RgbImage = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);

    // Render image
    let mut rng = rand::thread_rng();
    for j in (0..IMAGE_HEIGHT).rev() {
        print!("\rScanlines remaining {:3}", j);
        for i in 0..IMAGE_WIDTH {
            let mut pixel_color = Color::new(0.0, 0.0, 0.0);
            for _ in 0..SAMPLES_PER_PIXEL {
                let u = (i as f32 + rng.gen::<f32>()) / (IMAGE_WIDTH - 1) as f32;
                let v = (j as f32 + rng.gen::<f32>()) / (IMAGE_HEIGHT - 1) as f32;
                let ray = camera.get_ray(u, v);
                pixel_color += ray_color(ray, &world, MAX_DEPTH);
            }
            // print pixel
            img.put_pixel(
                i,
                IMAGE_HEIGHT - j - 1,
                to_pixel(pixel_color, SAMPLES_PER_PIXEL),
            );
        }
    }
    println!("");
    // save image
    img.save("out.png").expect("Faild to save image");
}
