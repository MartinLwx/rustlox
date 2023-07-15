use crate::chunk::OpCode;
use crate::compiler::Compiler;
use crate::disassembler::disassemble_instruction;
use crate::value::{Function, FunctionType, Value};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

#[derive(Debug)]
pub struct CallFrame {
    function: Function,
    ip: usize,
    slots: usize,
}

impl CallFrame {
    pub fn new(func: Function) -> Self {
        Self {
            function: func,
            ip: 0,
            slots: 0,
        }
    }
}

pub struct VM {
    pub frames: Vec<CallFrame>,

    pub stack: Vec<Value>,

    globals: HashMap<String, Value>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            frames: vec![],
            stack: vec![],
            globals: HashMap::new(),
        }
    }

    pub fn current_frame(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    /// Runs the chunk and then responds with a value
    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let compiler = Compiler::new(FunctionType::Script);
        let Ok(func) = compiler.compile(source) else {return InterpretResult::CompileError};
        self.frames.push(CallFrame::new(func));
        self.run()
    }

    /// Read the current byte pointed by `frame.ip` as an instruction and then advances the `self.ip`
    fn read_byte(&mut self) -> u8 {
        let frame = self.current_frame();
        frame.ip += 1;
        frame.function.chunk.code[frame.ip - 1]
    }

    /// Read a two bytes operand
    fn read_short(&mut self) -> u16 {
        let frame = self.current_frame();
        frame.ip += 2;
        let last_two = frame.function.chunk.code[frame.ip - 2] as u16;
        let last_one = frame.function.chunk.code[frame.ip - 1] as u16;

        (last_two << 8) | last_one
    }

    /// For a two bytes byte code: `[Opcode, the index of value]`, return the corresponding value
    fn read_constant(&mut self) -> Value {
        let frame = self.current_frame();
        let constant_idx = frame.function.chunk.code[frame.ip];
        frame.ip += 1;
        frame.function.chunk.constants.values[constant_idx as usize].clone()
    }

    fn binary_operator(&mut self, op: char) -> InterpretResult {
        if let (Some(b), Some(a)) = (self.stack.pop(), self.stack.pop()) {
            match (a, b) {
                (Value::Number(a), Value::Number(b)) => {
                    let val = match op {
                        '+' => Value::Number(a + b),
                        '-' => Value::Number(a - b),
                        '*' => Value::Number(a * b),
                        '/' => Value::Number(a / b),
                        '>' => Value::Bool(a > b),
                        '<' => Value::Bool(a < b),
                        _ => panic!("Impossible"),
                    };
                    self.stack.push(val);
                    InterpretResult::Ok
                }
                (Value::String(a), Value::String(b)) => {
                    self.stack.push(Value::String(format!("{a}{b}")));
                    InterpretResult::Ok
                }
                _ => {
                    self.runtime_error("Operands must be numbers.");
                    InterpretResult::RuntimeError
                }
            }
        } else {
            InterpretResult::RuntimeError
        }
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
    }

    fn runtime_error(&mut self, msg: &str) {
        // The VM advances past each instruction before executing it
        eprintln!("{msg}");
        let frame = self.current_frame();
        let instruction = frame.ip - frame.function.chunk.code.len() - 1;
        let line = frame.function.chunk.lines[instruction];
        eprintln!("[line {line}] in script");
        self.reset_stack()
    }

    /// Only `Nil` and `false` is falsey, everything else is `true`
    fn is_falsey(&self, value: &Value) -> bool {
        matches!(value, Value::Nil | Value::Bool(false))
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Nil, _) => true,
            (Value::Number(x), Value::Number(y)) => x == y,
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            _ => false,
        }
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
                disassemble_instruction(
                    &self.frames.last().unwrap().function.chunk,
                    self.frames.last().unwrap().ip,
                );
            }

            let instruction: OpCode = self.read_byte().into();
            match instruction {
                OpCode::Return => {
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
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::True => self.stack.push(Value::Bool(true)),
                OpCode::False => self.stack.push(Value::Bool(false)),
                OpCode::Not => {
                    if let Some(operand) = self.stack.pop() {
                        self.stack.push(Value::Bool(self.is_falsey(&operand)));
                    }
                }
                OpCode::Equal => {
                    if let (Some(b), Some(a)) = (self.stack.pop(), self.stack.pop()) {
                        self.stack.push(Value::Bool(self.values_equal(&a, &b)));
                    }
                }
                OpCode::Greater => {
                    self.binary_operator('>');
                }
                OpCode::Less => {
                    self.binary_operator('<');
                }
                OpCode::Print => {
                    // When the VM reaches this instruction, it has already executed the code for
                    // the expression, leaving the result value on top of the stack
                    println!("{}", self.stack.pop().unwrap());
                }
                OpCode::Pop => {
                    self.stack.pop().unwrap();
                }
                OpCode::DefineGlobal => {
                    // Get the name of the variable from the constant table
                    let name = self.read_constant();

                    if let Value::String(s) = name {
                        let val = self.stack.pop().unwrap();
                        self.globals.insert(s, val);
                    }
                }
                OpCode::GetGlobal => {
                    let name = self.read_constant();

                    if let Value::String(s) = name {
                        if self.globals.contains_key(&s) {
                            self.stack.push(self.globals.get(&s).unwrap().clone());
                        } else {
                            self.runtime_error(&format!("Undefined variable '{s}'"));
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::SetGlobal => {
                    let name = self.read_constant();

                    if let Value::String(s) = name {
                        // todo: avoid copy or look up the hashmap twice?
                        if let Entry::Occupied(mut e) = self.globals.entry(s.clone()) {
                            // Assignment is an expression, so it needs to leave that value there
                            // incase the assignment is nested inside some larger expression
                            let val = self.stack.last().unwrap().clone();
                            e.insert(val);
                        } else {
                            self.runtime_error(&format!("Undefined variable '{s}'"));
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::GetLocal => {
                    // It takes a single-byte operand for the stack slot where the local lives
                    let index = self.read_byte();
                    let slots_offset = self.current_frame().slots;

                    // Load the value from that index and then push it on top of the stack s.t.
                    // later instruction can find it
                    self.stack
                        .push(self.stack[index as usize + slots_offset].clone());
                }
                OpCode::SetLocal => {
                    // It taks a single-byte operand for the stack slot where the local lives
                    let index = self.read_byte();
                    let slots_offset = self.current_frame().slots;
                    self.stack[index as usize + slots_offset] = self.stack.last().unwrap().clone();
                }
                OpCode::JumpIfFalse => {
                    let offset = self.read_short();
                    if let Some(condition) = self.stack.last() {
                        if self.is_falsey(condition) {
                            self.frames.last_mut().unwrap().ip += offset as usize;
                        }
                    }
                }
                OpCode::Jump => {
                    let offset = self.read_short();
                    self.current_frame().ip += offset as usize;
                }
                OpCode::Loop => {
                    let offset = self.read_short();
                    self.current_frame().ip -= offset as usize;
                }
            }
        }
    }
}
