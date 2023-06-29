use crate::chunk::{Chunk, OpCode};
use crate::compiler::Compiler;
use crate::disassembler::disassemble_instruction;
use crate::value::Value;

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

    pub stack: Vec<Value>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            ip: 0,
            stack: vec![],
        }
    }

    /// Runs the chunk and then responds with a value
    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let mut chunk = Chunk::new();
        let mut compiler = Compiler::new(&mut chunk);
        compiler.compile(source);
        self.chunk = chunk;
        println!("RUNNN");
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

    fn binary_operator<F>(&mut self, op: F)
    where
        F: Fn(Value, Value) -> Value,
    {
        // use Fn s.t. we can pass either a closure or a function pointer(fn)
        let val2 = self.stack.pop().unwrap();
        let val1 = self.stack.pop().unwrap();
        self.stack.push(op(val1, val2))
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            // stack tracing - show the current contents of the stack before we interpret each
            // instruction
            #[cfg(debug_assertions)]
            {
                print!("          ");
                for val in &self.stack {
                    print!("[ {val} ]");
                }
                println!();
                disassemble_instruction(&self.chunk, self.ip);
            }

            let instruction = self.read_byte();
            match instruction {
                OpCode::Return => {
                    println!("{}", self.stack.last().expect("Empty stack in VM"));
                    return InterpretResult::Ok;
                }
                OpCode::Constant => {
                    let constant = self.read_constant();
                    self.stack.push(constant);
                }
                OpCode::Negate => {
                    if let Some(v) = self.stack.pop() {
                        self.stack.push(-v);
                    }
                }
                OpCode::Add => self.binary_operator(|x, y| x + y),
                OpCode::Substract => self.binary_operator(|x, y| x - y),
                OpCode::Multiply => self.binary_operator(|x, y| x * y),
                OpCode::Divide => self.binary_operator(|x, y| x / y),
            }
        }
    }
}
