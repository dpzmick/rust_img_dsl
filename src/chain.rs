use function::*;
use Img;

use llvm;
use llvm::GetContext;
use llvm::Compile;

// whatever let it be pub WHO CARES
pub enum ChainLink<'a> {
    ImageSource(&'a Img),
    Linked(Vec<&'a ChainLink<'a>>, &'a Function <'a>)
}

impl<'a> ChainLink<'a> {
    pub fn image_source(img: &'a Img) -> Self {
        ChainLink::ImageSource(img)
    }

    pub fn link(inputs: Vec<&'a ChainLink>, to: &'a Function<'a>) -> Self {
        // TODO check that all inputs have the same dimensions
        ChainLink::Linked(inputs.to_vec(), to)
    }

    /// the emitted module will use the provided images, therefore lifetime must be same as
    /// lifetime of the images
    pub fn compile(&self) -> llvm::CSemiBox<'a, llvm::Module> {
        let c = unsafe {
            llvm::Context::get_global()
        };

        let module = llvm::Module::new("jitmodule", c);
        {
            let topfunc = self.compile_into(&module);
        }
        module
    }

    fn compile_into(&self, module: &'a llvm::CSemiBox<'a, llvm::Module>) -> &'a llvm::Function {
        match self {
            &ChainLink::ImageSource(img) => compile_image_to_llvm_function(module, img),
            &ChainLink::Linked(ref links, ref func) => {
                let funs = links.iter().map(|f| f.compile_into(module)).collect();
                compile_function_to_llvm_function(module, func, &funs)
            }
        }
    }

    fn width(&self) -> i64 {
        match self {
            &ChainLink::ImageSource(img) => img.width() as i64,
            &ChainLink::Linked(ref links, _) => {
                links[0].width()
            }
        }
    }

    fn height(&self) -> i64 {
        match self {
            &ChainLink::ImageSource(img) => img.height() as i64,
            &ChainLink::Linked(ref links, _) => {
                links[0].height()
            }
        }
    }
}

/// helper function which finds or adds a function in/to an llvm module which will access a single
/// pixel of a rust image
/// TODO find existing function
fn compile_image_to_llvm_function<'a>(module: &'a llvm::CSemiBox<'a, llvm::Module>,
                                      img: &'a Img)
    -> &'a llvm::Function
{
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

fn compile_function_to_llvm_function<'a>(module: &'a llvm::CSemiBox<'a, llvm::Module>,
                                         func: &'a Function,
                                         inputs: &Vec<&'a llvm::Function>)
    -> &'a llvm::Function
{
    let context = module.get_context();
    let builder = llvm::Builder::new(&context);

    let ft = llvm::Type::get::<fn(i64, i64) -> i64>(module.get_context());
    let f = module.add_function("function", ft); // totally unreadable
    f.add_attribute(llvm::Attribute::AlwaysInline);

    let entry = f.append("entry");
    builder.position_at_end(entry);

    let x = &f[0];
    let y = &f[1];

    let e = func.get_expr();
    let e = e.compile(x, y, context, module, &*builder, inputs);
    builder.build_ret(e);

    f
}

// compilation strategy: one function per Function. each function (x,y) -> result
// will emit pretty inefficient code that the optimizer can't do much about, but that's okay
// try to keep as much as possible in this file
