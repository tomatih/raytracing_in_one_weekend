use cgmath::InnerSpace;

use crate::{
    common::{Point3, Vec3},
    ray::Ray,
};

/// Data of the last hit by a ray
pub struct HitRecord {
    pub p: Point3,
    pub normal: Vec3,
    pub t: f32,
    pub mat_index: usize,
    pub front_face: bool,
}

impl HitRecord {
    pub fn new(p: Point3, t: f32, mat_ptr: usize, r: &Ray, outward_normal: Vec3) -> Self {
        let front_face = r.direction.dot(outward_normal) < 0.0;
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };
        Self {
            p,
            normal,
            t,
            mat_index: mat_ptr,
            front_face,
        }
    }
}
