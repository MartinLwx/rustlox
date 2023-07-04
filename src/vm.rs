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
        self.ip = 0;
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

    fn binary_operator(&mut self, op: char) -> InterpretResult {
        if let (Value::Number(a), Value::Number(b)) = (
            self.stack[self.stack.len() - 2],
            self.stack[self.stack.len() - 1],
        ) {
            self.stack.pop();
            self.stack.pop();
            let val = match op {
                '+' => Value::Number(a + b),
                '-' => Value::Number(a - b),
                '*' => Value::Number(a * b),
                '/' => Value::Number(a / b),
                _ => panic!("Impossible"),
            };
            self.stack.push(val);
            InterpretResult::Ok
        } else {
            self.runtime_error("Operands must be numbers.");
            InterpretResult::RuntimeError
        }
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
    }

    fn runtime_error(&mut self, msg: &str) {
        // The VM advances past each instruction before executing it
        eprintln!("{msg}");
        let line = self.chunk.lines[self.ip - 1];
        eprintln!("[line {line}] in script");
        self.reset_stack()
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
                        if let Value::Number(v) = v {
                            self.stack.push(Value::Number(-v));
                        } else {
                            self.stack.push(v); // todo: shoule we cancel the previous pop
                                                // operation?
                            self.runtime_error("Operand must be a number.");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::Add => {
                    self.binary_operator('+');
                }
                OpCode::Substract => {
                    self.binary_operator('-');
                }
                OpCode::Multiply => {
                    self.binary_operator('*');
                }
                OpCode::Divide => {
                    self.binary_operator('/');
                }
            }
        }
    }
}
