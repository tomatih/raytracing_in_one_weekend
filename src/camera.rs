use crate::{common::{Point3, Vec3}, ray::Ray};
use cgmath::{Angle, InnerSpace};

pub struct Camera{
    origin: Point3,
    lower_left_corner: Point3,
    horizontal: Vec3,
    vertical: Vec3
}

impl Camera {
    pub fn new(look_from: Point3, look_at: Point3, up: Vec3,vfov: cgmath::Deg<f32>,aspect_ratio: f32) -> Self {
        //fov
        let h = (vfov/2.0).tan();
        // properties
        let viewport_height = 2.0 * h;
        let viewport_width = aspect_ratio * viewport_height;
        // unit vectors
        let w = (look_from- look_at).normalize();
        let u = up.cross(w).normalize();
        let v = w.cross(u);
        // viewport dimentions
        let horizontal = viewport_width * u;
        let vertical = viewport_height * v;
        let origin = look_from;
        // construct camera
        Self {
            origin,
            lower_left_corner: origin- horizontal/2.0 - vertical/2.0 - w,
            horizontal,
            vertical,
        }
    }
    
    pub fn get_ray(&self, u: f32, v:f32) -> Ray{
        Ray {
            origin: self.origin,
            direction: self.lower_left_corner +u*self.horizontal + v*self.vertical - self.origin
        }
    }
}