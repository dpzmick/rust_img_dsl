extern crate image;

use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;
use std::fmt::Debug;

use std::fs::File;
use std::path::Path;
use image::ImageBuffer;
use image::Pixel;
use image::Luma;

trait Inputable {
    fn at(&self, x: i64, y: i64) -> i64;
    fn width(&self) -> i64;
    fn height(&self) -> i64;
}

trait Eval: Debug {
    fn eval(&self, env: FunctionEnv, inputs: &Vec<&Inputable>) -> i64;
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


#[derive(Debug, Clone, Copy)]
struct ConstExpr { v: i64 }

impl Eval for ConstExpr {
    fn eval(&self, _: FunctionEnv, _: &Vec<&Inputable>) -> i64 {
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

impl Eval for VarRef {
    fn eval(&self, env: FunctionEnv, _: &Vec<&Inputable>) -> i64 {
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
struct AddExpr{ e1: Box<Eval>, e2: Box<Eval> }

impl Eval for AddExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Vec<&Inputable>) -> i64 {
        let e1 = self.e1.eval(env.clone(), inpt);
        let e2 = self.e2.eval(env.clone(), inpt);

        e1 + e2
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
struct MulExpr{ e1: Box<Eval>, e2: Box<Eval> }

impl Eval for MulExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Vec<&Inputable>) -> i64 {
        let e1 = self.e1.eval(env.clone(), inpt);
        let e2 = self.e2.eval(env.clone(), inpt);

        e1 * e2
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
struct SqrtExpr { x: Box<Eval> }

impl Eval for SqrtExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Vec<&Inputable>) -> i64 {
        let x = self.x.eval(env, inpt);
        (x as f64).sqrt() as i64
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
struct InputExpr { id: usize, x: Box<Eval>, y: Box<Eval> }

impl Eval for InputExpr {
    fn eval(&self, env: FunctionEnv, inpt: &Vec<&Inputable>) -> i64 {
        let x = self.x.eval(env.clone(), inpt);
        let y = self.y.eval(env.clone(), inpt);

        inpt[self.id].at(x, y)
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

struct Function1 {
    input: Box<Inputable>,
    e:     Box<Eval>
}

impl Inputable for Function1 {
    fn at(&self, x: i64, y: i64) -> i64 {
        if x < 0 || x >= self.input.width() {
            return 0
        }

        if y < 0 || y >= self.input.height() {
            return 0
        }

        let env = FunctionEnv {x: x, y: y};
        let inpts = vec![&*self.input];
        self.e.eval(env, &inpts)
    }

    fn width(&self) -> i64 {
        self.input.width()
    }

    fn height(&self) -> i64 {
        self.input.height()
    }
}

impl Function1 {
    pub fn new<F>(inpt: Box<Inputable>, gen: F) -> Self
        where F: Fn(VarRef, VarRef, usize) -> Box<Eval>
    {
        let x = VarRef {v: Var::X};
        let y = VarRef {v: Var::Y};

        let e = gen(x, y, 0);

        Function1 { e: e, input: inpt }
    }

    pub fn eval(&self) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let xbound = self.input.width();
        let ybound = self.input.height();

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

    pub fn gen_3x3_kernel(inpt: Box<Inputable>, k: [[i64; 3]; 3]) -> Self {
        Function1::new(inpt, |x, y, id| {
            let input = |x, y| {
                InputExpr {id: id, x: x, y: y }
            };

            let e =
                  (input(Box::new(x - 1), Box::new(y - 1)) * k[0][0])
                + (input(Box::new(x - 1), Box::new(y + 0)) * k[1][0])
                + (input(Box::new(x - 1), Box::new(y + 1)) * k[2][0])
                + (input(Box::new(x + 0), Box::new(y - 1)) * k[0][1])
                + (input(Box::new(x + 0), Box::new(y + 0)) * k[1][1])
                + (input(Box::new(x + 0), Box::new(y + 1)) * k[2][1])
                + (input(Box::new(x + 1), Box::new(y - 1)) * k[0][2])
                + (input(Box::new(x + 1), Box::new(y + 0)) * k[1][2])
                + (input(Box::new(x + 1), Box::new(y + 1)) * k[2][2]);

            Box::new(e)
        })
    }
}

struct Function2 {
    input1: Box<Inputable>,
    input2: Box<Inputable>,
    e:      Box<Eval>
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
}

impl Function2 {
    pub fn new<F>(input1: Box<Inputable>, input2: Box<Inputable>, gen: F) -> Self
        where F: Fn(VarRef, VarRef, usize, usize) -> Box<Eval>
    {
        let x = VarRef {v: Var::X};
        let y = VarRef {v: Var::Y};

        let e = gen(x, y, 0, 1);

        Function2 { e: e, input1: input1, input2: input2 }
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

    let id = Function1::new(Box::new(img), |x, y, input| {
        Box::new(InputExpr {id: input, x: Box::new(x), y: Box::new(y) })
    });

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
        let xx = x - 1;

        Box::new(InputExpr { id: input, x: Box::new(xx), y: Box::new(y) })
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
    let inpt = image::open(&Path::new("in1.png")).unwrap();

    let sobel_x = [[-1, 0, 1],
                   [-2, 0, 2],
                   [-1, 0, 1]];

    let sobel_x = Function1::gen_3x3_kernel(Box::new(inpt.to_luma()), sobel_x);

    let sobel_y = [[-1, -2, -1],
                   [ 0,  0,  0],
                   [ 1,  2,  1]];

    let sobel_y = Function1::gen_3x3_kernel(Box::new(inpt.to_luma()), sobel_y);

    let grad = Function2::new(Box::new(sobel_x), Box::new(sobel_y), |x, y, i1, i2| {
        let input1 = |x, y| {
            InputExpr {id: i1, x: x, y: y }
        };

        let input2 = |x, y| {
            InputExpr {id: i2, x: x, y: y }
        };

        let t1 = input1(Box::new(x), Box::new(y)) * input1(Box::new(x), Box::new(y));
        let t2 = input2(Box::new(x), Box::new(y)) * input2(Box::new(x), Box::new(y));

        Box::new(SqrtExpr { x: Box::new(t1 + t2) })
    });

    let out = grad.eval();

    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    let _ = image::ImageLuma8(out).save(fout, image::PNG);
}
