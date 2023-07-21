use crate::chunk::Chunk;
use std::rc::Rc;
#[derive(Default, Clone, Debug)]
pub struct Function {
    pub name: String,
    /// The number of parameters the function expects
    pub arity: usize,
    pub chunk: Chunk,
}

#[derive(Clone, Debug)]
pub struct Closure {
    pub function: Rc<Function>,
    obj: Option<Box<Value>>,
}

impl Closure {
    pub fn new(function: Rc<Function>, obj: Option<Box<Value>>) -> Self {
        Self { function, obj }
    }
}

#[derive(Clone)]
pub struct NativeFunction(pub fn(&[Value]) -> Value);

impl std::fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

/// Let the compiler tell when it's compiling top-level code vs. the body of a function
#[derive(PartialEq, Debug, Default)]
pub enum FunctionType {
    Function,
    #[default]
    Script,
}

#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    /// A pointer to a String in the heap
    String(String),
    Func(Rc<Function>),
    NativeFunc(NativeFunction),
    Closure(Rc<Closure>),
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
                "<fn {}>",
                if func.name.is_empty() {
                    "<script>"
                } else {
                    &func.name
                }
            ),
            Self::NativeFunc(..) => write!(f, "<native fn>"),
            Self::Closure(closure) => write!(f, "<closure {}>", closure.function.name),
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
#[derive(Default, Clone, Debug)]
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
