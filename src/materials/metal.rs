use cgmath::InnerSpace;
use crate::{common::{Color, reflect, random_in_unit_sphere}, ray::Ray, hit_system::HitRecord};

use super::Material;

pub struct Metal{
    pub albedo: Color,
    fuzziness: f32
}

impl Metal {
    pub fn new(albedo: Color, fuzziness: f32) -> Self {
        Self { albedo, fuzziness: if fuzziness < 1.0 {
            fuzziness
        } else{
            1.0
        } }
    }
}

impl Material for Metal {
    fn scatter(&self, ray_in: Ray, hit_record: &HitRecord) -> Option<(Color, Ray)> {
        // calculate scattered ray
        let reflected = reflect(ray_in.direction.normalize(), hit_record.normal);
        let scattered = Ray{ origin: hit_record.p, direction: reflected + self.fuzziness * random_in_unit_sphere() };
        
        // if ray pointed outward
        if scattered.direction.dot(hit_record.normal) > 0.0 {
            Some((self.albedo, scattered ))
        }
        else {
            None
        }
    }
}