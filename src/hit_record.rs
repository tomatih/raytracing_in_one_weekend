use cgmath::InnerSpace;

use crate::{Vec3, Point3, Ray};

/// Data of the last hit by a ray
pub struct HitRecord{
    pub p: Point3,
    pub normal: Vec3,
    pub t: f32,
    pub front_face: bool
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
