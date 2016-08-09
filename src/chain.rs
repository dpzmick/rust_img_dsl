use Img;
use function::*;

use image::ImageBuffer;

use llvm::analysis::*;
use llvm::bit_reader::*;
use llvm::core::*;
use llvm::execution_engine::*;
use llvm::prelude::*;
use llvm::transforms::pass_manager_builder::*;
use llvm::transforms::ipo::*;
use llvm::target::*;
use llvm::*;

use std::mem;
// use std::ffi::CStr;

pub enum ChainLink<'a> {
    ImageSource(i64),
    Linked(Vec<&'a ChainLink<'a>>, &'a Function <'a>)
}

impl<'a> ChainLink<'a> {
    pub fn link(inputs: Vec<&'a ChainLink>, to: &'a Function<'a>) -> Self {
        ChainLink::Linked(inputs.to_vec(), to)
    }

    pub fn compile(&self) -> CompiledChain {
        unsafe {
            let context = LLVMGetGlobalContext();

            let mut mem_buffer = mem::uninitialized();
            let err            = mem::zeroed();

            let res = LLVMCreateMemoryBufferWithContentsOfFile(
                b"./core.bc".as_ptr() as *const _,
                &mut mem_buffer,
                err);
            assert!(res == 0);

            let mut module = mem::uninitialized();
            let res = LLVMParseBitcodeInContext(
                context,
                mem_buffer,
                &mut module,
                err);
            assert!(res == 0);
            LLVMDisposeMemoryBuffer(mem_buffer);

            let builder = LLVMCreateBuilderInContext(context);

            // get the predefined function out of the module
            let function = LLVMGetNamedFunction(module, b"function\0".as_ptr() as *const _);

            let bb = LLVMAppendBasicBlockInContext(
                context,
                function,
                b"entry\0".as_ptr() as *const _);

            LLVMPositionBuilderAtEnd(builder, bb);

            // compile the function and
            let ft = LLVMTypeOf(function);
            let ft = LLVMGetElementType(ft);
            let ftocall = self.compile_into(module, ft);

            let x          = LLVMGetParam(function, 0);
            let y          = LLVMGetParam(function, 1);
            let inputs     = LLVMGetParam(function, 2);
            let num_inputs = LLVMGetParam(function, 3);

            let mut args = [x, y, inputs, num_inputs];

            let res  = LLVMBuildCall(
                builder,
                ftocall,
                args.as_mut_ptr(),
                args.len() as ::libc::c_uint,
                b"call\0".as_ptr() as *const _);

            LLVMBuildRet(builder, res);

            // for every function, add the inline always attribute
            let mut fun = LLVMGetFirstFunction(module);
            while !fun.is_null() {
                // println!("adding always inline to function {}",
                //        CStr::from_ptr(LLVMGetValueName(fun)).to_str().unwrap());

                LLVMAddFunctionAttr(fun, LLVMAlwaysInlineAttribute);
                fun = LLVMGetNextFunction(fun);
            }

            // LLVMPrintModuleToFile(module, b"out_preopt.ll\0".as_ptr() as *const _, err);
            // LLVMVerifyModule(module, LLVMVerifierFailureAction::LLVMAbortProcessAction, err);

            // optimize the module
            let builder = LLVMPassManagerBuilderCreate();
            LLVMPassManagerBuilderSetOptLevel(builder, 3 as ::libc::c_uint);
            LLVMPassManagerBuilderSetSizeLevel(builder, 0 as ::libc::c_uint);

            let pass_manager = LLVMCreatePassManager();
            // we can use the always inline pass because everything is marked alwaysinline
            // LLVMPassManagerBuilderUseInlinerWithThreshold(builder, 3 as ::libc::c_uint);
            LLVMAddAlwaysInlinerPass(pass_manager);
            LLVMPassManagerBuilderPopulateModulePassManager(builder, pass_manager);
            LLVMPassManagerBuilderDispose(builder);
            LLVMRunPassManager(pass_manager, module);

            // LLVMPrintModuleToFile(module, b"out_postop.ll\0".as_ptr() as *const _, err);
            // LLVMVerifyModule(module, LLVMVerifierFailureAction::LLVMAbortProcessAction, err);

            // create a MCJIT execution engine
            LLVMLinkInMCJIT();
            assert!(0 == LLVM_InitializeNativeTarget());
            assert!(0 == LLVM_InitializeNativeAsmPrinter());

            // takes ownership of module
            let mut ee = mem::uninitialized();
            LLVMCreateExecutionEngineForModule(&mut ee, module, err);
            // TODO check for err

            CompiledChain { engine: ee }
        }
    }

    fn compile_into(&self, module: LLVMModuleRef, ft: LLVMTypeRef) -> LLVMValueRef
    {
        match self {
            &ChainLink::ImageSource(idx) =>
                unsafe { compile_image_src_to_llvm_function(module, idx, ft) },

            &ChainLink::Linked(ref links, ref func) => {
                let funs = links.iter().map(|f| f.compile_into(module, ft)).collect();
                unsafe { compile_function_to_llvm_function(module, func, ft, &funs) }
            }
        }
    }
}

// create a function that will call the core function with the appropriate index
unsafe fn compile_image_src_to_llvm_function(
    module: LLVMModuleRef,
    idx: i64,
    ft: LLVMTypeRef) -> LLVMValueRef
{
    let context = LLVMGetModuleContext(module);
    let builder = LLVMCreateBuilderInContext(context);
    let f = LLVMAddFunction(module, b"image_source\0".as_ptr() as *const _, ft);

    let bb = LLVMAppendBasicBlockInContext(
        context,
        f,
        b"entry\0".as_ptr() as *const _);

    LLVMPositionBuilderAtEnd(builder, bb);

    let x          = LLVMGetParam(f, 0);
    let y          = LLVMGetParam(f, 1);
    let inputs     = LLVMGetParam(f, 2);
    let num_inputs = LLVMGetParam(f, 3);

    // get type of idx
    let idx = LLVMConstInt(
        LLVMInt64TypeInContext(context),
        idx as ::libc::c_ulonglong,
        1);

    let core_f = LLVMGetNamedFunction(module, b"core_input_at\0".as_ptr() as *const _);
    assert!(core_f as usize != 0);

    let mut args = [x, y, inputs, num_inputs, idx];

    let res  = LLVMBuildCall(
        builder,
        core_f,
        args.as_mut_ptr(),
        args.len() as ::libc::c_uint,
        b"call\0".as_ptr() as *const _);

    let _    = LLVMBuildRet(builder, res);

    f
}

// pass in the function type that should be used
unsafe fn compile_function_to_llvm_function<'a>(
    module: LLVMModuleRef,
    func: &'a Function,
    ft: LLVMTypeRef,
    fun_inputs: &Vec<LLVMValueRef>) -> LLVMValueRef
{
    let context = LLVMGetModuleContext(module);
    let builder = LLVMCreateBuilderInContext(context);

    let f = LLVMAddFunction(module, b"compiled_fn\0".as_ptr() as *const _, ft);

    let bb = LLVMAppendBasicBlockInContext(
        context,
        f,
        b"entry\0".as_ptr() as *const _);

    LLVMPositionBuilderAtEnd(builder, bb);

    let x          = LLVMGetParam(f, 0);
    let y          = LLVMGetParam(f, 1);
    let inputs     = LLVMGetParam(f, 2);
    let num_inputs = LLVMGetParam(f, 3);

    let e = func.get_expr();
    let e = e.compile(x, y, inputs, num_inputs, context, module, builder, fun_inputs);

    LLVMBuildRet(builder, e);

    f
}

/// holds a compiled chain
/// this actually holds an llvm execution engine eagerly waiting to execute your jit compiled
/// function
pub struct CompiledChain {
    pub engine: LLVMExecutionEngineRef
}
// TODO implement drop

impl CompiledChain {
    /// will fail with assertion failure if inputs not all same dimensions
    pub fn run_on(&self, inputs: &[&Img]) -> Img {
        assert!(inputs.len() >= 1);

        let dim = inputs[0].dimensions();

        // for &i in inputs {
        //     assert!(i.dimensions() == dim);
        // }

        let out = ImageBuffer::new(dim.0, dim.1);
        let outptr = out.as_ptr() as *const i8;

        let inptsptrs: Vec<*const i8>
            = inputs.iter().map(|i| i.as_ptr() as *const i8).collect();

        // pretend everything is const, but we know it actually isn't :)
        let func: fn(i64, i64, *const i8, *const *const i8, u64) -> () = unsafe {
            let f = LLVMGetFunctionAddress(self.engine, b"jitfunction\0".as_ptr() as *const _);
            mem::transmute(f)
        };

        let width: i64  = out.width() as i64;
        let height: i64 = out.height() as i64;
        func(width, height, outptr, inptsptrs.as_ptr(), inputs.len() as u64);

        out
    }
}

// compilation strategy: one function per Function. each function (x,y) -> result
// will emit pretty inefficient code that the optimizer can't do much about, but that's okay
// try to keep as much as possible in this file
