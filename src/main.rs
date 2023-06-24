mod disassembler;
mod chunk;
mod value;

use chunk::{Chunk, OpCode};
use disassembler::disassemble_chunk;

fn main() {
    let mut chunk = Chunk::new();
    let constant = chunk.add_constant(1.2);
    chunk.write(OpCode::Constant);
    chunk.write(constant as u8);
    disassemble_chunk(&chunk, "test chunk");
}
