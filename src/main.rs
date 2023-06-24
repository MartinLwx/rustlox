mod disassembler;
mod chunk;
mod value;

use chunk::{Chunk, OpCode};
use disassembler::disassemble_chunk;

fn main() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return);
    disassemble_chunk(&chunk, "test chunk");
}
