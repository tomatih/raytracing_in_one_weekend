use cgmath::InnerSpace;

use crate::{common::{Color, reflect}, material::Material, ray::Ray, hit_record::HitRecord};

pub struct Metal{
    pub albedo: Color
}

impl Material for Metal {
    fn scatter(&self, ray_in: Ray, hit_record: &HitRecord) -> Option<(Color, Ray)> {
        // calculate scattered ray
        let direction = reflect(ray_in.direction.normalize(), hit_record.normal);
        let scattered = Ray{ origin: hit_record.p, direction };
        
        // if ray pointed outward
        if scattered.direction.dot(hit_record.normal) > 0.0 {
            Some((self.albedo, scattered ))
        }
        else {
            None
        }
    }
}