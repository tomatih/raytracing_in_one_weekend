use crate::{Ray, HitRecord};

/// Allows rays to hit an object
pub trait Hittable {
    /// Returns whether the given ray will hit the object withing given bounds and updates the hit record accordingly
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32, hit_record: &mut HitRecord) -> bool;
}