#![feature(unboxed_closures)]

extern crate llvm;
extern crate image;

#[macro_use]
pub mod macros;
mod expression;
mod function;
mod source;

pub use expression::*;
pub use function::*;
pub use source::*;
