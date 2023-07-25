use cgmath::InnerSpace;
use rand::Rng;

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

        let canot_refrect = refraction_ratio* sin_theta > 1.0;
        let mut rng = rand::thread_rng();
        let direction = if canot_refrect || reflectance(cos_theta, refraction_ratio) > rng.gen::<f32>(){
            reflect(unit_direction, hit_record.normal)
        } else {
            refract(unit_direction, hit_record.normal, refraction_ratio)
        };
        
        Some((Color::new(1.0, 1.0, 1.0), Ray{ origin: hit_record.p, direction}))
    }
}

fn reflectance(cosine: f32, ref_idx: f32) -> f32{
    let r0 = (1.0-ref_idx) / (1.0+ref_idx);
    let r0 = r0*r0;
    r0 + (1.0-r0)*(1.0- cosine).powi(5)
}