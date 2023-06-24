///  Operation code for the Lox
#[derive(Debug)]
#[repr(u8)]
pub enum OpCode {
    /// Return from the current function
    Return,
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
            _ => unimplemented!("May be later")
        }
    }
}

/// A chunk is a series of instrucitons
#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: vec![],
        }
    }
    pub fn write<T>(&mut self, byte: T)
    where T: Into<u8> {
        self.code.push(byte.into());
    }
}
