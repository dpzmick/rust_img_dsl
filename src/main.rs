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
    fn eval<I: Inputable>(&self, env: FunctionEnv, inpt: &I) -> i64;
}

#[derive(Debug, Clone, Copy)]
struct ConstExpr { v: i64 }

impl Eval for ConstExpr {
    fn eval<I: Inputable>(&self, env: FunctionEnv, inpt: &I) -> i64 {
        self.v
    }
}

#[derive(Debug, Clone, Copy)]
struct VarRef { v: Var }

impl Eval for VarRef {
    fn eval<I: Inputable>(&self, env: FunctionEnv, inpt: &I) -> i64 {
        match self.v {
            Var::X => env.x,
            Var::Y => env.y
        }
    }
}

impl Add<i64> for VarRef {
    type Output = AddExpr<VarRef, ConstExpr>;

    fn add(self, rhs: i64) -> AddExpr<VarRef, ConstExpr> {
        AddExpr { e1: Box::new(self), e2: Box::new(ConstExpr { v: rhs }) }
    }
}

impl Sub<i64> for VarRef {
    type Output = AddExpr<VarRef, ConstExpr>;

    fn sub(self, rhs: i64) -> AddExpr<VarRef, ConstExpr> {
        AddExpr { e1: Box::new(self), e2: Box::new(ConstExpr { v: -rhs }) }
    }
}

#[derive(Debug, Clone)]
struct AddExpr<E1: Eval, E2: Eval>{ e1: Box<E1>, e2: Box<E2> }

impl<E1: Eval, E2: Eval> Eval for AddExpr<E1, E2> {
    fn eval<I: Inputable>(&self, env: FunctionEnv, inpt: &I) -> i64 {
        let e1 = self.e1.eval(env.clone(), inpt);
        let e2 = self.e2.eval(env.clone(), inpt);

        e1 + e2
    }
}

#[derive(Debug, Clone)]
struct MulExpr<E1: Eval, E2: Eval>{ e1: Box<E1>, e2: Box<E2> }

impl<E1: Eval, E2: Eval> Eval for MulExpr<E1, E2> {
    fn eval<I: Inputable>(&self, env: FunctionEnv, inpt: &I) -> i64 {
        let e1 = self.e1.eval(env.clone(), inpt);
        let e2 = self.e2.eval(env.clone(), inpt);

        e1 * e2
    }
}

struct InputExpr<E1: Eval, E2: Eval> { x: Box<E1>, y: Box<E2> }

impl<E1: Eval, E2: Eval> Eval for InputExpr<E1, E2> {
    fn eval<I: Inputable>(&self, env: FunctionEnv, inpt: &I) -> i64 {
        let x = self.x.eval(env.clone(), inpt);
        let y = self.y.eval(env.clone(), inpt);

        inpt.at(x, y)
    }
}

// need these defined in both directions for each type....
impl<E1: Eval, E2: Eval> Mul<i64> for InputExpr<E1, E2> {
    type Output = MulExpr<InputExpr<E1, E2>, ConstExpr>;

    fn mul(self, rhs: i64) -> MulExpr<InputExpr<E1, E2>, ConstExpr> {
        MulExpr { e1: Box::new(self), e2: Box::new(ConstExpr { v: rhs }) }
    }
}

impl<E1: Eval, E2: Eval> Mul<InputExpr<E1, E2>> for i64 {
    type Output = MulExpr<InputExpr<E1, E2>, ConstExpr>;

    fn mul(self, rhs: InputExpr<E1, E2>) -> MulExpr<InputExpr<E1, E2>, ConstExpr> {
        MulExpr { e1: Box::new(rhs), e2: Box::new(ConstExpr { v: self }) }
    }
}


#[derive(Debug)]
struct Function<E: Eval> {
    e: E
}

impl<E: Eval> Function<E> {
    // TODO chaining

    pub fn new<F, E1: Eval, E2: Eval>(gen: F) -> Self
        where F : Fn(VarRef, VarRef, &Fn(E1, E2) -> InputExpr<E1, E2>) -> E,
    {
        let x = VarRef {v: Var::X};
        let y = VarRef {v: Var::Y};

        let input = |x, y| { InputExpr { x: Box::new(x), y: Box::new(y) } };

        let e = gen(x, y, &input);

        Function { e: e }
    }

    pub fn eval_on<I: Inputable>(&self, inpt: I) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let mut out = ImageBuffer::new(inpt.width() as u32, inpt.height() as u32);

        for x in 0..inpt.width() {
            for y in 0..inpt.height() {
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
    let id = Function::new(|x, y, input| {
        input(x - 10, y)
    });

    let inpt = image::open(&Path::new("in1.png")).unwrap();

    let out = id.eval_on(inpt.to_luma());

    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    let _ = image::ImageLuma8(out).save(fout, image::PNG);
}
