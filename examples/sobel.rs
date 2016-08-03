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

        Box::new(SqrtExpr::new(t1 + t2)) // required to appease type system
    });

    // create a unique image source
    // each one of these will represent a different input to the final compiled function
    // TODO simplify interface, figure out input ids and whatnot
    // let image = ChainLink::create_image_source();
    let image = ChainLink::ImageSource(0);

    let c1 = ChainLink::link(vec![&image], &sobel_x);
    let c2 = ChainLink::link(vec![&image], &sobel_y);
    let c3 = ChainLink::link(vec![&c1, &c2], &grad);

    // compile the function
    let cc = c3.compile();

    let luma = image::open(&Path::new("in1.png")).unwrap().to_luma();
    let out = cc.run_on(&[&luma]);

    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    let _ = image::ImageLuma8(out).save(fout, image::PNG);
}
