fn main() {
    // Image constants
    const IMAGE_HEIGHT: i32 = 256;
    const IMAGE_WIDTH: i32 = 256;

    // Print image header
    println!("P3");
    println!("{} {}", IMAGE_WIDTH, IMAGE_HEIGHT);
    println!("255");

    // Render image
    for j in (0..IMAGE_HEIGHT).rev() {
        for i in 0..IMAGE_WIDTH{
            // normalised values
            let r = i as f32 / (IMAGE_WIDTH-1) as f32;
            let g = j as f32 / (IMAGE_HEIGHT-1) as f32;
            let b = 0.25;
            // pixel values
            let ir = (255.99 * r) as i32;
            let ig = (255.99 * g) as i32;
            let ib = (255.99 * b) as i32;
            // print pixel
            println!("{} {} {}", ir, ig, ib);
        }
    }
}
