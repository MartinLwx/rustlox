use crate::chunk::OpCode;
use crate::compiler::Compiler;
use crate::disassembler::disassemble_instruction;
use crate::value::{Closure, FunctionType, NativeFunction, Value};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

#[derive(Debug)]
pub struct CallFrame {
    closure: Rc<Closure>,
    ip: usize,
    /// The starts position of this CallFrame in the VM's stack
    slots: usize,
}

impl CallFrame {
    pub fn new(closure: Rc<Closure>, ip: usize, slots: usize) -> Self {
        Self { closure, ip, slots }
    }
}

fn clock(_args: &[Value]) -> Value {
    // see: https://stackoverflow.com/questions/26593387/how-can-i-get-the-current-time-in-milliseconds
    let since_the_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    Value::Number(since_the_epoch.as_secs_f64())
}

pub struct VM {
    pub frames: Vec<CallFrame>,

    pub stack: Vec<Value>,

    globals: HashMap<String, Value>,
}

impl VM {
    pub fn new() -> Self {
        let mut vm = Self {
            frames: vec![],
            stack: vec![],
            globals: HashMap::new(),
        };
        vm.define_native("clock", NativeFunction(clock));
        vm
    }

    pub fn current_frame(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    /// Runs the chunk and then responds with a value
    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let compiler = Compiler::new(FunctionType::Script);
        let Ok(func) = compiler.compile(source) else {return InterpretResult::CompileError};
        self.frames.push(CallFrame::new(
            Rc::new(Closure::new(Rc::new(func), None)),
            0,
            0,
        ));
        self.run()
    }

    /// Read the current byte pointed by `frame.ip` as an instruction and then advances the `self.ip`
    fn read_byte(&mut self) -> u8 {
        let frame = self.current_frame();
        frame.ip += 1;
        frame.closure.function.chunk.code[frame.ip - 1]
    }

    /// Read a two bytes operand
    fn read_short(&mut self) -> u16 {
        let frame = self.current_frame();
        frame.ip += 2;
        let last_two = frame.closure.function.chunk.code[frame.ip - 2] as u16;
        let last_one = frame.closure.function.chunk.code[frame.ip - 1] as u16;

        (last_two << 8) | last_one
    }

    /// For a two bytes byte code: `[Opcode, the index of value]`, return the corresponding value
    fn read_constant(&mut self) -> Value {
        let frame = self.current_frame();
        let constant_idx = frame.closure.function.chunk.code[frame.ip];
        frame.ip += 1;
        frame.closure.function.chunk.constants.values[constant_idx as usize].clone()
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

        // print stack trace
        for frame in self.frames.iter().rev() {
            let instruction = frame.ip - 1;
            let line = frame.closure.function.chunk.lines[instruction];
            eprintln!(
                "[line {}] in {}",
                line,
                if frame.closure.function.name.is_empty() {
                    "<script>"
                } else {
                    &frame.closure.function.name
                }
            );
        }
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

    /// Create a new CallFrame and push it to `self.frames`
    fn call(&mut self, closure: Rc<Closure>, arg_cnt: u8) -> bool {
        if arg_cnt as usize != closure.function.arity {
            self.runtime_error(&format!(
                "Expected {} arguments but got {}.",
                closure.function.arity, arg_cnt,
            ));
            return false;
        }
        // the starts slots DOES NOT include the function name in the stack
        self.frames.push(CallFrame::new(
            closure,
            0,
            self.stack.len() - arg_cnt as usize,
        ));

        true
    }

    fn call_value(&mut self, arg_cnt: u8) -> bool {
        // todo: can we avoid the cloning overhead?
        //       how to solve the ownership issue?
        let callee = self.stack[self.stack.len() - 1 - arg_cnt as usize].clone();
        match callee {
            Value::NativeFunc(fp) => {
                let arg_start = self.stack.len() - arg_cnt as usize;
                let result = fp.0(&self.stack[arg_start..]);
                self.stack.truncate(arg_start - 1);
                self.stack.push(result);
                true
            }
            Value::Closure(closure) => self.call(closure, arg_cnt),
            _ => {
                self.runtime_error("Can only call functions and classes.");
                false
            }
        }
    }

    /// `fp` is a function pointer
    fn define_native(&mut self, name: &str, fp: NativeFunction) {
        self.globals.insert(name.to_string(), Value::NativeFunc(fp));
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
                    &self.frames.last().unwrap().closure.function.chunk,
                    self.frames.last().unwrap().ip,
                );
            }

            let instruction: OpCode = self.read_byte().into();
            match instruction {
                OpCode::Return => {
                    let result = self.stack.pop().unwrap();
                    let return_addr = self.current_frame().slots.saturating_sub(1);
                    self.frames.pop().unwrap();
                    // It means we have finished executing the top-level code
                    // , then we exit the VM
                    if self.frames.is_empty() {
                        return InterpretResult::Ok;
                    }

                    self.stack.truncate(return_addr);

                    // The return value of the callee
                    self.stack.push(result);
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
                            // todo: copying function object may be inefficient here, should we
                            // avoid the clone() here?
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
                OpCode::Call => {
                    let arg_cnt = self.read_byte();
                    // Do not decide callee here because the ownership issue
                    // let callee = &self.stack[self.stack.len() - 1 - arg_cnt as usize];
                    if !self.call_value(arg_cnt) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::Closure => {
                    let Value::Func(func) = self.read_constant() else {panic!("Impossible");};
                    let rc_closure = Rc::new(Closure::new(func, None));
                    self.stack.push(Value::Closure(rc_closure));
                }
            }
        }
    }
}
