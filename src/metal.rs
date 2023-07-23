use cgmath::InnerSpace;

use crate::{common::{Color, reflect}, material::Material, ray::Ray, hit_record::HitRecord};

pub struct Metal{
    albedo: Color
}

impl Material for Metal {
    fn scatter(&self, ray_in: Ray, hit_record: &mut HitRecord, attenuation: &Color, sattered: &Ray) -> bool {
        let direction = reflect(ray_in.direction.normalize(), hit_record.normal);
        *sattered = Ray{ origin: hit_record.p, direction };
        *attenuation = self.albedo;
        sattered.direction.dot(hit_record.normal) > 0.0
    }
}