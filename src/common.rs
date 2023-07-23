use cgmath::Vector3;

// intention aliases aliases
/// Vector used for the raytracer
pub type Vec3 = Vector3<f32>;
/// Point in 3D space
pub type Point3 = Vec3;
/// RGB represenation of colour 
pub type Color = Vec3;

/// Convert color value into a writable pixel
pub fn to_pixel(color: Color) -> image::Rgb<u8>{
    // convert values
    let ir = (255.99 * color.x) as u8;
    let ig = (255.99 * color.y) as u8;
    let ib = (255.99 * color.z) as u8;
    image::Rgb([ir, ig, ib])
}