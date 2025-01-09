use cgmath::{Vector3, InnerSpace};

// intention aliases aliases
/// Vector used for the raytracer
pub type Vec3 = Vector3<f32>;
/// Point in 3D space
pub type Point3 = Vec3;
/// RGB represenation of colour 
pub type Color = Vec3;



/// Reflect a vector over a different vector
pub fn reflect(v: Vec3, n: Vec3) -> Vec3{
    v - 2.0*n.dot(v)*n
}

/// Refract a vector
pub fn refract(uv: Vec3, n: Vec3, etai_over_etat: f32) -> Vec3{
    let cos_theta = n.dot(-uv).min(1.0);
    let r_out_perpendicular = etai_over_etat * (uv + cos_theta*n);
    let r_out_parallel = -(1.0 - r_out_perpendicular.magnitude2()).abs().sqrt() * n;
    r_out_parallel + r_out_perpendicular
}
