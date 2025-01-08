use crate::{common::Point3};

/// A sphere object
pub struct Sphere{
    pub center: Point3,
    pub radius: f32,
    pub material: usize,
    pub material_type: u32,
}

impl Sphere{
    pub fn new(center: Point3, radius: f32, material: usize, material_type:u32) -> Self{
        Self{
            center,
            radius,
            material,
            material_type
        }
    }
}

impl Into<crate::compute_shader::Sphere> for Sphere{
    fn into(self) -> crate::compute_shader::Sphere {
        crate::compute_shader::Sphere{
            center: self.center.into(),
            radius: self.radius,
            material_index: self.material as u32,
            material_type: self.material_type,
            padding: [0.0, 0.0],
        }
    }
}

// impl Hittable for Sphere {
//     fn hit(&self, r: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
//         // quadratic setup
//         let oc = r.origin - self.center;
//         let a = r.direction.magnitude2();
//         let half_b = oc.dot(r.direction);
//         let c = oc.magnitude2() - self.radius*self.radius;

//         // check if there are real solutions
//         let discriminant = half_b*half_b - a*c;
//         if discriminant < 0.0 {
//             return None;
//         }
//         let sqrtd = discriminant.sqrt();

//         // find the nearest root in the acceptable range
//         let mut root = (-half_b - sqrtd) /a;
//         if root < t_min || root > t_max {
//             root = (-half_b + sqrtd) /a;
//             if root < t_min || root > t_max{
//                 return None;
//             }
//         }

//         // return hit record
//         let outward_normal = (r.at(root) - self.center) / self.radius;
//         Some(HitRecord::new(r.at(root), root, self.material, r, outward_normal))
//     }
// }
