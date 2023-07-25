mod material;
mod metal;
mod lambertian;
mod true_black;
mod dielectric;

pub use metal::Metal;
pub use lambertian::Lambertian;
pub use material::Material;
pub use true_black::TrueBlack;
pub use dielectric::Dielectric;