use cgmath::InnerSpace;

use crate::{Point3, Ray, HitRecord, Hittable};

/// A sphere object
pub struct Sphere{
    pub center: Point3,
    pub radius: f32
}

impl Hittable for Sphere {
    fn hit(&self, r: &Ray, t_min: f32, t_max: f32, hit_record: &mut HitRecord) -> bool {
        // quadratic setup
        let oc = r.origin - self.center;
        let a = r.direction.magnitude2();
        let half_b = oc.dot(r.direction);
        let c = oc.magnitude2() - self.radius*self.radius;

        // check if there are real solutions
        let discriminant = half_b*half_b - a*c;
        if discriminant < 0.0 {
            return false;
        }
        let sqrtd = discriminant.sqrt();

        // find the nearest root in the acceptable range
        let mut root = (-half_b - sqrtd) /a;
        if root < t_min || root > t_max {
            root = (-half_b + sqrtd) /a;
            if root < t_min || root > t_max{
                return false;
            }
        }

        // update hit record
        (*hit_record).t =root;
        (*hit_record).p = r.at(root);
        let outward_normal = (hit_record.p - self.center) / self.radius;
        (*hit_record).set_face_normal(r, outward_normal);
        true
    }
}