use std::fmt::Debug;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

use std::marker::PhantomData;

use llvm;
use llvm::Compile;

pub trait Expr: Debug {
    // this is complicated little beast
    fn compile<'a>(&self,
                   x: &'a llvm::Value,
                   y: &'a llvm::Value,
                   image_inputs: &'a llvm::Value,
                   num_inputs: &'a llvm::Value,
                   context: &'a llvm::Context,
                   module: &'a llvm::CSemiBox<'a, llvm::Module>,
                   builder: &'a llvm::Builder,
                   function_inputs: &Vec<&'a llvm::Function>)
        -> &'a llvm::Value;
}

#[derive(Debug, Clone, Copy)]
pub enum Var { X, Y }

#[derive(Debug, Clone, Copy)]
pub struct ConstExpr<'a> {
    v: i64,
    p: PhantomData<&'a Expr>
}

#[derive(Debug, Clone, Copy)]
pub struct VarRef<'a> {
    v: Var,
    p: PhantomData<&'a Expr>
}

#[derive(Debug)]
pub struct InputExpr<'a> { id: usize, x: Box<Expr + 'a>, y: Box<Expr + 'a> }

#[derive(Debug)]
pub struct MulExpr<'a> { e1: Box<Expr + 'a>, e2: Box<Expr + 'a> }

#[derive(Debug)]
pub struct AddExpr<'a> { e1: Box<Expr + 'a>, e2: Box<Expr + 'a> }

#[derive(Debug)]
pub struct SqrtExpr<'a> { x: Box<Expr + 'a> }

impl<'a> ConstExpr<'a> {
    pub fn new(v: i64) -> Self {
        ConstExpr { v: v, p: PhantomData }
    }
}

impl<'a> VarRef<'a> {
    pub fn new(v: Var) -> Self {
        VarRef { v: v, p: PhantomData }
    }
}

impl<'a> InputExpr<'a> {
    pub fn new(id: usize, x: Box<Expr + 'a>, y: Box<Expr + 'a>) -> Self {
        InputExpr { id: id, x: x, y: y }
    }
}

impl<'a> SqrtExpr<'a> {
    pub fn new(x: Box<Expr + 'a>) -> Self { SqrtExpr{ x: x } }
}

impl<'a> Expr for ConstExpr<'a> {
    fn compile<'b>(&self,
                   _: &'b llvm::Value,
                   _: &'b llvm::Value,
                   _: &'b llvm::Value,
                   _: &'b llvm::Value,
                   context: &'b llvm::Context,
                   _: &'b llvm::CSemiBox<'b, llvm::Module>,
                   _: &'b llvm::Builder,
                   _: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        self.v.compile(context)
    }
}

impl<'a> Expr for VarRef<'a> {
    fn compile<'b>(&self,
                   x: &'b llvm::Value,
                   y: &'b llvm::Value,
                   _: &'b llvm::Value,
                   _: &'b llvm::Value,
                   _: &'b llvm::Context,
                   _: &'b llvm::CSemiBox<'b, llvm::Module>,
                   _: &'b llvm::Builder,
                   _: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        match self.v {
            Var::X => x,
            Var::Y => y
        }
    }
}

impl<'a> Expr for AddExpr<'a> {
    fn compile<'b>(&self,
                   x: &'b llvm::Value,
                   y: &'b llvm::Value,
                   inputs: &'b llvm::Value,
                   num_inputs: &'b llvm::Value,
                   context: &'b llvm::Context,
                   module: &'b llvm::CSemiBox<'b, llvm::Module>,
                   builder: &'b llvm::Builder,
                   function_inputs: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        let v1 = self.e1.compile(x, y, inputs, num_inputs,
                                 context, module, builder, function_inputs);

        let v2 = self.e2.compile(x, y, inputs, num_inputs,
                                 context, module, builder, function_inputs);

        builder.build_add(v1, v2)
    }
}

impl<'a> Expr for MulExpr<'a> {
    fn compile<'b>(&self,
                   x: &'b llvm::Value,
                   y: &'b llvm::Value,
                   inputs: &'b llvm::Value,
                   num_inputs: &'b llvm::Value,
                   context: &'b llvm::Context,
                   module: &'b llvm::CSemiBox<'b, llvm::Module>,
                   builder: &'b llvm::Builder,
                   function_inputs: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        let v1 = self.e1.compile(x, y, inputs, num_inputs,
                                 context, module, builder, function_inputs);

        let v2 = self.e2.compile(x, y, inputs, num_inputs,
                                 context, module, builder, function_inputs);

        builder.build_mul(v1, v2)
    }
}

impl<'a> Expr for SqrtExpr<'a> {
    fn compile<'b>(&self,
                   x: &'b llvm::Value,
                   y: &'b llvm::Value,
                   inputs: &'b llvm::Value,
                   num_inputs: &'b llvm::Value,
                   context: &'b llvm::Context,
                   module: &'b llvm::CSemiBox<'b, llvm::Module>,
                   builder: &'b llvm::Builder,
                   function_inputs: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        let f = module.get_function("core_isqrt").unwrap();

        // expression
        let v = self.x.compile(x, y, inputs, num_inputs,
                               context, module, builder, function_inputs);

        builder.build_call(f, &[v])
    }
}

impl<'a> Expr for InputExpr<'a> {
    fn compile<'b>(&self,
                   x: &'b llvm::Value,
                   y: &'b llvm::Value,
                   inputs: &'b llvm::Value,
                   num_inputs: &'b llvm::Value,
                   context: &'b llvm::Context,
                   module: &'b llvm::CSemiBox<'b, llvm::Module>,
                   builder: &'b llvm::Builder,
                   function_inputs: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        let x = self.x.compile(x, y, inputs, num_inputs,
                               context, module, builder, function_inputs);

        let y = self.y.compile(x, y, inputs, num_inputs,
                               context, module, builder, function_inputs);

        let f = function_inputs[self.id];
        builder.build_call(f, &[x, y, inputs, num_inputs])
    }
}

impl<'a> Add<i64> for Box<Expr +'a> {
    type Output = Box<Expr + 'a>;

    fn add(self, rhs: i64) -> Box<Expr + 'a> {
        let expr = AddExpr { e1: self, e2: Box::new(ConstExpr::new(rhs)) };
        Box::new(expr)
    }
}

impl<'a> Add<Box<Expr + 'a>> for i64 {
    type Output = Box<Expr + 'a>;

    fn add(self, rhs: Box<Expr + 'a>) -> Box<Expr + 'a> {
        let expr = AddExpr { e1: rhs, e2: Box::new(ConstExpr::new(self)) };
        Box::new(expr)
    }
}

impl<'a> Add<Box<Expr + 'a>> for Box<Expr + 'a> {
    type Output = Box<Expr + 'a>;

    fn add(self, rhs: Box<Expr + 'a>) -> Box<Expr + 'a> {
        let expr = AddExpr { e1: self, e2: rhs };
        Box::new(expr)
    }
}

impl<'a> Sub<i64> for Box<Expr +'a> {
    type Output = Box<Expr + 'a>;

    fn sub(self, rhs: i64) -> Box<Expr + 'a> {
        let expr = AddExpr { e1: self, e2: Box::new(ConstExpr::new(-rhs)) };
        Box::new(expr)
    }
}

impl<'a> Mul<i64> for Box<Expr + 'a> {
    type Output = Box<Expr + 'a>;

    fn mul(self, rhs: i64) -> Box<Expr + 'a> {
        Box::new(MulExpr { e1: self, e2: Box::new(ConstExpr::new(rhs)) })
    }
}

impl<'a> Mul<Box<Expr + 'a>> for i64 {
    type Output = Box<Expr + 'a>;

    fn mul(self, rhs: Box<Expr + 'a>) -> Box<Expr + 'a> {
        Box::new(MulExpr { e1: rhs, e2: Box::new(ConstExpr::new(self)) })
    }
}

impl<'a> Mul<Box<Expr + 'a>> for Box<Expr + 'a> {
    type Output = Box<Expr + 'a>;

    fn mul(self, rhs: Box<Expr + 'a>) -> Box<Expr + 'a> {
        Box::new(MulExpr { e1: self, e2: rhs })
    }
}
