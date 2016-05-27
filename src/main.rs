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

// make a variable a simple type which has Copy
// we don't care about actual names in the code
type Var = u64;

#[derive(Debug)]
struct Function<'a> {
    e: Expression<'a>
}

impl<'a> Function<'a> {
    pub fn new<F>(gen: F) -> Self
        where F : Fn(Var,Var) -> Expression<'a> {
        let x: Var = 0;
        let y: Var = 1;

        let e = gen(x,y);

        Function { e: e }
    }
}

#[derive(Debug, Clone, Copy)]
struct FunctionEnv {
    x: i64,
    y: i64
}

#[derive(Debug, Clone)]
enum IdxExpression {
    Const(i64),
    VarRef(Var),
    Add(Box<IdxExpression>, Box<IdxExpression>),
    Sub(Box<IdxExpression>, Box<IdxExpression>),
    Mul(Box<IdxExpression>, Box<IdxExpression>)
}

impl Add for IdxExpression {
    type Output = IdxExpression;

    fn add(self, other: IdxExpression) -> IdxExpression {
        IdxExpression::Add(Box::new(self), Box::new(other))
    }
}

impl Mul for IdxExpression {
    type Output = IdxExpression;

    fn mul(self, other: IdxExpression) -> IdxExpression {
        IdxExpression::Mul(Box::new(self), Box::new(other))
    }
}

impl Sub for IdxExpression {
    type Output = IdxExpression;

    fn sub(self, other: IdxExpression) -> IdxExpression {
        IdxExpression::Sub(Box::new(self), Box::new(other))
    }
}

fn interpret_idx(e: &IdxExpression, env: FunctionEnv) -> i64 {
    match e {
        &IdxExpression::Const(c) => c,

        &IdxExpression::VarRef(v) => if v == 0 { env.x } else { env.y },

        &IdxExpression::Add(ref b1, ref b2) =>
            interpret_idx(&*b1, env) + interpret_idx(&*b2, env),

        &IdxExpression::Mul(ref b1, ref b2) =>
            interpret_idx(&*b1, env) * interpret_idx(&*b2, env),

        &IdxExpression::Sub(ref b1, ref b2) =>
            interpret_idx(&*b1, env) - interpret_idx(&*b2, env),
    }
}

#[derive(Debug, Clone)]
enum Expression<'a> {
    Const(i64),
    Add(Box<Expression<'a>>, Box<Expression<'a>>),
    Sub(Box<Expression<'a>>, Box<Expression<'a>>),
    Mul(Box<Expression<'a>>, Box<Expression<'a>>),
    Call(&'a Function<'a>, Box<IdxExpression>, Box<IdxExpression>),
    InputImage(Box<IdxExpression>, Box<IdxExpression>)
}

impl<'a> Add for Expression<'a> {
    type Output = Expression<'a>;

    fn add(self, other: Expression<'a>) -> Expression<'a> {
        Expression::Add(Box::new(self), Box::new(other))
    }
}

impl<'a> Mul for Expression<'a> {
    type Output = Expression<'a>;

    fn mul(self, other: Expression<'a>) -> Expression<'a> {
        Expression::Mul(Box::new(self), Box::new(other))
    }
}

impl<'a> Sub for Expression<'a> {
    type Output = Expression<'a>;

    fn sub(self, other: Expression<'a>) -> Expression<'a> {
        Expression::Sub(Box::new(self), Box::new(other))
    }
}

// calls the function for every pixel in the image
fn interpret<C: Deref<Target=[u8]>>(f: &Function, inpt: ImageBuffer<Luma<u8>, C>)
    -> ImageBuffer<Luma<u8>, Vec<u8>>
{
    let mut out = ImageBuffer::new(inpt.width(), inpt.height());

    for x in 0..(inpt.width()) {
        for y in 0..(inpt.height()) {
            let e = Expression::Call(&f,
                                     Box::new(IdxExpression::Const(x as i64)),
                                     Box::new(IdxExpression::Const(y as i64)));

            let env = FunctionEnv {x: 0, y: 0};
            let p = interpret_help(&e, &inpt, env);

            let p = if p > 255 {
                255
            } else if p < 0 {
                0
            } else {
                p
            } as u8;

            let p = Luma::from_channels(p, 255, 255, 255);
            out.put_pixel(x, y, p);
        }
    }

    out
}

fn interpret_help<C: Deref<Target=[u8]>>(e: &Expression,
                                         inpt: &ImageBuffer<Luma<u8>, C>,
                                         env: FunctionEnv)
    -> i64
{
    let res = match e {
        &Expression::Const(x)            => x,

        &Expression::Add(ref b1, ref b2) =>
            interpret_help(&*b1, inpt, env) + interpret_help(&*b2, inpt, env),

        &Expression::Mul(ref b1, ref b2) =>
            interpret_help(&*b1, inpt, env) * interpret_help(&*b2, inpt, env),

        &Expression::Sub(ref b1, ref b2) =>
            interpret_help(&*b1, inpt, env) - interpret_help(&*b2, inpt, env),

        &Expression::Call(f, ref ba1, ref ba2)   => {
            let x = interpret_idx(&*ba1, env);
            let y = interpret_idx(&*ba2, env);

            let newenv = FunctionEnv {x:x, y:y};
            interpret_help(&f.e, inpt, newenv)
        },

        &Expression::InputImage(ref b1, ref b2) => {
            let idx1 = interpret_idx(&*b1, env.clone());
            let idx2 = interpret_idx(&*b2, env.clone());

            if     idx1 >= 0
                && idx1 < inpt.width() as i64
                && idx2 >= 0
                && idx2 < inpt.height() as i64
            {
                let (px, _, _, _) = inpt.get_pixel(idx1 as u32, idx2 as u32).channels4();
                px as i64
            } else {
                0
            }
        }
    };

    res
}

fn input<'a>(x: IdxExpression, y: IdxExpression) -> Expression<'a> {
    Expression::InputImage(Box::new(x), Box::new(y))
}

fn gen_apply_3x3_kernel<'a>(k: [[i64; 3]; 3]) -> Function<'a>{
    Function::new(|x, y| {
        let xx = IdxExpression::VarRef(x) - IdxExpression::Const(1);
        let yy = IdxExpression::VarRef(y) - IdxExpression::Const(1);
        let mut e = Expression::Const(k[0][0]) * input(xx, yy);

        let xx = IdxExpression::VarRef(x);
        let yy = IdxExpression::VarRef(y) - IdxExpression::Const(1);
        e = e + Expression::Const(k[0][1]) * input(xx, yy);

        let xx = IdxExpression::VarRef(x) + IdxExpression::Const(1);
        let yy = IdxExpression::VarRef(y) - IdxExpression::Const(1);
        e = e + Expression::Const(k[0][2]) * input(xx, yy);

        let xx = IdxExpression::VarRef(x) - IdxExpression::Const(1);
        let yy = IdxExpression::VarRef(y);
        e = e + Expression::Const(k[1][0]) * input(xx, yy);

        let xx = IdxExpression::VarRef(x);
        let yy = IdxExpression::VarRef(y);
        e = e + Expression::Const(k[1][1]) * input(xx, yy);

        let xx = IdxExpression::VarRef(x) + IdxExpression::Const(1);
        let yy = IdxExpression::VarRef(y);
        e = e + Expression::Const(k[1][2]) * input(xx, yy);

        let xx = IdxExpression::VarRef(x) - IdxExpression::Const(1);
        let yy = IdxExpression::VarRef(y) + IdxExpression::Const(1);
        e = e + Expression::Const(k[2][0]) * input(xx, yy);

        let xx = IdxExpression::VarRef(x);
        let yy = IdxExpression::VarRef(y) + IdxExpression::Const(1);
        e = e + Expression::Const(k[2][1]) * input(xx, yy);

        let xx = IdxExpression::VarRef(x) + IdxExpression::Const(1);
        let yy = IdxExpression::VarRef(y) + IdxExpression::Const(1);
        e + Expression::Const(k[2][2]) * input(xx, yy)
    })
}

#[test]
fn test_id() {
    let id = Function::new(|x, y| { input(IdxExpression::VarRef(x), IdxExpression::VarRef(y)) });

    let mut raw: Vec<u8> = Vec::new();
    raw.push(0);
    raw.push(1);
    raw.push(2);
    raw.push(3);

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(2, 2, raw).unwrap();
    let out = interpret(&id, img);

    let raw = out.into_raw();
    assert!(raw.len() == 4);
    assert!(raw[0] == 0);
    assert!(raw[1] == 1);
    assert!(raw[2] == 2);
    assert!(raw[3] == 3);
}

#[test]
fn test_shift_one() {
    let shift_one = Function::new(|x, y| {
        input(IdxExpression::VarRef(x) - IdxExpression::Const(1), IdxExpression::VarRef(y))
    });

    let mut raw: Vec<u8> = Vec::new();
    raw.push(0);
    raw.push(100);
    raw.push(200);
    raw.push(255);

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(2, 2, raw).unwrap();

    let out = interpret(&shift_one, img);

    let raw = out.into_raw();
    assert!(raw.len() == 4);
    assert!(raw[0] == 0);
    assert!(raw[1] == 0);
    assert!(raw[2] == 0);
    assert!(raw[3] == 200);
}

#[test]
fn test_simple_sobel() {
    println!("simple sobel");
    let inpt = [[0, 0, 0],
                [255, 255, 255],
                [0, 0, 0]];

    let mut raw: Vec<u8> = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            raw.push(inpt[x][y]);
        }
    }

    let k = [[-1, 0, 1],
             [-2, 0, 2],
             [-1, 0, 1]];

    let sobel_x = gen_apply_3x3_kernel(k);

    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(3, 3, raw).unwrap();
    let out = interpret(&sobel_x, img);
    let raw = out.into_raw();
    assert!(raw.len() == 9);

    let expected = [255, 0, 0, 255, 0, 0, 255, 0, 0];
    for i in 0..9 {
        assert!(raw[i] == expected[i]);
    }
}

fn main() {
    let k = [[-1, 0, 1],
             [-2, 0, 2],
             [-1, 0, 1]];

    let sobel_x = gen_apply_3x3_kernel(k);

    let inpt = image::open(&Path::new("in1.png")).unwrap();
    let _ = inpt.clone().to_luma().save("out_luma.png");

    let out = interpret(&sobel_x, inpt.to_luma());

    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    let _ = image::ImageLuma8(out).save(fout, image::PNG);
}
