use std::fmt::Debug;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

use std::marker::PhantomData;

use llvm;
use llvm::Compile;
use llvm::GetContext;

pub trait Expr: Debug {
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
pub struct FunctionEnv {
    x: i64,
    y: i64
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

// TODO use a macro to generate these
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
                   context: &'b llvm::Context,
                   module: &'b llvm::CSemiBox<'b, llvm::Module>,
                   builder: &'b llvm::Builder,
                   inputs: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        let v1 = self.e1.compile(x, y, context, module, builder, inputs);
        let v2 = self.e2.compile(x, y, context, module, builder, inputs);
        builder.build_add(v1, v2)
    }
}

impl<'a> Expr for MulExpr<'a> {
    fn compile<'b>(&self,
                   x: &'b llvm::Value,
                   y: &'b llvm::Value,
                   context: &'b llvm::Context,
                   module: &'b llvm::CSemiBox<'b, llvm::Module>,
                   builder: &'b llvm::Builder,
                   inputs: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        let v1 = self.e1.compile(x, y, context, module, builder, inputs);
        let v2 = self.e2.compile(x, y, context, module, builder, inputs);
        builder.build_mul(v1, v2)
    }
}

impl<'a> Expr for SqrtExpr<'a> {
    fn compile<'b>(&self,
                   x: &'b llvm::Value,
                   y: &'b llvm::Value,
                   _: &'b llvm::Context,
                   module: &'b llvm::CSemiBox<'b, llvm::Module>,
                   builder: &'b llvm::Builder,
                   inputs: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
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

impl<'a> Expr for InputExpr<'a> {
    fn compile<'b>(&self,
                   x: &'b llvm::Value,
                   y: &'b llvm::Value,
                   context: &'b llvm::Context,
                   module: &'b llvm::CSemiBox<'b, llvm::Module>,
                   builder: &'b llvm::Builder,
                   inputs: &Vec<&'b llvm::Function>)
        -> &'b llvm::Value
    {
        let x = self.x.compile(x, y, context, module, builder, inputs);
        let y = self.y.compile(x, y, context, module, builder, inputs);
        builder.build_call(inputs[self.id], &[x, y])
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
