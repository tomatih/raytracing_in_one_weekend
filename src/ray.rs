use crate::common::{Point3, Vec3};
use crate::compute_shader;

/// Simulated ray of light
pub struct Ray{
    pub origin: Point3,
    pub direction: Vec3
}

impl Ray {
    /// Returns point hit by a ray after t steps
    pub fn at(&self, t: f32) -> Point3{
        self.origin + t*self.direction
    }
}

impl Into<compute_shader::Ray> for Ray{
    fn into(self) -> compute_shader::Ray { 
        compute_shader::Ray{
            origin: self.origin.into(),
            padding1: 0.0f32,
            direction: self.direction.into(),
            padding2: 0.0f32,
        }
    }
}