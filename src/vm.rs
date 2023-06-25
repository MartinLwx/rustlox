use crate::value::Value;
use crate::chunk::{Chunk, OpCode};
use crate::disassembler::disassemble_instruction;

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct VM {
    chunk: Chunk,

    /// `ip` = instruction pointer, which is also called  "PC". The `ip` always points to the next
    /// instruction
    pub ip: usize,
}

impl VM {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            ip: 0,
        }
    }

    /// Runs the chunk and then responds with a value
    pub fn interpret(&mut self, another_chunk: Chunk) -> InterpretResult {
        self.chunk = another_chunk;

        self.run()
    }

    /// Read the current bytepointed byte `self.ip` as an instruction and then advances the `self.ip`
    fn read_byte(&mut self) -> OpCode {
        self.ip += 1;
        self.chunk.code[self.ip - 1].into()
    }

    fn read_constant(&mut self) -> Value {
        let constant_idx = self.chunk.code[self.ip];
        self.ip += 1;
        self.chunk.constants.values[constant_idx as usize]
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            #[cfg(debug_assertions)]
            disassemble_instruction(&self.chunk, self.ip);

            let instruction = self.read_byte();
            match instruction {
                OpCode::Return => {
                    return InterpretResult::Ok;
                }
                OpCode::Constant => {
                    println!("'{:?}'", self.read_constant());
                }
            }
        }
    }
}
