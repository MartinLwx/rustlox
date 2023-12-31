use crate::chunk::{Chunk, OpCode};
use crate::value::Value;

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
        print!("   | ");
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
        OpCode::Divide => simple_instruction("OP_DIVIDE", offset),
        OpCode::Nil => simple_instruction("OP_NIL", offset),
        OpCode::True => simple_instruction("OP_TRUE", offset),
        OpCode::False => simple_instruction("OP_FALE", offset),
        OpCode::Not => simple_instruction("OP_NOT", offset),
        OpCode::Equal => simple_instruction("OP_EQUAL", offset),
        OpCode::Greater => simple_instruction("OP_GREATER", offset),
        OpCode::Less => simple_instruction("OP_LESS", offset),
        OpCode::Print => simple_instruction("OP_PRINT", offset),
        OpCode::Pop => simple_instruction("OP_POP", offset),
        OpCode::DefineGlobal => constant_instruction("OP_DEFINE_GLOBAL", chunk, offset),
        OpCode::GetGlobal => constant_instruction("OP_GET_GLOBAL", chunk, offset),
        OpCode::SetGlobal => constant_instruction("OP_SET_GLOBAL", chunk, offset),
        OpCode::GetLocal => byte_instruction("OP_GET_LOCAL", chunk, offset),
        OpCode::SetLocal => byte_instruction("OP_SET_LOCAL", chunk, offset),
        OpCode::Jump => jump_instruction("OP_JUMP", 1, chunk, offset),
        OpCode::JumpIfFalse => jump_instruction("OP_JUMP_IF_ELSE", 1, chunk, offset),
        OpCode::Loop => jump_instruction("OP_LOOP", -1, chunk, offset),
        OpCode::Call => byte_instruction("OP_CALL", chunk, offset),
        OpCode::Closure => {
            let constant_idx = chunk.code[offset + 1];
            print!("{:-16} {:04} ", "OP_CLOSURE", constant_idx);
            let Value::Func(func) = &chunk.constants.values[constant_idx as usize] else {panic!("Impossible")};
            println!("'{func}'");

            for (idx, v) in func.upvalues.iter().enumerate() {
                println!(
                    "{:04}    |                       {} {}",
                    offset + idx + 1,
                    if v.is_local { "local" } else { "upvalue" },
                    v.index
                );
            }

            // offset
            offset + func.upvalues.len() * 2 + 2
        }
        OpCode::GetUpvalue => byte_instruction("OP_GET_UPVALUE", chunk, offset),
        OpCode::SetUpvalue => byte_instruction("OP_SET_UPVALUE", chunk, offset),
        OpCode::ClosedUpvalue => simple_instruction("OP_CLOSED_UPVALUE", offset),
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

/// The compiler compiles local variables to direct slot access, so we just show the slot number
fn byte_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let slot = chunk.code[offset + 1];
    println!("{name:-16} {slot:04} ");

    offset + 2
}

fn jump_instruction(name: &str, sign: i32, chunk: &Chunk, offset: usize) -> usize {
    // Compute the jump offset
    let mut jump = (chunk.code[offset + 1] as usize) << 8;
    jump |= chunk.code[offset + 2] as usize;
    let jump_target = if sign == 1 {
        offset + 3 + jump
    } else {
        offset + 3 - jump
    };

    println!("{name:-16} {offset:04} -> {jump_target}");

    offset + 3
}
