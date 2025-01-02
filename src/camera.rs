use crate::common::{Point3, Vec3};
use crate::compute_shader;
use cgmath::{Angle, InnerSpace};

pub struct Camera {
    origin: Point3,
    lower_left_corner: Point3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    w: Vec3,
    lens_radius: f32,
}

impl Camera {
    pub fn new(look_from: Point3, look_at: Point3, up: Vec3, vfov: cgmath::Deg<f32>, aspect_ratio: f32, apeture: f32, focus_distance: f32) -> Self {
        //fov
        let h = (vfov / 2.0).tan();
        // properties
        let viewport_height = 2.0 * h;
        let viewport_width = aspect_ratio * viewport_height;
        // unit vectors
        let w = (look_from - look_at).normalize();
        let u = up.cross(w).normalize();
        let v = w.cross(u);
        // viewport dimentions
        let horizontal = focus_distance * viewport_width * u;
        let vertical = focus_distance * viewport_height * v;
        let origin = look_from;
        // construct camera
        Self {
            origin,
            lower_left_corner:origin - horizontal / 2.0 - vertical / 2.0 - focus_distance * w,
            horizontal,
            vertical,
            u,
            v,
            w,
            lens_radius: apeture / 2.0,
        }
    }

    pub fn to_push_constant(self) -> compute_shader::PushConstantData {
        compute_shader::PushConstantData{
            origin: Into::<[f32;3]>::into(self.origin).into(),
            lower_left_corner: Into::<[f32;3]>::into(self.lower_left_corner).into(),
            horizontal: Into::<[f32;3]>::into(self.horizontal).into(),
            vertical: Into::<[f32;3]>::into(self.vertical).into(),
            u: Into::<[f32;3]>::into(self.u).into(),
            v: Into::<[f32;3]>::into(self.v).into(),
            w: Into::<[f32;3]>::into(self.w).into(),
            lens_radius: self.lens_radius,
        }
    }

    // pub fn get_ray(&self, s: f32, t:f32) -> Ray{
    //     let rd = self.lens_radius * random_in_unit_sphere();
    //     let offset = self.u * rd.x + self.v * rd.y;
    //
    //     Ray {
    //         origin: self.origin + offset,
    //         direction: self.lower_left_corner +s*self.horizontal + t*self.vertical - self.origin - offset
    //     }
    // }
}