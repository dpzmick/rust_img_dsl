extern crate image;
extern crate num;

use std::fs::File;
use std::path::Path;

use image::GenericImage;
use image::Pixel;
use image::Luma;
use image::Rgba;

use image::math::utils::clamp;

use num::num_traits::NumCast;

// every function is a function (u32, u32) -> u8

enum Expression {
    ConstExpr(u8),
    AddExpr(Box<Expression>, Box<Expression>)
}

fn luma<T: Pixel>(input: T) -> Luma<T::Subpixel> {
    input.to_luma()
}

fn sx<T: Pixel + 'static>(input: &[[T; 3]; 3]) -> Luma<T::Subpixel> {
    let sobel_x = [[-1, 0, 1],
                   [-2, 0, 2],
                   [-1, 0, 1]];

    let mut out: i32 = 0;

    for i in 0..3 {
        for j in 0..3 {
            let px_val: i32 = NumCast::from(luma(input[i][j]).channels4().0).unwrap();
            out += sobel_x[i][j] * px_val;
        }
    }

    Luma::from_channels(
        NumCast::from(clamp(out, 0, 255)).unwrap(),
        NumCast::from(0).unwrap(),
        NumCast::from(0).unwrap(),
        NumCast::from(0).unwrap())
}

fn sy<T: Pixel + 'static>(input: &[[T; 3]; 3]) -> Luma<T::Subpixel> {
    let sobel_y = [[-1, -2, -1],
                   [ 0,  0,  0],
                   [ 1,  2,  1]];

    let mut out: i32 = 0;

    for i in 0..3 {
        for j in 0..3 {
            let px_val: i32 = NumCast::from(luma(input[i][j]).channels4().0).unwrap();
            out += sobel_y[i][j] * px_val;
        }
    }

    Luma::from_channels(
        NumCast::from(clamp(out, 0, 255)).unwrap(),
        NumCast::from(0).unwrap(),
        NumCast::from(0).unwrap(),
        NumCast::from(0).unwrap())
}

fn g(a: Luma<u8>, b: Luma<u8>) -> Luma<u8> {
    let aa: f64 = NumCast::from(a.channels4().0).unwrap();
    let bb: f64 = NumCast::from(b.channels4().0).unwrap();
    let out = (((aa * aa) + (bb * bb))).sqrt() as i32;
    Luma::from_channels(
        NumCast::from(clamp(out, 0, 255)).unwrap(),
        NumCast::from(0).unwrap(),
        NumCast::from(0).unwrap(),
        NumCast::from(0).unwrap())
}

fn main() {
    let inpt = image::open(&Path::new("in1.png")).unwrap();
    let mut sobel_out = image::ImageBuffer::new(inpt.width(), inpt.height());

    // compute x sobel on luma image
    for x in 0..(inpt.width()) as i32 {
        for y in 0..(inpt.height()) as i32 {
            // get the local region
            let mut region = [[Rgba::from_channels(0,0,0,0); 3]; 3];

            for i in -1..2 as i32 {
                for j in -1..2 as i32 {
                    let x: i32 = x + i;
                    let y: i32 = y + j;

                    if     (x + i) >= 0
                        && (y + j) >= 0
                        && (x + i) < (inpt.width() as i32)
                        && (y + j) < (inpt.height() as i32)
                    {
                        // region is in range
                        let x = (x + i) as u32;
                        let y = (y + j) as u32;
                        region[(i+1) as usize][(j+1) as usize] = inpt.get_pixel(x, y);
                    }
                }
            }

            let p = g(sx(&region), sy(&region));
            sobel_out.put_pixel(x as u32, y as u32, p);
        }
    }

    // Write the contents of this image to the Writer in PNG format.
    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    let _ = image::ImageLuma8(sobel_out).save(fout, image::PNG);
}
