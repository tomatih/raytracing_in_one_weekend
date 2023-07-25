use crate::{ray::Ray, common::Color, hit_system::HitRecord};

/// Allows materials to scatter light
pub trait Material {
    fn scatter(&self, ray_in: Ray, hit_record: &HitRecord) -> Option<(Color, Ray)>;
}