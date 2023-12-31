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
    Nil,
    True,
    False,
    Not,
    // Equality and comparison operators
    Equal,
    Greater,
    Less,
    Print,
    Pop,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    JumpIfFalse,
    /// Unconditional jump
    Jump,
    Loop,
    Call,
    Closure,
    SetUpvalue,
    GetUpvalue,
    ClosedUpvalue,
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
            7 => Self::Nil,
            8 => Self::True,
            9 => Self::False,
            10 => Self::Not,
            11 => Self::Equal,
            12 => Self::Greater,
            13 => Self::Less,
            14 => Self::Print,
            15 => Self::Pop,
            16 => Self::DefineGlobal,
            17 => Self::GetGlobal,
            18 => Self::SetGlobal,
            19 => Self::GetLocal,
            20 => Self::SetLocal,
            21 => Self::JumpIfFalse,
            22 => Self::Jump,
            23 => Self::Loop,
            24 => Self::Call,
            25 => Self::Closure,
            26 => Self::SetUpvalue,
            27 => Self::GetUpvalue,
            28 => Self::ClosedUpvalue,
            _ => unimplemented!("May be later"),
        }
    }
}

/// A chunk is a series of instrucitons
#[derive(Default, Clone, Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ValueArray,
    pub lines: Vec<usize>,
}

impl Chunk {
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
