use llvm::core::*;
use llvm::prelude::*;

use std::fmt::Debug;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

use std::marker::PhantomData;

pub trait Expr: Debug {
    // this is complicated little beast
    unsafe fn compile(&self,
                      x:            LLVMValueRef,
                      y:            LLVMValueRef,
                      image_inputs: LLVMValueRef,
                      num_inputs:   LLVMValueRef,
                      context:      LLVMContextRef,
                      module:       LLVMModuleRef,
                      builder:      LLVMBuilderRef,
                      function_inputs: &Vec<LLVMValueRef>)
        -> LLVMValueRef;
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
    unsafe fn compile(&self,
                      _:       LLVMValueRef,
                      _:       LLVMValueRef,
                      _:       LLVMValueRef,
                      _:       LLVMValueRef,
                      context: LLVMContextRef,
                      _:       LLVMModuleRef,
                      _:       LLVMBuilderRef,
                      _:       &Vec<LLVMValueRef>)
        -> LLVMValueRef
    {
        LLVMConstInt(
            LLVMInt64TypeInContext(context),
            self.v as ::libc::c_ulonglong,
            1)
    }
}

impl<'a> Expr for VarRef<'a> {
    unsafe fn compile(&self,
                      x: LLVMValueRef,
                      y: LLVMValueRef,
                      _: LLVMValueRef,
                      _: LLVMValueRef,
                      _: LLVMContextRef,
                      _: LLVMModuleRef,
                      _: LLVMBuilderRef,
                      _: &Vec<LLVMValueRef>)
        -> LLVMValueRef
    {
        match self.v {
            Var::X => x,
            Var::Y => y
        }
    }
}

impl<'a> Expr for AddExpr<'a> {
    unsafe fn compile(&self,
                      x:            LLVMValueRef,
                      y:            LLVMValueRef,
                      image_inputs: LLVMValueRef,
                      num_inputs:   LLVMValueRef,
                      context:      LLVMContextRef,
                      module:       LLVMModuleRef,
                      builder:      LLVMBuilderRef,
                      function_inputs: &Vec<LLVMValueRef>)
        -> LLVMValueRef
    {
        let v1 = self.e1.compile(
            x, y, image_inputs, num_inputs, context, module, builder, function_inputs);

        let v2 = self.e2.compile(
            x, y, image_inputs, num_inputs, context, module, builder, function_inputs);

        LLVMBuildAdd(builder, v1, v2, b"add\0".as_ptr() as *const _)
    }
}

impl<'a> Expr for MulExpr<'a> {
    unsafe fn compile(&self,
                      x:            LLVMValueRef,
                      y:            LLVMValueRef,
                      image_inputs: LLVMValueRef,
                      num_inputs:   LLVMValueRef,
                      context:      LLVMContextRef,
                      module:       LLVMModuleRef,
                      builder:      LLVMBuilderRef,
                      function_inputs: &Vec<LLVMValueRef>)
        -> LLVMValueRef
    {
        let v1 = self.e1.compile(
            x, y, image_inputs, num_inputs, context, module, builder, function_inputs);

        let v2 = self.e2.compile(
            x, y, image_inputs, num_inputs, context, module, builder, function_inputs);

        LLVMBuildMul(builder, v1, v2, b"mul\0".as_ptr() as *const _)
    }
}

impl<'a> Expr for SqrtExpr<'a> {
    unsafe fn compile(&self,
                      x:            LLVMValueRef,
                      y:            LLVMValueRef,
                      image_inputs: LLVMValueRef,
                      num_inputs:   LLVMValueRef,
                      context:      LLVMContextRef,
                      module:       LLVMModuleRef,
                      builder:      LLVMBuilderRef,
                      function_inputs: &Vec<LLVMValueRef>)
        -> LLVMValueRef
    {
        let function = LLVMGetNamedFunction(module, b"core_isqrt\0".as_ptr() as *const _);

        // expression
        let val = self.x.compile(
            x, y, image_inputs, num_inputs, context, module, builder, function_inputs);

        let mut args = [val];
        LLVMBuildCall(
            builder,
            function,
            args.as_mut_ptr(),
            args.len() as ::libc::c_uint,
            b"call\0".as_ptr() as *const _)
    }
}

impl<'a> Expr for InputExpr<'a> {
    unsafe fn compile(&self,
                      x:            LLVMValueRef,
                      y:            LLVMValueRef,
                      image_inputs: LLVMValueRef,
                      num_inputs:   LLVMValueRef,
                      context:      LLVMContextRef,
                      module:       LLVMModuleRef,
                      builder:      LLVMBuilderRef,
                      function_inputs: &Vec<LLVMValueRef>)
        -> LLVMValueRef
    {
        let x = self.x.compile(
            x, y, image_inputs, num_inputs, context, module, builder, function_inputs);

        let y = self.y.compile(
            x, y, image_inputs, num_inputs, context, module, builder, function_inputs);

        let f = function_inputs[self.id];
        let mut args = [x, y, image_inputs, num_inputs];
        LLVMBuildCall(
            builder,
            f,
            args.as_mut_ptr(),
            args.len() as ::libc::c_uint,
            b"call\0".as_ptr() as *const _)
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
