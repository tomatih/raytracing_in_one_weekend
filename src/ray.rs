use crate::common::{Point3, Vec3};

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