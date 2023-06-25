mod disassembler;
mod chunk;
mod value;

use chunk::{Chunk, OpCode};
use disassembler::disassemble_chunk;

fn main() {
    let mut chunk = Chunk::new();
    let constant = chunk.add_constant(1.2);
    chunk.write(OpCode::Return, 0);
    chunk.write(OpCode::Constant, 0);
    chunk.write(constant as u8, 0);
    chunk.write(OpCode::Return, 0);
    disassemble_chunk(&chunk, "test chunk");
}
