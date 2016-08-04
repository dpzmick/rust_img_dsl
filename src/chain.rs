use std::mem;

use function::*;
use Img;
use image::ImageBuffer;

use llvm;
use llvm::GetContext;
use llvm::Compile;
use llvm::JitOptions;
use llvm::JitEngine;
use llvm::ExecutionEngine;

static mut source_count: i64 = 0;

// whatever let it be pub WHO CARES
pub enum ChainLink<'a> {
    ImageSource(i64),
    Linked(Vec<&'a ChainLink<'a>>, &'a Function <'a>)
}

impl<'a> ChainLink<'a> {
    /// creates a unique image source
    pub fn create_image_source() -> Self {
        let v = unsafe {
            source_count += 1;
            source_count
        };

        ChainLink::ImageSource(v)
    }

    pub fn link(inputs: Vec<&'a ChainLink>, to: &'a Function<'a>) -> Self {
        ChainLink::Linked(inputs.to_vec(), to)
    }

    pub fn compile(&self) -> CompiledChain<'a> {
        let c = unsafe {
            llvm::Context::get_global()
        };

        // read the core module
        let module = llvm::Module::parse_bitcode(c, "./core.bc").unwrap();

        {
            let builder = llvm::Builder::new(&c);

            // pull out the already defined function
            let function = module.get_function("function").unwrap();
            let ft = function.get_signature();
            function.add_attribute(llvm::Attribute::AlwaysInline);

            let entry = function.append("entry");
            builder.position_at_end(entry);

            // compile the chain into the module
            let ftocall = self.compile_into(&module, &ft);

            // call the new chain from function
            let x          = &function[0];
            let y          = &function[1];
            let inputs     = &function[2];
            let num_inputs = &function[3];

            let ret = builder.build_call(ftocall, &[x, y, inputs, num_inputs]);
            builder.build_ret(ret);
        }

        module.optimize(3, 0);

        CompiledChain { module: module }
    }

    fn compile_into(&self, module: &'a llvm::CSemiBox<'a, llvm::Module>, ft: &'a llvm::Type)
        -> &'a llvm::Function
    {
        match self {
            &ChainLink::ImageSource(idx) =>
                compile_image_src_to_llvm_function(module, idx, ft),

            &ChainLink::Linked(ref links, ref func) => {
                let funs = links.iter().map(|f| f.compile_into(module, ft)).collect();
                compile_function_to_llvm_function(module, func, ft, &funs)
            }
        }
    }
}

// create a function that will call the core function with the appropriate index
fn compile_image_src_to_llvm_function<'a>(module: &'a llvm::CSemiBox<'a, llvm::Module>,
                                          idx: i64,
                                          ft: &'a llvm::Type)
    -> &'a llvm::Function
{
    let context = module.get_context();
    let builder = llvm::Builder::new(&context);

    let f = module.add_function("image_source", ft);
    f.add_attribute(llvm::Attribute::AlwaysInline);

    let entry = f.append("entry");
    builder.position_at_end(entry);

    let x          = &f[0];
    let y          = &f[1];
    let inputs     = &f[2];
    let num_inputs = &f[3];

    let idx = idx.compile(context);

    let core_f = module.get_function("core_input_at").unwrap();
    let res = builder.build_call(core_f, &[x, y, inputs, num_inputs, idx]);
    builder.build_ret(res);

    f
}

// pass in the function type that should be used
fn compile_function_to_llvm_function<'a>(module: &'a llvm::CSemiBox<'a, llvm::Module>,
                                         func: &'a Function,
                                         ft: &'a llvm::Type,
                                         fun_inputs: &Vec<&'a llvm::Function>)
    -> &'a llvm::Function
{
    let context = module.get_context();
    let builder = llvm::Builder::new(&context);

    let f = module.add_function("compiled_fn", ft);
    f.add_attribute(llvm::Attribute::AlwaysInline);

    let entry = f.append("entry");
    builder.position_at_end(entry);

    let x          = &f[0];
    let y          = &f[1];
    let inputs     = &f[2];
    let num_inputs = &f[3];

    let e = func.get_expr();
    let e = e.compile(x, y, inputs, num_inputs, context, module, &*builder, fun_inputs);
    builder.build_ret(e);

    f
}

pub struct CompiledChain<'a> {
    pub module: llvm::CSemiBox<'a, llvm::Module>
}

impl<'a> CompiledChain<'a> {
    /// will fail with assertion failure if inputs not all same dimensions
    pub fn run_on(&self, inputs: &[&Img]) -> Img {
        assert!(inputs.len() >= 1);

        let dim = inputs[0].dimensions();

        for &i in inputs {
            assert!(i.dimensions() == dim);
        }

        let out = ImageBuffer::new(dim.0, dim.1);

        // call some llvm functions to get some llvm values
        // such a mess of types someone get me off of this wild ride
        let opts = JitOptions {opt_level: 3};
        let engine = JitEngine::new(&self.module, opts).unwrap();

        let outptr = out.as_ptr() as *const i8;

        let inptsptrs: Vec<*const i8> = inputs.iter().map(|i| i.as_ptr() as *const i8).collect();

        let jitfunction = engine.find_function("jitfunction").unwrap();
        engine.with_function(jitfunction,
                             |f: extern fn((i64, i64, *const i8, i64, u64))-> () | {

            // the llvm-rs crate doesn't actually work, have to cast the function it gives us to
            // one of the correct type
            let f: fn(i64, i64, *const i8, i64, u64) -> ()
                = unsafe { mem::transmute(f) };

            let width: i64  = out.width() as i64;
            let height: i64 = out.height() as i64;
            f(width, height, outptr, inptsptrs.as_ptr() as i64, inputs.len() as u64);
        });

        out
    }
}

// compilation strategy: one function per Function. each function (x,y) -> result
// will emit pretty inefficient code that the optimizer can't do much about, but that's okay
// try to keep as much as possible in this file
