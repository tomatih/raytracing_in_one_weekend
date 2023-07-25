use cgmath::InnerSpace;

use crate::{ray::Ray, hit_system::HitRecord, common::{Color, refract, reflect}};

use super::Material;

pub struct Dielectric{
    pub ir: f32   
}

impl Material for Dielectric {
    fn scatter(&self, ray_in: Ray, hit_record: &HitRecord) -> Option<(Color, Ray)> {
        let refraction_ratio = if hit_record.front_face {
            1.0/self.ir
        }
        else{
            self.ir
        };

        let unit_direction = ray_in.direction.normalize();

        let cos_theta = hit_record.normal.dot(-unit_direction).min(1.0);
        let sin_theta = (1.0 - cos_theta*cos_theta).sqrt();

        let direction = if refraction_ratio* sin_theta > 1.0{
            reflect(unit_direction, hit_record.normal)
        } else {
            refract(unit_direction, hit_record.normal, refraction_ratio)
        };
        
        Some((Color::new(1.0, 1.0, 1.0), Ray{ origin: hit_record.p, direction}))
    }
}