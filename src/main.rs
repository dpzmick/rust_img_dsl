#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(slice_patterns)]

extern crate image;
extern crate llvm;

use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;
use std::fmt::Debug;

use llvm::Compile;
use llvm::ExecutionEngine;
use llvm::GetContext;

use std::fs::File;
use std::path::Path;
use image::ImageBuffer;
use image::Pixel;
use image::Luma;

trait Inputable {
    fn at(&self, x: i64, y: i64) -> i64;
    fn width(&self) -> i64;
    fn height(&self) -> i64;
    fn compile<'a>(&self, module: &'a llvm::CSemiBox<'a, llvm::Module>) -> &'a llvm::Function;
}

trait Expr: Debug {
    fn eval(&self, env: FunctionEnv, inputs: &Vec<&Inputable>) -> i64;
    fn compile<'a>(&self,
                   x: &'a llvm::Value,
                   y: &'a llvm::Value,
                   context: &'a llvm::Context,
                   module: &'a llvm::CSemiBox<'a, llvm::Module>,
                   builder: &'a llvm::Builder,
                   inputs: &Vec<&'a llvm::Function>)
        -> &'a llvm::Value;
}

#[derive(Debug, Clone, Copy)]
enum Var { X, Y }

struct VecInpt { v: Vec<i64> }

impl VecInpt {
    pub fn new(v: Vec<i64>) -> Self {
        VecInpt {v: v}
    }
}

impl Inputable for VecInpt {
    fn at(&self, x: i64, y: i64) -> i64 {
        if y != 0 {
            return 0
        }

        if x < 0  || x >= (self.v.len() as i64) {
            return 0
        }

        self.v[x as usize]
    }

    fn width(&self) -> i64 {
        self.v.len() as i64
    }

    fn height(&self) -> i64 {
        0
    }

    fn compile<'a>(&self, module: &'a llvm::CSemiBox<'a, llvm::Module>) -> &'a llvm::Function {
        let ft = llvm::Type::get::<fn(i64, i64) -> i64>(module.get_context());

        module.add_function("vec_input", ft)
    }
}

impl Inputable for ImageBuffer<Luma<u8>, Vec<u8>> {
    fn at(&self, x: i64, y: i64) -> i64 {
        if x < 0 || x >= self.width() as i64 {
            return 0
        }

        if y < 0 || y >= self.height() as i64 {
            return 0
        }

        let (px, _, _, _) = self.get_pixel(x as u32, y as u32).channels4();
        px as i64
    }

    fn width(&self) -> i64 {
        self.width() as i64
    }

    fn height(&self) -> i64 {
        self.height() as i64
    }

    fn compile<'a>(&self, module: &'a llvm::CSemiBox<'a, llvm::Module>) -> &'a llvm::Function {
        let context = module.get_context();

        let ft = llvm::Type::get::<fn(i64, i64) -> i64>(module.get_context());
        let f = module.add_function("image_buffer", ft);
        f.add_attribute(llvm::Attribute::AlwaysInline);
        let entry = f.append("entry");

        let builder = llvm::Builder::new(&context);

        builder.position_at_end(entry);

        let x = &f[0];
        let y = &f[1];
        let w = (self.width() as i64).compile(&context);

        let offset = builder.build_mul(w, y);
        let offset = builder.build_add(offset, x);

        let ty0 = llvm::Type::get::<i8>(&context);
        let ty1 = llvm::Type::new_pointer(ty0);

        let ptr = (self.as_ptr() as u64).compile(&context);
        let ptr = builder.build_int_to_ptr(ptr, ty1);
        let ptr = builder.build_gep(ptr, &[offset]);
        let e = builder.build_load(ptr);

        let e = builder.build_zext(e, llvm::Type::get::<i64>(&context));
        builder.build_ret(e);

        f
    }
}


#[derive(Debug, Clone, Copy)]
struct ConstExpr { v: i64 }

impl Expr for ConstExpr {
    fn eval(&self, _: FunctionEnv, _: &Vec<&Inputable>) -> i64 {
        self.v
    }

    fn compile<'a>(&self,
                   _: &'a llvm::Value,
                   _: &'a llvm::Value,
                   context: &'a llvm::Context,
                   _: &'a llvm::CSemiBox<'a, llvm::Module>,
                   _: &'a llvm::Builder,
                   _: &Vec<&'a llvm::Function>)
        -> &'a llvm::Value
    {
        self.v.compile(context)
    }
}

impl Add<ConstExpr> for ConstExpr {
    type Output = AddExpr;
    fn add(self, rhs: ConstExpr) -> AddExpr {
        AddExpr { e1: Box::new(self), e2: Box::new(rhs) }
    }
}

impl Mul<ConstExpr> for ConstExpr {
    type Output = MulExpr;

    fn mul(self, rhs: ConstExpr) -> MulExpr {
        MulExpr { e1: Box::new(self), e2: Box::new(rhs) }
    }
}

#[test]
fn test_const() {
    let env = FunctionEnv { x: 0, y: 100 };
    let inpt = VecInpt::new(Vec::new());

    let e = ConstExpr { v: 100 };

    let r = e.eval(env, &vec![&inpt]);
    assert!(r == 100);
}

#[derive(Debug, Clone, Copy)]
struct VarRef { v: Var }

impl Expr for VarRef {
    fn eval(&self, env: FunctionEnv, _: &Vec<&Inputable>) -> i64 {
        match self.v {
            Var::X => env.x,
            Var::Y => env.y
        }
    }

    fn compile<'a>(&self,
                   x: &'a llvm::Value,
                   y: &'a llvm::Value,
                   _: &'a llvm::Context,
                   _: &'a llvm::CSemiBox<'a, llvm::Module>,
                   _: &'a llvm::Builder,
                   _: &Vec<&'a llvm::Function>)
        -> &'a llvm::Value
    {
        match self.v {
            Var::X => x,
            Var::Y => y
        }
    }
}

impl Add<i64> for VarRef {
    type Output = AddExpr;

    fn add(self, rhs: i64) -> AddExpr {
        AddExpr { e1: Box::new(self), e2: Box::new(ConstExpr { v: rhs }) }
    }
}

impl Sub<i64> for VarRef {
    type Output = AddExpr;

    fn sub(self, rhs: i64) -> AddExpr {
        AddExpr { e1: Box::new(self), e2: Box::new(ConstExpr { v: -rhs }) }
    }
}

#[test]
fn test_var_ref() {
    let env = FunctionEnv { x: 0, y: 100 };
    let inpt = VecInpt::new(Vec::new());
    let inpts = vec![&inpt as &Inputable];

    let e1 = VarRef { v: Var::X };
    assert!(e1.eval(env, &inpts) == 0);

    let e1 = VarRef { v: Var::Y };
    assert!(e1.eval(env, &inpts) == 100);

    // should be easy to work with
    let e2 = e1 + 1;
    assert!(e2.eval(env, &inpts) == 101);

    let e2 = e1 + 2;
    assert!(e2.eval(env, &inpts) == 102);
}

#[derive(Debug)]
struct AddExpr{ e1: Box<Expr>, e2: Box<Expr> }

impl Expr for AddExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Vec<&Inputable>) -> i64 {
        let e1 = self.e1.eval(env.clone(), inpt);
        let e2 = self.e2.eval(env.clone(), inpt);

        e1 + e2
    }

    fn compile<'a>(&self,
                   x: &'a llvm::Value,
                   y: &'a llvm::Value,
                   context: &'a llvm::Context,
                   module: &'a llvm::CSemiBox<'a, llvm::Module>,
                   builder: &'a llvm::Builder,
                   inputs: &Vec<&'a llvm::Function>)
        -> &'a llvm::Value
    {
        let v1 = self.e1.compile(x, y, context, module, builder, inputs);
        let v2 = self.e2.compile(x, y, context, module, builder, inputs);
        builder.build_add(v1, v2)
    }
}

impl Add<AddExpr> for AddExpr {
    type Output = AddExpr;

    fn add(self, rhs: AddExpr) -> AddExpr {
        AddExpr { e1: Box::new(self), e2: Box::new(rhs) }
    }
}

impl Add<MulExpr> for AddExpr {
    type Output = AddExpr;

    fn add(self, rhs: MulExpr) -> AddExpr {
        AddExpr { e1: Box::new(self), e2: Box::new(rhs) }
    }
}

#[test]
fn test_add_expr() {
    let inpt = VecInpt::new(Vec::new());
    let inpts = vec![&inpt as &Inputable];
    let env = FunctionEnv {x:0, y:0};

    let e1 = ConstExpr { v: 100 };
    let e2 = ConstExpr { v: 100 };

    let e = AddExpr { e1: Box::new(e1), e2: Box::new(e2) };

    let r = e.eval(env, &inpts);
    assert!(r == 200);

    // try with overload
    let e = e1 + e2;

    let r = e.eval(env, &inpts);
    assert!(r == 200);
}

#[derive(Debug)]
struct MulExpr{ e1: Box<Expr>, e2: Box<Expr> }

impl Expr for MulExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Vec<&Inputable>) -> i64 {
        let e1 = self.e1.eval(env.clone(), inpt);
        let e2 = self.e2.eval(env.clone(), inpt);

        e1 * e2
    }

    fn compile<'a>(&self,
                   x: &'a llvm::Value,
                   y: &'a llvm::Value,
                   context: &'a llvm::Context,
                   module: &'a llvm::CSemiBox<'a, llvm::Module>,
                   builder: &'a llvm::Builder,
                   inputs: &Vec<&'a llvm::Function>)
        -> &'a llvm::Value
    {
        let v1 = self.e1.compile(x, y, context, module, builder, inputs);
        let v2 = self.e2.compile(x, y, context, module, builder, inputs);
        builder.build_mul(v1, v2)
    }
}

impl Add<MulExpr> for MulExpr {
    type Output = AddExpr;

    fn add(self, rhs: MulExpr) -> AddExpr {
        AddExpr { e1: Box::new(self), e2: Box::new(rhs) }
    }
}

#[test]
fn test_mul_expr() {
    let inpt = VecInpt::new(Vec::new());
    let inpts = vec![&inpt as &Inputable];
    let env = FunctionEnv {x:0, y:0};

    let e1 = ConstExpr { v: 100 };
    let e2 = ConstExpr { v: 100 };

    let e = MulExpr { e1: Box::new(e1), e2: Box::new(e2) };

    let r = e.eval(env, &inpts);
    assert!(r == 100*100);

    // try with overload
    let e = e1 * e2;
    let r = e.eval(env, &inpts);
    assert!(r == 100*100);
}

#[derive(Debug)]
struct SqrtExpr { x: Box<Expr> }

impl Expr for SqrtExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Vec<&Inputable>) -> i64 {
        let x = self.x.eval(env, inpt);
        (x as f64).sqrt() as i64
    }

    fn compile<'a>(&self,
                   x: &'a llvm::Value,
                   y: &'a llvm::Value,
                   _: &'a llvm::Context,
                   module: &'a llvm::CSemiBox<'a, llvm::Module>,
                   builder: &'a llvm::Builder,
                   inputs: &Vec<&'a llvm::Function>)
        -> &'a llvm::Value
    {
        let context = module.get_context();

        let ftype = llvm::Type::get::<fn(i64) -> (i64)>(context);
        let f = module.add_function("sqrt", ftype);
        f.add_attribute(llvm::Attribute::AlwaysInline);
        let entry     = f.append("entry");
        let loop_cond = f.append("loop_cond");
        let loop_body = f.append("loop_body");
        let exit      = f.append("exit");

        let builder2 = llvm::Builder::new(&context);

        builder2.position_at_end(entry);
        let tmp = builder2.build_alloca(llvm::Type::get::<i64>(context));
        let one = 1i64.compile(context);
        builder2.build_store(one, tmp);
        builder2.build_br(loop_cond);

        builder2.position_at_end(loop_cond);
        let val = builder2.build_load(tmp);
        let val = builder2.build_mul(val, val);
        let cmp = builder2.build_cmp(val, &f[0], llvm::Predicate::LessThan);
        builder2.build_cond_br(cmp, loop_body, Some(exit));

        builder2.position_at_end(loop_body);
        let val = builder2.build_load(tmp);
        let inc = builder2.build_add(val, 1i64.compile(&context));
        builder2.build_store(inc, tmp);
        builder2.build_br(loop_cond);

        builder2.position_at_end(exit);
        let val = builder2.build_load(tmp);
        builder2.build_ret(val);

        // expression
        let v = self.x.compile(x, y, context, module, builder, inputs);
        builder.build_call(f, &[v])
    }
}

#[test]
fn test_sqrt_expr() {
    let inpt = VecInpt::new(Vec::new());
    let env = FunctionEnv {x:0, y:0};

    let c = ConstExpr { v: 100 };
    let e1 = SqrtExpr { x: Box::new(c) };
    assert!(e1.eval(env, &vec![&inpt]) == 10);
}

#[derive(Debug)]
struct InputExpr { id: usize, x: Box<Expr>, y: Box<Expr> }

impl Expr for InputExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Vec<&Inputable>) -> i64 {
        let x = self.x.eval(env.clone(), inpt);
        let y = self.y.eval(env.clone(), inpt);

        inpt[self.id].at(x, y)
    }

    fn compile<'a>(&self,
                   x: &'a llvm::Value,
                   y: &'a llvm::Value,
                   context: &'a llvm::Context,
                   module: &'a llvm::CSemiBox<'a, llvm::Module>,
                   builder: &'a llvm::Builder,
                   inputs: &Vec<&'a llvm::Function>)
        -> &'a llvm::Value
    {
        let x = self.x.compile(x, y, context, module, builder, inputs);
        let y = self.y.compile(x, y, context, module, builder, inputs);
        builder.build_call(inputs[self.id], &[x, y])
    }
}

impl Mul<i64> for InputExpr {
    type Output = MulExpr;

    fn mul(self, rhs: i64) -> MulExpr {
        MulExpr { e1: Box::new(self), e2: Box::new(ConstExpr { v: rhs }) }
    }
}

impl Mul<InputExpr> for InputExpr {
    type Output = MulExpr;

    fn mul(self, rhs: InputExpr) -> MulExpr {
        MulExpr { e1: Box::new(self), e2: Box::new(rhs) }
    }
}

#[test]
fn test_inpt_expr() {
    let mut vec = Vec::new();
    vec.push(0);

    let inpt = VecInpt::new(vec);
    let env = FunctionEnv {x:0, y:0};

    let x = ConstExpr { v: 0 };
    let y = ConstExpr { v: 0 };
    let e = InputExpr { id:0, x: Box::new(x), y: Box::new(y) };

    assert!(0 == e.eval(env, &vec![&inpt]));
}

macro_rules! make_input(
    ($name:ident, $number:expr) => (
            fn $name<E1: Expr + 'static, E2: Expr + 'static>(x: E1, y: E2) -> InputExpr {
                InputExpr {id: $number, x: Box::new(x), y: Box::new(y) }
            }
        );
    );

struct InputBuilder { id: usize }

impl InputBuilder {
    pub fn build<E1: Expr + 'static, E2: Expr + 'static>(&self)
        -> fn(E1, E2) -> InputExpr
        {
            match self.id {
                0 => {
                    make_input!(input, 0);
                    input
                },
                1 => {
                    make_input!(input, 1);
                    input
                }
                _ => unimplemented!()
            }
        }
}

impl<E1: Expr + 'static, E2: Expr + 'static> Fn<(E1, E2)> for InputBuilder {
    extern "rust-call" fn call(&self, args: (E1, E2)) -> InputExpr {
        let (a1, a2) = args;
        self.build()(a1, a2)
    }
}

impl<E1: Expr + 'static, E2: Expr + 'static> FnMut<(E1, E2)> for InputBuilder {
    extern "rust-call" fn call_mut(&mut self, args: (E1, E2)) -> InputExpr {
        let (a1, a2) = args;
        self.build()(a1, a2)
    }
}

impl<E1: Expr + 'static, E2: Expr + 'static> FnOnce<(E1, E2)> for InputBuilder {
    type Output = InputExpr;
    extern "rust-call" fn call_once(self, args: (E1, E2)) -> InputExpr {
        let (a1, a2) = args;
        self.build()(a1, a2)
    }
}

struct Function {
    num_inputs: usize,
    e:          Box<Expr>
}

impl Function {
    pub fn new<F, E: Expr + 'static>(num_inputs: usize, gen: F) -> Self
        where F: Fn(VarRef, VarRef, &[InputBuilder]) -> E
    {
        let x = VarRef {v: Var::X};
        let y = VarRef {v: Var::Y};

        let b = InputBuilder { id: 0 };
        let e = gen(x, y, &[b]);

        Function { e: Box::new(e), num_inputs: num_inputs }
    }

    // pub fn eval(&self) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    //     let xbound = self.input.width();
    //     let ybound = self.input.height();

    //     let mut out = ImageBuffer::new(xbound as u32, ybound as u32);

    //     for x in 0..xbound {
    //         for y in 0..ybound {
    //             let v = self.at(x, y);
    //             let v = if v > 255 { 255 } else if v < 0 { 0 } else { v } as u8;
    //             let p = Luma::from_channels(v, 255, 255, 255);
    //             out.put_pixel(x as u32, y as u32, p);
    //         }
    //     }

    //     out
    // }

    pub fn gen_3x3_kernel(inpt: Box<Inputable>, k: [[i64; 3]; 3]) -> Self {
        Function::new(1, |x, y, inputs| {
            let ref input = inputs[0];

              (input(x - 1, y - 1) * k[0][0])
            + (input(x - 1, y    ) * k[1][0])
            + (input(x - 1, y + 1) * k[2][0])
            + (input(x    , y - 1) * k[0][1])
            + (input(x    , y    ) * k[1][1])
            + (input(x    , y + 1) * k[2][1])
            + (input(x + 1, y - 1) * k[0][2])
            + (input(x + 1, y    ) * k[1][2])
            + (input(x + 1, y + 1) * k[2][2])
        })
    }
}

// impl Inputable for Function1 {
//     fn at(&self, x: i64, y: i64) -> i64 {
//         if x < 0 || x >= self.input.width() {
//             return 0
//         }

//         if y < 0 || y >= self.input.height() {
//             return 0
//         }

//         let env = FunctionEnv {x: x, y: y};
//         let inpts = vec![&*self.input];
//         self.e.eval(env, &inpts)
//     }

//     fn width(&self) -> i64 {
//         self.input.width()
//     }

//     fn height(&self) -> i64 {
//         self.input.height()
//     }

//     fn compile<'a>(&self, module: &'a llvm::CSemiBox<'a, llvm::Module>) -> &'a llvm::Function {
//         // compile all the inputs
//         let input = self.input.compile(&module);

//         let context = module.get_context();

//         let ftype = llvm::Type::get::<fn(i64, i64) -> (i64)>(&context);
//         let f = module.add_function("function1", ftype);
//         f.add_attribute(llvm::Attribute::AlwaysInline);
//         let entry = f.append("entry");

//         let builder = llvm::Builder::new(&context);

//         builder.position_at_end(entry);
//         let e = self.e.compile(&f[0], &f[1], &context, module, &builder, &vec![input]);
//         builder.build_ret(e);

//         f
//     }
// }


struct Function2 {
    input1: Box<Inputable>,
    input2: Box<Inputable>,
    e:      Box<Expr>
}

impl Inputable for Function2 {
    fn at(&self, x: i64, y: i64) -> i64 {
        if x < 0 || x >= self.input1.width() {
            return 0
        }

        if y < 0 || y >= self.input1.height() {
            return 0
        }

        let env = FunctionEnv {x: x, y: y};
        let inpts = vec![&*self.input1, &*self.input2];
        self.e.eval(env, &inpts)
    }

    fn width(&self) -> i64 {
        self.input1.width()
    }

    fn height(&self) -> i64 {
        self.input1.height()
    }

    fn compile<'a>(&self, module: &'a llvm::CSemiBox<'a, llvm::Module>) -> &'a llvm::Function {
        // compile all the inputs
        let input1 = self.input1.compile(&module);
        let input2 = self.input2.compile(&module);

        let context = module.get_context();

        let ftype = llvm::Type::get::<fn(i64, i64) -> (i64)>(&context);
        let f = module.add_function("function2", ftype);
        f.add_attribute(llvm::Attribute::AlwaysInline);
        let entry = f.append("entry");

        let builder = llvm::Builder::new(&context);

        builder.position_at_end(entry);
        let e = self.e.compile(&f[0], &f[1], &context, module, &builder, &vec![input1, input2]);
        builder.build_ret(e);

        f
    }

}

impl Function2 {
    pub fn new<E: Expr + 'static, F>(input1: Box<Inputable>, input2: Box<Inputable>, gen: F) -> Self
        where F: Fn(VarRef, VarRef, InputBuilder, InputBuilder) -> E
    {
        let x = VarRef {v: Var::X};
        let y = VarRef {v: Var::Y};

        let i1 = InputBuilder { id: 0 };
        let i2 = InputBuilder { id: 1 };

        let e = gen(x, y, i1, i2);

        Function2 { e: Box::new(e), input1: input1, input2: input2 }
    }

    pub fn eval(&self) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let xbound = self.input1.width();
        let ybound = self.input1.height();

        let mut out = ImageBuffer::new(xbound as u32, ybound as u32);

        for x in 0..xbound {
            for y in 0..ybound {
                let v = self.at(x, y);
                let v = if v > 255 { 255 } else if v < 0 { 0 } else { v } as u8;
                let p = Luma::from_channels(v, 255, 255, 255);
                out.put_pixel(x as u32, y as u32, p);
            }
        }

        out
    }
}

fn run_jit<I: Inputable>(inpt: &I) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let xbound = inpt.width();
    let ybound = inpt.height();

    // this is actually mutated but don't tell anyone
    let out = ImageBuffer::new(xbound as u32, ybound as u32);

    let context = unsafe { llvm::Context::get_global() };
    let m = llvm::Module::new("jitmodule", &context);

    {
        let e = inpt.compile(&m);

        let ft = llvm::Type::get::<fn() -> ()>(&context);
        let f = m.add_function("jitfunction", ft);
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
        let ibound = inpt.width().compile(&context);
        let ival = builder.build_load(i);
        let cmp = builder.build_cmp(ival, ibound, llvm::Predicate::LessThan);
        builder.build_cond_br(cmp, iloopb, Some(exit));

        builder.position_at_end(iloopb);
        builder.build_br(jlooph);

        builder.position_at_end(jlooph);
        builder.build_store(0i64.compile(&context), j);
        builder.build_br(jloopc);

        builder.position_at_end(jloopc);
        let jbound = inpt.height().compile(&context);
        let jval = builder.build_load(j);
        let cmp = builder.build_cmp(jval, jbound, llvm::Predicate::LessThan);
        builder.build_cond_br(cmp, jloopb, Some(iloope));

        builder.position_at_end(jloopb);
        let ival = builder.build_load(i);
        let jval = builder.build_load(j);

        let px = builder.build_call(e, &[ival, jval]);

        // // clamp the pixel
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

        let w = (inpt.width() as i64).compile(&context);
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


    m.verify().unwrap();
    m.optimize(3, 1000000);

    println!("{:?}", m);

    let opts = llvm::JitOptions { opt_level: 3 };
    let ee = llvm::JitEngine::new(&m, opts).unwrap();
    let f = ee.find_function("jitfunction").unwrap();
    ee.run_function(f, &[]);

    out
}

#[derive(Debug, Clone, Copy)]
struct FunctionEnv {
    x: i64,
    y: i64
}

#[test]
fn test_id() {
    let mut raw: Vec<u8> = Vec::new();
    raw.push(0);
    raw.push(1);
    raw.push(2);
    raw.push(3);

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(2, 2, raw).unwrap();

    let id = Function1::new(Box::new(img), |x, y, input| { input(x, y) });

    let out = id.eval();

    let raw = out.into_raw();
    assert!(raw.len() == 4);
    assert!(raw[0] == 0);
    assert!(raw[1] == 1);
    assert!(raw[2] == 2);
    assert!(raw[3] == 3);
}

#[test]
fn test_shift_one() {
    let mut raw: Vec<u8> = Vec::new();
    raw.push(0);
    raw.push(100);
    raw.push(200);
    raw.push(255);
    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(2, 2, raw).unwrap();

    let shift_one = Function1::new(Box::new(img), |x, y, input| {
        input(x - 1, y)
    });

    let out = shift_one.eval();

    let raw = out.into_raw();
    assert!(raw.len() == 4);
    assert!(raw[0] == 0);
    assert!(raw[1] == 0);
    assert!(raw[2] == 0);
    assert!(raw[3] == 200);
}

#[test]
fn test_simple_sobel() {
    let inpt = [[0, 0, 0],
                [255, 255, 255],
                [0, 0, 0]];

    let mut raw: Vec<u8> = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            raw.push(inpt[x][y]);
        }
    }

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(3, 3, raw).unwrap();

    let k = [[-1, 0, 1],
             [-2, 0, 2],
             [-1, 0, 1]];

    let sobel_x = Function1::gen_3x3_kernel(Box::new(img), k);
    let out = sobel_x.eval();

    let raw = out.into_raw();
    assert!(raw.len() == 9);

    let expected = [255, 0, 0, 255, 0, 0, 255, 0, 0];
    for i in 0..9 {
        assert!(raw[i] == expected[i]);
    }
}

fn main() {
    // let inpt = image::open(&Path::new("in1.png")).unwrap();

    // let sobel_x = [[-1, 0, 1],
    //                [-2, 0, 2],
    //                [-1, 0, 1]];

    // let sobel_x = Function1::gen_3x3_kernel(Box::new(inpt.to_luma()), sobel_x);

    // let sobel_y = [[-1, -2, -1],
    //                [ 0,  0,  0],
    //                [ 1,  2,  1]];

    // let sobel_y = Function1::gen_3x3_kernel(Box::new(inpt.to_luma()), sobel_y);

    // let grad = Function2::new(Box::new(sobel_x), Box::new(sobel_y), |x, y, input1, input2| {
    //     let t1 = input1(x, y) * input1(x, y);
    //     let t2 = input2(x, y) * input2(x, y);

    //     SqrtExpr { x: Box::new(t1 + t2) }
    // });

    // let out = run_jit(&grad);

    // let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    // let _ = image::ImageLuma8(out).save(fout, image::PNG);
}
