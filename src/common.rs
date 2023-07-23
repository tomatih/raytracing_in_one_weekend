use cgmath::{Vector3, num_traits::clamp};

// intention aliases aliases
/// Vector used for the raytracer
pub type Vec3 = Vector3<f32>;
/// Point in 3D space
pub type Point3 = Vec3;
/// RGB represenation of colour 
pub type Color = Vec3;

/// Convert color value into a writable pixel
pub fn to_pixel(color: Color, samples_per_pixel: i32) -> image::Rgb<u8>{
    // scale color
    let color = color / samples_per_pixel as f32;

    // convert values
    let ir = (255.99 * clamp(color.x, 0.0, 0.999)) as u8;
    let ig = (255.99 * clamp(color.y, 0.0, 0.999)) as u8;
    let ib = (255.99 * clamp(color.z, 0.0, 0.999)) as u8;
    image::Rgb([ir, ig, ib])
}