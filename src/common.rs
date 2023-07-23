use cgmath::{Vector3, num_traits::clamp, InnerSpace};
use rand::Rng;

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
    let ir = (255.99 * clamp(color.x.sqrt(), 0.0, 0.999)) as u8;
    let ig = (255.99 * clamp(color.y.sqrt(), 0.0, 0.999)) as u8;
    let ib = (255.99 * clamp(color.z.sqrt(), 0.0, 0.999)) as u8;
    image::Rgb([ir, ig, ib])
}

/// Create a random vector withing iven paramters
pub fn rand_vec3(min: f32, max:f32) -> Vec3{
    let mut rng = rand::thread_rng();
    Vec3::new(
        rng.gen_range(min..max),
        rng.gen_range(min..max),
        rng.gen_range(min..max),
    )
}

/// Create a random vector within a unit spere
fn random_in_unit_sphere() -> Vec3{
    loop {
        let p = rand_vec3(-1.0, 1.0);
        if p.magnitude2() >= 1.0 {continue;}
        return p;
    }
}

/// Create a random unit vector
pub fn random_unit_vector() -> Vec3{
    random_in_unit_sphere().normalize()
}

/// Check if a vector is almost 0
pub fn near_zero(vec: Vec3) -> bool{
    let epsilon = 1e-8;

    (vec.x.abs() < epsilon) && (vec.y.abs() < epsilon) && (vec.z.abs() < epsilon)
}

/// Reflect a vector over a different vector
pub fn reflect(v: Vec3, n: Vec3) -> Vec3{
    v - 2.0*n.dot(v)*n
}
