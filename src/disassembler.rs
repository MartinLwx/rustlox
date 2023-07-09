use crate::chunk::{Chunk, OpCode};

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
pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{offset:04} ");
    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        // Show a | for any instruction that comes from the same source line as the preceding one.
        print!("  |  ");
    } else {
        print!("{:4} ", chunk.lines[offset]);
    }
    match chunk.code[offset].into() {
        OpCode::Return => simple_instruction("OP_RETURN", offset),
        OpCode::Constant => constant_instruction("OP_CONSTANT", chunk, offset),
        OpCode::Negate => simple_instruction("OP_NEGATE", offset),
        OpCode::Add => simple_instruction("OP_ADD", offset),
        OpCode::Substract => simple_instruction("OP_SUBSTRACT", offset),
        OpCode::Multiply => simple_instruction("OP_MULTIPLY", offset),
        OpCode::Divide => simple_instruction("OP_DEVIDE", offset),
        OpCode::Nil => simple_instruction("OP_NIL", offset),
        OpCode::True => simple_instruction("OP_TRUE", offset),
        OpCode::False => simple_instruction("OP_FALE", offset),
        OpCode::Not => simple_instruction("OP_NOT", offset),
        OpCode::Equal => simple_instruction("OP_EQUAL", offset),
        OpCode::Greater => simple_instruction("OP_GREATER", offset),
        OpCode::Less => simple_instruction("OP_LESS", offset),
        OpCode::Print => simple_instruction("OP_PRINT", offset),
        OpCode::Pop => simple_instruction("OP_POP", offset),
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
