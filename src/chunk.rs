use crate::value::{Value, ValueArray};

///  Operation code for the Lox
#[derive(Debug)]
#[repr(u8)]
pub enum OpCode {
    /// Return from the current function
    Return,
    /// Produce a particular constant
    Constant,
    /// Negate a value
    Negate,
    Add,
    Substract,
    Multiply,
    Divide,
}

impl From<OpCode> for u8 {
    fn from(value: OpCode) -> Self {
        value as u8
    }
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Return,
            1 => Self::Constant,
            2 => Self::Negate,
            3 => Self::Add,
            4 => Self::Substract,
            5 => Self::Multiply,
            6 => Self::Divide,
            _ => unimplemented!("May be later"),
        }
    }
}

/// A chunk is a series of instrucitons
#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ValueArray,
    pub lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: vec![],
            constants: ValueArray::new(),
            lines: vec![],
        }
    }

    pub fn write<T>(&mut self, byte: T, line: usize)
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
