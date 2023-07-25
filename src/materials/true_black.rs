use crate::{hit_system::HitRecord, ray::Ray, common::Vec3};

use super::Material;

pub struct TrueBlack;

impl Material for TrueBlack {
    fn scatter(&self, _ray_in: Ray, _hit_record: &HitRecord) -> Option<(Vec3, Ray)> {
        None
    }
}