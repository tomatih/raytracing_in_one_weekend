use crate::{common::Point3, ray_trace_shader};

/// A sphere object
pub struct Sphere {
    pub center: Point3,
    pub radius: f32,
    pub material: usize,
    pub material_type: u32,
}

impl Sphere {
    pub fn new(center: Point3, radius: f32, material: usize, material_type: u32) -> Self {
        Self {
            center,
            radius,
            material,
            material_type,
        }
    }
}

impl Into<ray_trace_shader::Sphere> for Sphere {
    fn into(self) -> ray_trace_shader::Sphere {
        ray_trace_shader::Sphere {
            center: self.center,
            radius: self.radius,
            material_index: self.material as u32,
            material_type: self.material_type,
            padding: [0.0, 0.0].into(),
        }
    }
}
