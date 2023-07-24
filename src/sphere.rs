use std::rc::Rc;
use cgmath::InnerSpace;

use crate::{common::Point3, hit_record::HitRecord, ray::Ray, hittable::Hittable, material::Material};

/// A sphere object
pub struct Sphere{
    pub center: Point3,
    pub radius: f32,
    pub material: Rc<dyn Material>
}

impl Hittable for Sphere {
    fn hit(&self, r: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        // quadratic setup
        let oc = r.origin - self.center;
        let a = r.direction.magnitude2();
        let half_b = oc.dot(r.direction);
        let c = oc.magnitude2() - self.radius*self.radius;

        // check if there are real solutions
        let discriminant = half_b*half_b - a*c;
        if discriminant < 0.0 {
            return None;
        }
        let sqrtd = discriminant.sqrt();

        // find the nearest root in the acceptable range
        let mut root = (-half_b - sqrtd) /a;
        if root < t_min || root > t_max {
            root = (-half_b + sqrtd) /a;
            if root < t_min || root > t_max{
                return None;
            }
        }

        // update hit record
        //(*hit_record).t =root;
        //(*hit_record).p = r.at(root);
        let outward_normal = (r.at(root) - self.center) / self.radius;
        //(*hit_record).set_face_normal(r, outward_normal);
        //(*hit_record).mat_ptr = self.material;
        Some(HitRecord::new(r.at(root), root, Rc::clone(&self.material), r, outward_normal))
    }
}
