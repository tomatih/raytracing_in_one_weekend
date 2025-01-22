use cgmath::Vector4;

use crate::common::Color;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Material {
    Dielectric { ir: f32 },
    Lambertian { albedo: Color },
    Metal { albedo: Color, fuzziness: f32 },
    TrueBlack,
}

impl From<Material> for Vector4<f32> {
    fn from(val: Material) -> Self {
        match val {
            Material::Dielectric { ir } => [ir, 0.0, 0.0, 0.0],
            Material::Lambertian { albedo } => [albedo.x, albedo.y, albedo.z, 0.0],
            Material::Metal { albedo, fuzziness } => [albedo.x, albedo.y, albedo.z, fuzziness],
            Material::TrueBlack => [0.0, 0.0, 0.0, 0.0],
        }
        .into()
    }
}

impl Material {
    pub fn get_type(&self) -> u32 {
        match self {
            Material::Lambertian { .. } => 0,
            Material::Dielectric { .. } => 1,
            Material::Metal { .. } => 2,
            Material::TrueBlack => 3,
        }
    }
}
