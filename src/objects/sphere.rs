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

impl From<Sphere> for ray_trace_shader::Sphere{
    fn from(value: Sphere) -> Self {
        ray_trace_shader::Sphere {
            center: value.center,
            radius: value.radius,
            material_index: value.material as u32,
            material_type: value.material_type,
            padding: [0.0, 0.0].into(),
        }    }
}
