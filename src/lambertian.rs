use crate::{common::{Color, random_unit_vector, near_zero}, material::Material, ray::Ray, hit_record::HitRecord};

pub struct Lambertian{
    pub albedo: Color
}

impl Material for Lambertian {
    fn scatter(&self, _ray_in: Ray, hit_record: &HitRecord) -> Option<(Color, Ray)> {
        let mut direction = hit_record.normal + random_unit_vector();
        if near_zero(direction){
            direction = hit_record.normal;
        }
        
        Some(( self.albedo,  Ray{ origin: hit_record.p, direction }))
    }
}