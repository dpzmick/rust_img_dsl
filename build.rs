use std::process::Command;
use std::env;

fn main() {
    let p = env::current_dir().unwrap();
    println!("The current directory is {}", p.display());

    Command::new("clang").args(&["src/core.c", "-S", "-emit-llvm", "-O0", "-c", "-o"])
                         .arg(&format!("{}/core.ll", p.display()))
                         .status().unwrap();

    Command::new("opt").args(&["core.ll", "-verify"])
                         .arg(&format!("-o={}/core.bc", p.display()))
                         .status().unwrap();
}
