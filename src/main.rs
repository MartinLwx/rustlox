mod chunk;
mod disassembler;
mod value;
mod vm;

use chunk::{Chunk, OpCode};
use vm::VM;

fn main() {
    let mut virtual_machine = VM::new();
    let mut chunk = Chunk::new();
    let constant = chunk.add_constant(1.2);
    chunk.write(OpCode::Constant, 0);
    chunk.write(constant as u8, 1);
    chunk.write(OpCode::Return, 2);
    virtual_machine.interpret(chunk);
}
