use llvm;

use llvm::Compile;
use llvm::GetContext;

use image::ImageBuffer;
use image::Luma;

/// helper function which finds or adds a function in/to an llvm module which will access a single
/// pixel of a rust image
/// TODO find existing function
pub fn compile_image_to_llvm_function<'a>(module: &'a llvm::CSemiBox<'a, llvm::Module>,
                                          img: &ImageBuffer<Luma<u8>, Vec<u8>>)
    -> &'a llvm::Function
{
    // have to build a new array UGH, the input array is all i8s
    let context = module.get_context();
    let builder = llvm::Builder::new(&context);

    let ft = llvm::Type::get::<fn(i64, i64) -> i64>(module.get_context());
    let f = module.add_function("image_source", ft);
    f.add_attribute(llvm::Attribute::AlwaysInline);

    let entry = f.append("entry");
    builder.position_at_end(entry);

    let x = &f[0];
    let y = &f[1];
    let w = (img.width() as i64).compile(&context);

    let offset = builder.build_mul(w, y);
    let offset = builder.build_add(offset, x);

    let ty0 = llvm::Type::get::<i8>(&context);
    let ty1 = llvm::Type::new_pointer(ty0);

    let ptr = (img.as_ptr() as u64).compile(&context);
    let ptr = builder.build_int_to_ptr(ptr, ty1);
    let ptr = builder.build_gep(ptr, &[offset]);
    let e = builder.build_load(ptr);

    let e = builder.build_zext(e, llvm::Type::get::<i64>(&context));
    builder.build_ret(e);

    f
}

pub trait ChainableSource {
    // this should be all that is needed to implement this method
    fn run_on_image_inputs(&self, &[ImageBuffer<Luma<u8>, Vec<u8>>])
        -> ImageBuffer<Luma<u8>, Vec<u8>>;
}
