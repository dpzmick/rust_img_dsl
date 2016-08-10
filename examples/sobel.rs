extern crate rust_img;
extern crate image;

use rust_img::*;

use std::fs;
use std::fs::File;
use std::path::Path;

use std::time::Instant;

fn main() {
    let now = Instant::now();

    let sobel_x = [[-1, 0, 1],
                   [-2, 0, 2],
                   [-1, 0, 1]];

    let sobel_y = [[-1, -2, -1],
                   [ 0,  0,  0],
                   [ 1,  2,  1]];

    let sobel_x = Function::gen_3x3_kernel(sobel_x);
    let sobel_y = Function::gen_3x3_kernel(sobel_y);

    let grad = Function::new(2, |x, y, inputs| {
        let input0 = &inputs[0];
        let input1 = &inputs[1];

        let t1 = input0(x(), y()) * input0(x(), y());
        let t2 = input1(x(), y()) * input1(x(), y());

        Box::new(SqrtExpr::new(t1 + t2)) // required to appease type system
    });

    // create a unique image source
    // this image source will be index 0 in the arguments array passed to run_on
    let image = ChainLink::ImageSource(0);

    let c1 = ChainLink::link(vec![&image], &sobel_x);
    let c2 = ChainLink::link(vec![&image], &sobel_y);
    let c3 = ChainLink::link(vec![&c1, &c2], &grad);

    let elapsed = now.elapsed();
    println!("construction\t{}", elapsed.as_secs()*(10e9 as u64) + elapsed.subsec_nanos() as u64);

    let now = Instant::now();

    // compile the function
    let cc = c3.compile();

    let elapsed = now.elapsed();
    println!("compilation\t{}", elapsed.as_secs()*(10e9 as u64) + elapsed.subsec_nanos() as u64);

    let paths = fs::read_dir("images").unwrap();
    for path in paths {
        let path = path.unwrap().path();
        let luma = image::open(&path).unwrap().to_luma();

        let now = Instant::now();
        let out = cc.run_on(&[&luma]);
        let elapsed = now.elapsed();

        println!("{:?}\t{}",
                 path.file_name().unwrap(),
                 elapsed.as_secs()*(10e9 as u64) + elapsed.subsec_nanos() as u64);

        let op = Path::new("out_jit").join(path.file_name().unwrap());
        let ref mut fout = File::create(&op).unwrap();
        let _ = image::ImageLuma8(out).save(fout, image::PNG);
    }
}
