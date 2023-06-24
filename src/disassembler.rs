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
    match chunk.code[offset].into() {
        OpCode::Return => simple_instruction("OP_RETURN", offset),
        _ => {
            println!("Unknown");
            offset + 1
        }
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{name}");
    offset + 1
}
