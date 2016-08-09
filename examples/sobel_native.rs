extern crate image;

use image::Luma;
use image::ImageBuffer;
use image::Pixel;

use std::fs;
use std::fs::File;
use std::path::Path;

use std::time::Instant;

fn at(img: &ImageBuffer<Luma<u8>, Vec<u8>>, x: i64, y: i64) -> i64 {
    if x >= img.dimensions().0 as i64 || x < 0 {
        return 0;
    }

    if y >= img.dimensions().1 as i64 || y < 0 {
        return 0;
    }

    let cs = img.get_pixel(x as u32, y as u32).channels();
    return cs[0] as i64;
}

fn main() {
    let sobel_x = [[-1, 0, 1],
                   [-2, 0, 2],
                   [-1, 0, 1]];

    let sobel_y = [[-1, -2, -1],
                   [ 0,  0,  0],
                   [ 1,  2,  1]];

    let paths = fs::read_dir("images").unwrap();

    for path in paths {
        let path = path.unwrap().path();

        let img = image::open(&path).unwrap().to_luma();

        let dim = img.dimensions();
        let mut out: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(dim.0, dim.1);

        let now = Instant::now();

        for x in 0..(dim.0 as i64) {
            for y in 0..(dim.1 as i64) {

                let pixel_x =
                    (sobel_x[0][0] * at(&img,x-1,y-1)) + (sobel_x[0][1] * at(&img,x,y-1)) + (sobel_x[0][2] * at(&img,x+1,y-1)) +
                    (sobel_x[1][0] * at(&img,x-1,y))   + (sobel_x[1][1] * at(&img,x,y))   + (sobel_x[1][2] * at(&img,x+1,y)) +
                    (sobel_x[2][0] * at(&img,x-1,y+1)) + (sobel_x[2][1] * at(&img,x,y+1)) + (sobel_x[2][2] * at(&img,x+1,y+1));

                let pixel_y =
                    (sobel_y[0][0] * at(&img,x-1,y-1)) + (sobel_y[0][1] * at(&img,x,y-1)) + (sobel_y[0][2] * at(&img,x+1,y-1)) +
                    (sobel_y[1][0] * at(&img,x-1,y))   + (sobel_y[1][1] * at(&img,x,y))   + (sobel_y[1][2] * at(&img,x+1,y)) +
                    (sobel_y[2][0] * at(&img,x-1,y+1)) + (sobel_y[2][1] * at(&img,x,y+1)) + (sobel_y[2][2] * at(&img,x+1,y+1));

                let grad = (((pixel_x * pixel_x) + (pixel_y * pixel_y)) as f64).sqrt() as u8;

                let mut outpxcs = out.get_pixel_mut(x as u32, y as u32).channels_mut();
                outpxcs[0] = grad;
            }
        }

        let elapsed = now.elapsed();
        println!("{:?}\t{}",
                 path.file_name().unwrap(),
                 elapsed.as_secs()*(10e9 as u64) + elapsed.subsec_nanos() as u64);

        let op = Path::new("out_rust").join(path.file_name().unwrap());
        let ref mut fout = File::create(&op).unwrap();
        let _ = image::ImageLuma8(out).save(fout, image::PNG);
    }
}
