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
    fn at(self, t: f32) -> Point3{
        self.origin + t*self.direction
    }
}

fn hit_sphere(center: Point3, radius: f32, r: &Ray) -> bool{
    let oc = r.origin - center;
    let a = InnerSpace::dot(r.direction, r.direction);
    let b = 2.0 * InnerSpace::dot(oc, r.direction);
    let c = InnerSpace::dot(oc, oc) - radius*radius;
    let discriminant = b*b - 4.0*a*c;
    discriminant > 0.0
}

fn ray_color(ray: &Ray) -> Color {
    if hit_sphere(Point3::new(0.0, 0.0, -1.0), 0.5, ray){
        return Color::new(1.0, 0.0, 0.0);
    }
    let unit_direction = InnerSpace::normalize(ray.direction);
    let t = 0.5*(unit_direction.y + 1.0);
    //(1.0-t)*Color::new(1.0, 1.0, 1.0) + t*Color::new(0.5, 0.7, 1.0)
    VectorSpace::lerp(Color::new(1.0, 1.0, 1.0), Color::new(0.5, 0.7, 1.0), t)
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
