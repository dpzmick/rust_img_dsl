use image::ImageBuffer;
use image::Luma;

use llvm;

use llvm::Compile;
use llvm::ExecutionEngine;

use expression::*;
use source::*;

#[derive(Debug)]
pub struct Function<'a> {
    num_inputs: usize,
    e:          Box<Expr + 'a>
}

impl<'a> Function<'a> {
    // wow
    pub fn new<F>(num_inputs: usize, gen: F) -> Self
        where F: Fn(
                  &Fn() -> Box<Expr + 'a>,
                  &Fn() -> Box<Expr + 'a>,
                  Vec<Box<Fn(Box<Expr + 'a>, Box<Expr + 'a>) -> Box<Expr + 'a>>>)
                  -> Box<Expr + 'a>
    {
        // create two functions which can be used to get a new boxed VarRef
        // we need this because all of the operators consume the boxes the operate on and its not a
        // good idea to try to implement Copy for a generic Box<Expr>, so each usage of a reference
        // to X or Y must exist inside of its own box
        let x = | | Box::new(VarRef::new(Var::X)) as Box<Expr + 'a>;
        let y = | | Box::new(VarRef::new(Var::Y)) as Box<Expr + 'a>;

        let mut vec = Vec::new();
        for i in 0..num_inputs {
            let input: Box<Fn(Box<Expr + 'a>, Box<Expr + 'a>) -> Box<Expr + 'a>>
                = Box::new(move |x, y| (Box::new(InputExpr::new(i, x, y)) as Box<Expr>));
            vec.push(input);
        }

        let e = gen(&x, &y, vec);

        Function { e: e, num_inputs: num_inputs }
    }

    pub fn gen_3x3_kernel(k: [[i64; 3]; 3]) -> Self {
        Function::new(1, |x, y, inputs| {
            let input = &inputs[0];

              (input(x() - 1, y() - 1) * k[0][0])
            + (input(x() - 1, y()    ) * k[1][0])
            + (input(x() - 1, y() + 1) * k[2][0])
            + (input(x()    , y() - 1) * k[0][1])
            + (input(x()    , y()    ) * k[1][1])
            + (input(x()    , y() + 1) * k[2][1])
            + (input(x() + 1, y() - 1) * k[0][2])
            + (input(x() + 1, y()    ) * k[1][2])
            + (input(x() + 1, y() + 1) * k[2][2])
        })
    }
}


impl<'a> ChainableSource for Function<'a> {
    fn run_on_image_inputs(&self, inpts: &[ImageBuffer<Luma<u8>, Vec<u8>>])
        -> ImageBuffer<Luma<u8>, Vec<u8>>
    {
        // a function doesn't currently have any dependency on any other function, it isn't yet
        // chained to anything.
        // to compile a function, the number of image inputs must equal the number of function
        // inputs
        assert!(self.num_inputs == inpts.len());

        // now, we need to compile the function
        // create a module to stick all the new code into
        let context = unsafe { llvm::Context::get_global() };
        let module = llvm::Module::new("jitmodule", &context);

        // the first hurdle is get the image data into the rust world
        let mut img_funcs = Vec::new();
        for inpt in inpts {
            let f = compile_image_to_llvm_function(&module, inpt);
            img_funcs.push(f);
        }

        let xbound = inpts[0].width();
        let ybound = inpts[0].height();
        let out: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(xbound as u32, ybound as u32);

        {
            let ft = llvm::Type::get::<fn() -> ()>(&context);
            let f = module.add_function("jitfunction", ft);
            let entry       = f.append("entry");
            let ilooph      = f.append("iloop_header");
            let iloopc      = f.append("iloop_cond");
            let iloopb      = f.append("iloop_body");
            let jlooph      = f.append("jloop_header");
            let jloopc      = f.append("jloop_cond");
            let jloopb      = f.append("jloop_body");
            let br_big      = f.append("clamp_ifbigtrue");
            let br_nobig    = f.append("clamp_ifbigfalse");
            let br_little   = f.append("clamp_iflittletrue");
            let br_nolittle = f.append("clamp_iflittlefalse");
            let br_done     = f.append("clamp_done");
            let jloope      = f.append("jloop_end");
            let iloope      = f.append("iloop_end");
            let exit        = f.append("exit");

            let builder = llvm::Builder::new(&context);

            builder.position_at_end(entry);
            let i = builder.build_alloca(llvm::Type::get::<i64>(&context));
            let j = builder.build_alloca(llvm::Type::get::<i64>(&context));
            builder.build_br(ilooph);

            builder.position_at_end(ilooph);
            builder.build_store(0i64.compile(&context), i);
            builder.build_br(iloopc);

            builder.position_at_end(iloopc);
            let ibound = (inpts[0].width() as i64).compile(&context);
            let ival = builder.build_load(i);
            let cmp = builder.build_cmp(ival, ibound, llvm::Predicate::LessThan);
            builder.build_cond_br(cmp, iloopb, Some(exit));

            builder.position_at_end(iloopb);
            builder.build_br(jlooph);

            builder.position_at_end(jlooph);
            builder.build_store(0i64.compile(&context), j);
            builder.build_br(jloopc);

            builder.position_at_end(jloopc);
            let jbound = (inpts[0].height() as i64).compile(&context);
            let jval = builder.build_load(j);
            let cmp = builder.build_cmp(jval, jbound, llvm::Predicate::LessThan);
            builder.build_cond_br(cmp, jloopb, Some(iloope));

            builder.position_at_end(jloopb);
            let ival = builder.build_load(i);
            let jval = builder.build_load(j);

            let px = self.e.compile(ival, jval, context, &module, &builder, &img_funcs);

            // clamp the pixel
            let newpx = builder.build_alloca(llvm::Type::get::<i64>(&context));
            let zero = 0i64.compile(&context);
            builder.build_store(zero, newpx);

            let max = 225i64.compile(&context);
            let cmp = builder.build_cmp(px, max, llvm::Predicate::GreaterThanOrEqual);
            builder.build_cond_br(cmp, br_big, Some(br_nobig));

            builder.position_at_end(br_big);
            let big = 255i64.compile(&context);
            builder.build_store(big, newpx);
            builder.build_br(br_done);

            builder.position_at_end(br_nobig);
            let min = 0i64.compile(&context);
            let cmp = builder.build_cmp(px, min, llvm::Predicate::LessThan);
            builder.build_cond_br(cmp, br_little, Some(br_nolittle));

            builder.position_at_end(br_little);
            let tmp = 0i64.compile(&context);
            builder.build_store(tmp, newpx);
            builder.build_br(br_done);

            builder.position_at_end(br_nolittle);
            builder.build_store(px, newpx);
            builder.build_br(br_done);

            builder.position_at_end(br_done);

            // store the new pixel
            let px = builder.build_load(newpx);
            let px = builder.build_trunc(px, llvm::Type::get::<i8>(&context));

            let ival = builder.build_load(i);
            let jval = builder.build_load(j);

            let w = (inpts[0].width() as i64).compile(&context);
            let offset = builder.build_mul(w, jval);
            let offset = builder.build_add(offset, ival);

            let ty0 = llvm::Type::get::<i8>(&context);
            let ty1 = llvm::Type::new_pointer(ty0);

            let ptr = (out.as_ptr() as u64).compile(&context);
            let ptr = builder.build_int_to_ptr(ptr, ty1);

            let ptr = builder.build_gep(ptr, &[offset]);
            builder.build_store(px, ptr);
            builder.build_br(jloope);

            builder.position_at_end(jloope);
            let jval = builder.build_load(j);
            let inc = builder.build_add(jval, 1i64.compile(&context));
            builder.build_store(inc, j);
            builder.build_br(jloopc);

            builder.position_at_end(iloope);
            let ival = builder.build_load(i);
            let inc = builder.build_add(ival, 1i64.compile(&context));
            builder.build_store(inc, i);
            builder.build_br(iloopc);

            builder.position_at_end(exit);
            builder.build_ret_void();
        }

        module.verify().unwrap();
        module.optimize(3, 1000000);
        println!("{:?}", module);

        let opts = llvm::JitOptions { opt_level: 3 };
        let ee = llvm::JitEngine::new(&module, opts).unwrap();
        let f = ee.find_function("jitfunction").unwrap();
        ee.run_function(f, &[]);

        out
    }
}
