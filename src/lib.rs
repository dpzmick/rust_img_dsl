#![feature(unboxed_closures)]

extern crate llvm;
extern crate image;

use image::Luma;
use image::ImageBuffer;
pub type Img = ImageBuffer<Luma<u8>, Vec<u8>>;

#[macro_use]
pub mod macros;
mod expression;
mod function;
mod chain;

pub use expression::*;
pub use function::*;
pub use chain::*;
