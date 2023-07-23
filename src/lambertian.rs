use crate::{common::{Color, random_unit_vector, near_zero}, material::Material, ray::Ray, hit_record::HitRecord};

pub struct Lambertian{
    albedo: Color
}

impl Material for Lambertian {z
    fn scatter(&self, ray_in: Ray, hit_record: &mut HitRecord, attenuation: &Color, scattered: &Ray) -> bool {
        let mut direction = hit_record.normal + random_unit_vector();
        if near_zero(direction){
            direction = hit_record.normal;
        }
        
        *scattered = Ray{ origin: hit_record.p, direction };
        *attenuation = self.albedo;
        true
    }
}