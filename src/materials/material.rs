use crate::common::Color;

pub enum Material {
    Dielectric { ir: f32 },
    Lambertian { albedo: Color },
    Metal { albedo: Color, fuzziness: f32 },
    TrueBlack,
}

impl Into<[f32; 4]> for Material {
    fn into(self) -> [f32; 4] {
        match self {
            Material::Dielectric { ir } => [ir, 0.0, 0.0, 0.0],
            Material::Lambertian { albedo } => [albedo.x, albedo.y, albedo.z, 0.0],
            Material::Metal { albedo, fuzziness } => [albedo.x, albedo.y, albedo.z, fuzziness],
            Material::TrueBlack => [0.0, 0.0, 0.0, 0.0],
        }
    }
}
