mod chunk;
mod compiler;
mod disassembler;
mod scanner;
mod value;
mod vm;

use std::{fs, io, io::Read, process};
use vm::{InterpretResult, VM};

fn repl(vm: &mut VM) {
    let mut line = String::new();
    loop {
        print!("> ");
        if let Ok(size) = io::stdin().read_line(&mut line) {
            println!();
            if size == 0 {
                break;
            }
        }
        vm.interpret(&line);
    }
}

fn run_file(filename: &str, vm: &mut VM) {
    let Ok(mut file) = fs::File::open(filename) else {
        eprintln!("Could not open the file {filename} or not enough memory to read");
        process::exit(74);
    };
    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() {
        eprintln!("Could not read file {filename}");
        process::exit(74);
    }
    match vm.interpret(&content) {
        InterpretResult::CompileError => process::exit(65),
        InterpretResult::RuntimeError => process::exit(70),
        InterpretResult::Ok => (),
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut virtual_machine = VM::new();

    match &args[1..] {
        [] => repl(&mut virtual_machine),
        [file] => run_file(file, &mut virtual_machine),
        _ => eprintln!("Usage: clox [path]"),
    }
}
