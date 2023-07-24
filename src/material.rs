use crate::{ray::Ray, hit_record::HitRecord, common::Color};

/// Allows materials to scatter light
pub trait Material {
    fn scatter(&self, ray_in: Ray, hit_record: &HitRecord) -> Option<(Color, Ray)>;
}