use crate::value::{Value, ValueArray};

///  Operation code for the Lox
#[derive(Debug)]
#[repr(u8)]
pub enum OpCode {
    /// Return from the current function
    Return,
    /// Produce a particular constant
    Constant,
}

impl From<OpCode> for u8 {
    fn from(value: OpCode) -> Self {
        value as u8
    }
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        match value {
            0 => OpCode::Return,
            1 => OpCode::Constant,
            _ => unimplemented!("May be later"),
        }
    }
}

/// A chunk is a series of instrucitons
#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ValueArray,
    pub lines: Vec<i32>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: vec![],
            constants: ValueArray::new(),
            lines: vec![],
        }
    }

    pub fn write<T>(&mut self, byte: T, line: i32)
    where
        T: Into<u8>,
    {
        self.code.push(byte.into());
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, val: Value) -> usize {
        self.constants.write(val);
        self.constants.values.len() - 1
    }
}
