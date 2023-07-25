use std::rc::Rc;

use cgmath::InnerSpace;

use crate::{common::{Point3, Vec3}, ray::Ray, materials::Material};

/// Data of the last hit by a ray
pub struct HitRecord{
    pub p: Point3,
    pub normal: Vec3,
    pub t: f32,
    pub mat_ptr: Rc<dyn Material>,
    pub front_face: bool
}

impl HitRecord {
    pub fn new(p: Point3, t: f32, mat_ptr: Rc<dyn Material>, r: &Ray, outward_normal: Vec3) -> Self {
        let front_face = r.direction.dot(outward_normal) < 0.0;
        let normal =  if front_face {
            outward_normal
        }else{
            -outward_normal
        };
        Self { p, normal, t, mat_ptr, front_face }
    }
}
