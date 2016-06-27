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

    // let img = ChainedSource::img();
    // let tmp1 = ChainedSource::new(&sobel_x, vec![&img]);
    // let tmp2 = ChainedSource::new(&sobel_y, vec![&img]);
    // let tmp3 = ChainedSource::new(&grad,    vec![&tmp1, &tmp2]);

    // chained source interface
    // takes n chained source input streams, returns a single chained source input stream
    // the "chain" operator produces a new chained source (stream)
    // * how many inputs does the new stream take?
    //
    // need to look at some other stream manipulation libraries to get a good idea of how to model
    // this

    // any chained source can be used as an input to any other chained source
    // every chained source has $n$ inputs
    // perhaps use a macro
    // sobel = chain!(grad, sobel_x, sobel_y) // call grad with sobel_x and sobel_y
    //
    // splitAndMerge(n, [a, b, ..., ], merge) -> copies input to n other inputs, runs a, b, c, ...,
    // then merges the result from each intermediate process with the function merge

    // let sobel_full = splitAndMerge!(img, [sobel_x, sobel_y], grad);


    // still need to define a compilation strategy
    // * want to emit code which relies on the compiler to do much of the optimizations (no
    // schedules)
    // * should expose as much room for optimization as possible
    //
    // * should each function in a chain get compiled into a function taking an input and returning
    // some pointer to a new buffer? For a function with dependencies, these could all be
    // precomputed. Would such a thing get inlined heavily?
    //  - yes, as long as you make sure to mark things as noalias.
    //  - This might be the simplest way to get something working
    //  - The jitfunction will set up the order of operations
    //
    // * should each lookup get compiled to a function call? No precomputation, but lots of
    // recomputation

    println!("reading image");
    let inpt = image::open(&Path::new("in1.png")).unwrap();

    let out1 = sobel_x.run_on_image_inputs(&[inpt.to_luma()]);
    let ref mut fout = File::create(&Path::new("out1.png")).unwrap();
    let _ = image::ImageLuma8(out1).save(fout, image::PNG);

    let out2 = sobel_y.run_on_image_inputs(&[inpt.to_luma()]);
    let ref mut fout = File::create(&Path::new("out2.png")).unwrap();
    let _ = image::ImageLuma8(out2).save(fout, image::PNG);
}
