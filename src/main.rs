use image::{RgbImage, ImageBuffer};

fn main() {
    // Image constants
    const IMAGE_HEIGHT: u32 = 256;
    const IMAGE_WIDTH: u32 = 256;

    // Allocate image buffer
    let mut img: RgbImage = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);

    // Render image
    for j in 1..=IMAGE_HEIGHT {
        for i in 0..IMAGE_WIDTH{
            // normalised values
            let r = i as f32 / (IMAGE_WIDTH-1) as f32;
            let g = j as f32 / (IMAGE_HEIGHT-1) as f32;
            let b = 0.25;
            // pixel values
            let ir = (255.99 * r) as u8;
            let ig = (255.99 * g) as u8;
            let ib = (255.99 * b) as u8;
            // print pixel
            img.put_pixel(i, IMAGE_HEIGHT-j, image::Rgb([ir, ig, ib]))
        }
    }
    // save image
    img.save("out.png").expect("Faild to save image");
}
