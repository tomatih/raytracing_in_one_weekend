use cgmath::InnerSpace;

use crate::{common::{Point3, Vec3}, ray::Ray};

/// Data of the last hit by a ray
pub struct HitRecord{
    pub p: Point3,
    pub normal: Vec3,
    pub t: f32,
    pub front_face: bool
}

impl Default for HitRecord {
    fn default() -> Self {
        Self {
            p: Point3::new(0.0, 0.0, 0.0),
            normal: Vec3::new(0.0, 0.0, 0.0),
            t: 0.0,
            front_face: false
        }
    }
}

impl HitRecord {
    /// Update front face and normal values
    pub fn set_face_normal(&mut self, r: &Ray, outward_normal: Vec3){
        self.front_face = r.direction.dot(outward_normal) < 0.0;
        self.normal =  if self.front_face {
            outward_normal
        }else{
            -outward_normal
        };
    }
}
