use crate::{common::{Point3, Vec3}, ray::Ray};
use cgmath::Angle;

pub struct Camera{
    origin: Point3,
    lower_left_corner: Point3,
    horizontal: Vec3,
    vertical: Vec3
}

impl Camera {
    pub fn new(vfov: cgmath::Deg<f32>,aspect_ratio: f32) -> Self {
        //fov
        let h = (vfov/2.0).tan();
        // properties
        let viewport_height = 2.0 * h;
        let viewport_width = aspect_ratio * viewport_height;
        let focal_length = 1.0;
        // viewport dimentions
        let horizontal = Vec3::new(viewport_width, 0.0, 0.0);
        let vertical = Vec3::new(0.0, viewport_height, 0.0);
        let origin = Vec3::new(0.0, 0.0, 0.0);
        // construct camera
        Self {
            origin,
            lower_left_corner: origin- horizontal/2.0 - vertical/2.0 - Vec3::new(0.0, 0.0, focal_length),
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