use crate::chunk::{OpCode, Chunk};

/// Disassemble all of the instructions in the entire chunk
pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {name} ==");
    let mut idx = 0;
    while idx < chunk.code.len() {
        idx = disassemble_instruction(chunk, idx);
    }
}

/// Disassemble a single instruction and return the offset of
/// the next instruction, as the instructions can have different sizes
fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{offset:04} ");
    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        // Show a | for any instruction that comes from the same source line as the preceding one.
        print!("  |  ");
    } else {
        print!("{:04} ", chunk.lines[offset]);
    }
    match chunk.code[offset].into() {
        OpCode::Return => simple_instruction("OP_RETURN", offset),
        OpCode::Constant => constant_instruction("OP_CONSTANT", chunk, offset),
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{name}");
    offset + 1
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant_idx = chunk.code[offset + 1];
    print!("{name:-16} {constant_idx:04} ");
    println!("'{:?}'", chunk.constants.values[constant_idx as usize]);
    offset + 2
}

