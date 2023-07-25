use cgmath::InnerSpace;

use crate::{ray::Ray, hit_system::HitRecord, common::{Color, refract}};

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
        let direction = refract(unit_direction, hit_record.normal, refraction_ratio);
        
        Some((Color::new(1.0, 1.0, 1.0), Ray{ origin: hit_record.p, direction}))
    }
}