extern crate image;
extern crate rust_img;

use std::fs::File;
use std::path::Path;

use rust_img::*;

fn main() {
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

        Box::new(SqrtExpr::new(t1 + t2))
    });

    println!("reading image");
    let input = image::open(&Path::new("in1.png")).unwrap();

    println!("constructing chain");
    let luma = input.to_luma();
    let image = ChainLink::image_source(&luma);

    let c1 = ChainLink::link(vec![&image], &sobel_x);
    let c2 = ChainLink::link(vec![&image], &sobel_y);
    let c3 = ChainLink::link(vec![&c1, &c2], &grad);

    println!("compiling module");
    let m = c3.compile();
    println!("{:?}", m);
}
