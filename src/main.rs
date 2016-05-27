extern crate image;

use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

use std::fs::File;
use std::path::Path;
use std::ops::Deref;
use image::ImageBuffer;
use image::Pixel;
use image::Luma;

#[derive(Debug, Clone, Copy)]
enum Var { X, Y }

trait Inputable {
    fn at(&self, x: i64, y: i64) -> i64;
    fn width(&self) -> i64;
    fn height(&self) -> i64;
}

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
}

trait Eval {
    fn eval(&self, env: FunctionEnv, inpt: &Box<Inputable>) -> i64;
}

#[derive(Debug, Clone, Copy)]
struct ConstExpr { v: i64 }

impl Eval for ConstExpr {
    fn eval(&self, _: FunctionEnv, _: &Box<Inputable>) -> i64 {
        self.v
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

#[derive(Debug, Clone, Copy)]
struct VarRef { v: Var }

impl Eval for VarRef {
    fn eval(&self, env: FunctionEnv, _: &Box<Inputable>) -> i64 {
        match self.v {
            Var::X => env.x,
            Var::Y => env.y
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

struct AddExpr{ e1: Box<Eval>, e2: Box<Eval> }

impl Eval for AddExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Box<Inputable>) -> i64 {
        let e1 = self.e1.eval(env.clone(), inpt);
        let e2 = self.e2.eval(env.clone(), inpt);

        e1 + e2
    }
}

struct MulExpr{ e1: Box<Eval>, e2: Box<Eval> }

impl Eval for MulExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Box<Inputable>) -> i64 {
        let e1 = self.e1.eval(env.clone(), inpt);
        let e2 = self.e2.eval(env.clone(), inpt);

        e1 * e2
    }
}

struct InputExpr { x: Box<Eval>, y: Box<Eval> }

impl Eval for InputExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Box<Inputable>) -> i64 {
        let x = self.x.eval(env.clone(), inpt);
        let y = self.y.eval(env.clone(), inpt);

        inpt.at(x, y)
    }
}

// need these defined in both directions for each type....
impl Mul<i64> for InputExpr {
    type Output = MulExpr;

    fn mul(self, rhs: i64) -> MulExpr {
        MulExpr { e1: Box::new(self), e2: Box::new(ConstExpr { v: rhs }) }
    }
}

impl Mul<InputExpr> for i64 {
    type Output = MulExpr<>;

    fn mul(self, rhs: InputExpr) -> MulExpr {
        MulExpr { e1: Box::new(rhs), e2: Box::new(ConstExpr { v: self }) }
    }
}

#[test]
fn test_const() {
    let e = ConstExpr { v: 100 };
    let env = FunctionEnv {x:0, y:0};
    let inpt = VecInpt::new(Vec::new());
    let r = e.eval(env, &(Box::new(inpt) as Box<Inputable>));
    assert!(r == 100);
}

#[test]
fn test_var_ref() {
    let env = FunctionEnv { x: 0, y: 100 };
    let inpt = Box::new(VecInpt::new(Vec::new())) as Box<Inputable>;

    let e1 = VarRef { v: Var::X };
    assert!(e1.eval(env, &inpt) == 0);

    let e1 = VarRef { v: Var::Y };
    assert!(e1.eval(env, &inpt) == 100);

    // should be easy to work with
    let e2 = e1 + 1;
    assert!(e2.eval(env, &inpt) == 101);

    let e2 = e1 + 2;
    assert!(e2.eval(env, &inpt) == 102);
}

#[test]
fn test_add_expr() {
    let inpt = Box::new(VecInpt::new(Vec::new())) as Box<Inputable>;
    let env = FunctionEnv {x:0, y:0};

    let e1 = ConstExpr { v: 100 };
    let e2 = ConstExpr { v: 100 };

    let e = AddExpr { e1: Box::new(e1), e2: Box::new(e2) };

    let r = e.eval(env, &inpt);
    assert!(r == 200);

    // try with overload
    let e = e1 + e2;

    let r = e.eval(env, &inpt);
    assert!(r == 200);
}

#[test]
fn test_mul_expr() {
    let inpt = Box::new(VecInpt::new(Vec::new())) as Box<Inputable>;
    let env = FunctionEnv {x:0, y:0};

    let e1 = ConstExpr { v: 100 };
    let e2 = ConstExpr { v: 100 };

    let e = MulExpr { e1: Box::new(e1), e2: Box::new(e2) };

    let r = e.eval(env, &inpt);
    assert!(r == 100*100);

    // try with overload
    let e = e1 * e2;
    let r = e.eval(env, &inpt);
    assert!(r == 100*100);
}

#[test]
fn test_inpt_expr() {
    let mut vec = Vec::new();
    vec.push(0);

    let inpt = Box::new(VecInpt::new(vec)) as Box<Inputable>;
    let env = FunctionEnv {x:0, y:0};

    let x = ConstExpr { v: 0 };
    let y = ConstExpr { v: 0 };
    let e = InputExpr { x: Box::new(x), y: Box::new(y) };

    assert!(0 == e.eval(env, &inpt));
}

struct Function {
    e: Box<Eval>
}

impl Function {
    // TODO chaining

    pub fn new<F, E1: Eval + 'static, E2: Eval + 'static, E3: Eval + 'static>(gen: F) -> Self
        where F : Fn(VarRef, VarRef, &Fn(E1, E2) -> InputExpr) -> E3,
    {
        let x = VarRef {v: Var::X};
        let y = VarRef {v: Var::Y};

        let input = |x, y| {
            InputExpr {
                x: Box::new(x) as Box<Eval>,
                y: Box::new(y) as Box<Eval>
            }
        };

        let e = gen(x, y, &input);

        Function { e: Box::new(e) }
    }

    pub fn eval_on<I: Inputable + 'static>(&self, inpt: I) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let mut out = ImageBuffer::new(inpt.width() as u32, inpt.height() as u32);
        let inpt = Box::new(inpt) as Box<Inputable>;

        let xbound = (*inpt).width();
        let ybound = (*inpt).height();

        for x in 0..xbound {
            for y in 0..ybound {
                let env = FunctionEnv {x: x, y: y};
                let v = self.e.eval(env, &inpt);
                let v = if v > 255 { 255 } else if v < 0 { 0 } else { v } as u8;
                let p = Luma::from_channels(v, 255, 255, 255);
                out.put_pixel(x as u32, y as u32, p);
            }
        }

        out
    }
}

#[derive(Debug, Clone, Copy)]
struct FunctionEnv {
    x: i64,
    y: i64
}

#[test]
fn test_id() {
    let id = Function::new(|x, y, input| { input(x, y) });

    let mut raw: Vec<u8> = Vec::new();
    raw.push(0);
    raw.push(1);
    raw.push(2);
    raw.push(3);

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(2, 2, raw).unwrap();
    let out = id.eval_on(img);

    let raw = out.into_raw();
    assert!(raw.len() == 4);
    assert!(raw[0] == 0);
    assert!(raw[1] == 1);
    assert!(raw[2] == 2);
    assert!(raw[3] == 3);
}

#[test]
fn test_shift_one() {
    let shift_one = Function::new(|x, y, input| { input(x - 1, y) });

    let mut raw: Vec<u8> = Vec::new();
    raw.push(0);
    raw.push(100);
    raw.push(200);
    raw.push(255);

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(2, 2, raw).unwrap();
    let out = shift_one.eval_on(img);

    let raw = out.into_raw();
    println!("{:?}", raw);
    assert!(raw.len() == 4);
    assert!(raw[0] == 0);
    assert!(raw[1] == 0);
    assert!(raw[2] == 0);
    assert!(raw[3] == 200);
}

fn main() {
    // let id = Function::new(|x, y, input| {
    //     input(x - 10, y) * -1
    // });

    // id = 10;

    // let inpt = image::open(&Path::new("in1.png")).unwrap();

    // let out = id.eval_on(inpt.to_luma());

    // let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    // let _ = image::ImageLuma8(out).save(fout, image::PNG);
}
