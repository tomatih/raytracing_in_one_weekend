use image::{RgbImage, ImageBuffer};
use cgmath::{Vector3, InnerSpace, VectorSpace};

// intention aliases aliases
type Point3 = Vector3<f32>;
type Color = Vector3<f32>;

fn to_pixel(color: Color) -> image::Rgb<u8>{
    // convert values
    let ir = (255.99 * color.x) as u8;
    let ig = (255.99 * color.y) as u8;
    let ib = (255.99 * color.z) as u8;
    image::Rgb([ir, ig, ib])
}

struct Ray{
    origin: Point3,
    direction: Vector3<f32>
}

impl Ray {
    fn at(&self, t: f32) -> Point3{
        self.origin + t*self.direction
    }
}

struct HitRecord{
    p: Point3,
    normal: Vector3<f32>,
    t: f32
}

trait Hittable {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32, hit_record: &mut HitRecord) -> bool;
}

struct Sphere{
    center: Point3,
    radius: f32
}

impl Hittable for Sphere {
    fn hit(&self, r: &Ray, t_min: f32, t_max: f32, hit_record: &mut HitRecord) -> bool {
        let oc = r.origin - self.center;
        let a = r.direction.magnitude2();
        let half_b = oc.dot(r.direction);
        let c = oc.magnitude2() - self.radius*self.radius;

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

        (*hit_record).t =root;
        (*hit_record).p = r.at(root);
        (*hit_record).normal = (hit_record.p - self.center) / self.radius;
        true
    }
}

fn hit_sphere(center: Point3, radius: f32, r: &Ray) -> f32{
    let oc = r.origin - center;
    let a = r.direction.magnitude2();
    let half_b = oc.dot(r.direction);
    let c = oc.magnitude2() - radius*radius;
    let discriminant = half_b*half_b - a*c;
    if discriminant < 0.0 {
        -1.0
    } else {
        (-half_b - discriminant.sqrt()) / a
    }
}

fn ray_color(ray: &Ray) -> Color {
    let t = hit_sphere(Point3::new(0.0, 0.0, -1.0), 0.5, ray);
    if t > 0.0 {
        let n = (ray.at(t) - Vector3::new(0.0, 0.0, -1.0)).normalize();
        return 0.5 * Color::new(n.x+1.0, n.y+1.0, n.z+1.0);
    }
    let unit_direction = ray.direction.normalize();
    let t = 0.5*(unit_direction.y + 1.0);
    Color::new(1.0, 1.0, 1.0).lerp(Color::new(0.5, 0.7, 1.0), t)
}

fn main() {
    // Image constants
    const ASPECT_RATIO: f32 = 16.0 / 9.0;
    const IMAGE_WIDTH: u32 = 400;
    const IMAGE_HEIGHT: u32 = (IMAGE_WIDTH as f32 /ASPECT_RATIO) as u32;

    // camera
    let viewport_height = 2.0;
    let viewport_width = ASPECT_RATIO * viewport_height;
    let focal_length = 1.0;

    let origin = Vector3::new(0.0, 0.0, 0.0);
    let horizontal = Vector3::new(viewport_width, 0.0, 0.0);
    let vertical = Vector3::new(0.0, viewport_height, 0.0);
    let lower_left_corner = origin - horizontal/2.0 - vertical/2.0 - Vector3::new(0.0, 0.0, focal_length);

    // Allocate image buffer
    let mut img: RgbImage = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);

    // Render image
    for j in (0..IMAGE_HEIGHT).rev() {
        print!("\rScanlines remaining {:3}", j);
        for i in 0..IMAGE_WIDTH{
            // create color
            let u = i as f32 / (IMAGE_WIDTH - 1) as f32;
            let v = j as f32 / (IMAGE_HEIGHT - 1) as f32;
            let r = Ray{ origin,
                direction: lower_left_corner+ u*horizontal + v*vertical - origin
            };
            let color = ray_color(&r);
            // print pixel
            img.put_pixel(i, IMAGE_HEIGHT-j-1, to_pixel(color));
        }
    }
    println!("");
    // save image
    img.save("out.png").expect("Faild to save image");
}
