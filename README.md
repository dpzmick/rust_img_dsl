An experimental jit-compiled dsl embedded within rust which can be used for image processing, inspired, but nowhere near as cool, as the [halide](halide-lang.org) project.

# Motivation
This project was done so that I could learn rust by doing something interesting that would force me to get my hands a bit dirty.
It is not intended to be production code (this objective shows its ugly head in a few places).

I have certainly learned quite a bit about rust by doing this project.

# Example

## Define a sobel operator
```
    // define all the function needed for a sobel operator
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
```

What exists in `grad` after executing this code is essentially an AST for the grad function.
The same applies for `sobel_x` and `sobel_y`.
To hook them all together, you create a function chain.

```
    let image = ChainLink::create_image_source();

    let c1 = ChainLink::link(vec![&image], &sobel_x);
    let c2 = ChainLink::link(vec![&image], &sobel_y);
    let c3 = ChainLink::link(vec![&c1, &c2], &grad);

    // compile the function
    let cc = c3.compile();

    let luma = // get luma image from somewhere
    let out = cc.run_on(&[&luma]);
```

The chaining interface allows functions to be used multiple times, in multiple ways.
When a function chain is compiled, the entire chain is compiled into an llvm module.
This module is optimized by the llvm optimizer.
The `run_on` function for a compiled chain executes the optimized code
For every image source used in the chain, an image must be provided to the jit compiled function.
