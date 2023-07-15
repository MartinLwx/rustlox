use crate::chunk::Chunk;
#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    /// The number of parameters the function expects
    pub arity: usize,
    pub chunk: Chunk,
}

/// Let the compiler tell when it's compiling top-level code vs. the body of a function
#[derive(Debug, Default)]
pub enum FunctionType {
    Function,
    #[default]
    Script,
}

impl Function {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            arity: 0,
            chunk: Chunk::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    /// A pointer to a String in the heap
    String(String),
    Func(Function),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(v) => write!(f, "{v}"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::Nil => write!(f, "nil"),
            Self::String(s) => write!(f, "{s}"),
            Self::Func(func) => write!(
                f,
                "{}",
                if func.name.is_empty() {
                    "<script>"
                } else {
                    &func.name
                }
            ),
        }
    }
}

impl std::ops::Neg for Value {
    type Output = Self;
    fn neg(self) -> Self::Output {
        match self {
            Self::Number(v) => Self::Number(-v),
            _ => panic!("Impossible"),
        }
    }
}

impl std::ops::Add for Value {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Self::Number(a + b),
            _ => panic!("Impossible"),
        }
    }
}
impl std::ops::Sub for Value {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Self::Number(a - b),
            _ => panic!("Impossible"),
        }
    }
}

impl std::ops::Div for Value {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Self::Number(a / b),
            _ => panic!("Impossible"),
        }
    }
}

impl std::ops::Mul for Value {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Self::Number(a * b),
            _ => panic!("Impossible"),
        }
    }
}

// A list of the values that appear as literals in the program
#[derive(Clone, Debug)]
pub struct ValueArray {
    pub values: Vec<Value>,
}

impl ValueArray {
    pub fn new() -> Self {
        Self { values: vec![] }
    }
    pub fn write(&mut self, val: Value) {
        self.values.push(val);
    }
}
