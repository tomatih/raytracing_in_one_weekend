use image::{RgbImage, ImageBuffer};
use cgmath::Vector3;

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

fn main() {
    // Image constants
    const IMAGE_HEIGHT: u32 = 256;
    const IMAGE_WIDTH: u32 = 256;

    // Allocate image buffer
    let mut img: RgbImage = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);

    // Render image
    for j in (0..IMAGE_HEIGHT).rev() {
        print!("\rScanlines remaining {:3}", j);
        for i in 0..IMAGE_WIDTH{
            // create color
            let color = Color::new(
                i as f32 / (IMAGE_WIDTH-1) as f32,
                j as f32 / (IMAGE_HEIGHT-1) as f32,
                0.25
            );
            // print pixel
            img.put_pixel(i, IMAGE_HEIGHT-j-1, to_pixel(color));
        }
    }
    println!("");
    // save image
    img.save("out.png").expect("Faild to save image");
}
