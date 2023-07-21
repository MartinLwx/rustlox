use crate::chunk::{Chunk, OpCode};
use crate::disassembler::disassemble_chunk;
use crate::scanner::{Scanner, Token, TokenType};
use crate::value::{Closure, Function, FunctionType, Value};
use crate::vm::InterpretResult;
use std::rc::Rc;

#[derive(Debug, Default)]
struct Parser {
    current: Token,
    previous: Token,
    had_error: bool,
    panic_mode: bool,
}

#[derive(PartialEq, PartialOrd)]
enum Precedence {
    None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Precedence {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Assignment,
            Self::Assignment => Self::Or,
            Self::Or => Self::And,
            Self::And => Self::Equality,
            Self::Equality => Self::Comparison,
            Self::Comparison => Self::Term,
            Self::Term => Self::Factor,
            Self::Factor => Self::Unary,
            Self::Unary => Self::Call,
            Self::Call => Self::Primary,
            Self::Primary => panic!("Impossible"),
        }
    }
}

/// A function type that takes no arguments and returns nothing
type ParseFn = fn(&mut Compiler, bool) -> (); // function pointer

/// The three properties which represents a single row in the Pratt parser table
struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Precedence,
}

impl ParseRule {
    fn get_rule(op_type: TokenType) -> ParseRule {
        match op_type {
            TokenType::LeftParen => ParseRule {
                prefix: Some(Compiler::grouping),
                infix: Some(Compiler::call),
                precedence: Precedence::Call,
            },
            TokenType::Minus => ParseRule {
                prefix: Some(Compiler::unary),
                infix: Some(Compiler::binary),
                precedence: Precedence::Term,
            },
            TokenType::Plus => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Term,
            },
            TokenType::Slash | TokenType::Star => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Factor,
            },
            TokenType::Number => ParseRule {
                prefix: Some(Compiler::number),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Nil | TokenType::True | TokenType::False => ParseRule {
                prefix: Some(Compiler::literal),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Bang => ParseRule {
                prefix: Some(Compiler::unary),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::BangEqual | TokenType::EqualEqual => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Equality,
            },
            TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Less
            | TokenType::LessEqual => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::STRING => ParseRule {
                prefix: Some(Compiler::string),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Identifier => ParseRule {
                prefix: Some(Compiler::variable),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::And => ParseRule {
                prefix: None,
                infix: Some(Compiler::and_),
                precedence: Precedence::And,
            },
            TokenType::Or => ParseRule {
                prefix: None,
                infix: Some(Compiler::or_),
                precedence: Precedence::Or,
            },
            _ => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        }
    }
}

/// A local variable in the stack
#[derive(Debug, Default)]
struct Local {
    name: Token,
    /// the level of nesting where this local variable was declared
    depth: i32,
}

impl Local {
    pub fn new(name: Token, depth: i32) -> Self {
        Self { name, depth }
    }
}

// To handle function declaration, we need to let the compiler reset the "state" but keep scanner
// and parser untouched. That's why I create this struct
#[derive(Default, Debug)]
struct CompilerState {
    enclosing: Option<Box<CompilerState>>,
    locals: Vec<Local>,
    scope_depth: i32,
    function: Function,
    function_type: FunctionType,
}

impl CompilerState {
    pub fn new(function_type: FunctionType) -> Self {
        Self {
            function_type,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct Compiler {
    scanner: Scanner,
    parser: Parser,
    state: CompilerState,
}

impl Compiler {
    pub fn new(function_type: FunctionType) -> Self {
        Self {
            scanner: Scanner::new(),
            parser: Parser::default(),
            state: CompilerState::new(function_type),
        }
    }

    fn error_at(&mut self, token: Token, msg: &str) {
        // While the panic mode flag is set, we simply suppress any other errors that get detected
        if self.parser.panic_mode {
            return;
        }
        self.parser.panic_mode = true;
        eprint!("[line {}] Error", token.line);
        match token.token_type {
            TokenType::Eof => eprint!(" at end"),
            TokenType::Error => eprint!(""),
            _ => eprint!(" at '{}'", token.lexeme),
        }
        eprintln!(": {msg}");
        self.parser.had_error = true;
    }

    /// Report an error at th location of the token we just consumed
    fn error(&mut self, msg: &str) {
        let token = std::mem::take(&mut self.parser.previous);
        self.error_at(token, msg);
    }

    fn error_at_current(&mut self, msg: &str) {
        let token = std::mem::take(&mut self.parser.current);
        self.error_at(token, msg);
    }

    fn advance(&mut self) {
        self.parser.previous = std::mem::take(&mut self.parser.current);

        // Keep looping, reading tokens and reporting the errors, until we hit a non-error one or
        // reach the end
        loop {
            self.parser.current = self.scanner.scan_token();
            // println!("prev:    {:?}", self.parser.previous);
            // println!("current: {:?}", self.parser.current);
            if self.parser.current.token_type != TokenType::Error {
                break;
            }
            // todo: can we avoid clone() here?
            self.error_at_current(&self.parser.current.lexeme.clone());
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn consume(&mut self, token_type: TokenType, msg: &str) {
        if self.parser.current.token_type == token_type {
            self.advance();
            return;
        }
        self.error_at_current(msg);
    }

    /// The current chunk refers to the chunk onwed by the function we're in the middle of
    /// compiling
    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.state.function.chunk
    }

    fn emit_byte<T>(&mut self, byte: T)
    where
        T: Into<u8>,
    {
        let lineno = self.parser.previous.line;
        self.current_chunk().write(byte.into(), lineno);
    }

    // A utlity function which write two bytes (one-byte Opcode + one-byte Operand)
    fn emit_bytes<T, U>(&mut self, byte1: T, byte2: U)
    where
        T: Into<u8>,
        U: Into<u8>,
    {
        self.emit_byte(byte1.into());
        self.emit_byte(byte2.into());
    }

    fn emit_constant(&mut self, value: Value) {
        let cosntant_idx = self.make_constant(value);
        self.emit_bytes(OpCode::Constant, cosntant_idx);
    }

    fn emit_return(&mut self) {
        // Lox will implicitly return nil
        self.emit_byte(OpCode::Nil);
        self.emit_byte(OpCode::Return);
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OpCode::Loop);

        // Jump backwards by a given offset
        // + 2 because we also need to consider the OP_LOOP instruction's own operands(2 bytes)
        let offset = self.current_chunk().code.len() - loop_start + 2;

        if offset > std::u16::MAX as usize {
            self.error("Loop body too large.");
        }

        // Jump offset - 2 bytes operand
        self.emit_byte((offset >> 8) as u8 & std::u8::MAX);
        self.emit_byte(offset as u8 & std::u8::MAX);
    }

    fn end_compiler(&mut self) -> Function {
        self.emit_return();

        #[cfg(debug_assertions)]
        {
            if !self.parser.had_error {
                let name = if self.state.function.name.is_empty() {
                    "<script>".to_string()
                } else {
                    self.state.function.name.clone()
                };
                disassemble_chunk(self.current_chunk(), &name);
            }
        }

        let ret_function = std::mem::take(&mut self.state.function);

        if self.state.enclosing.is_some() {
            self.state = *self.state.enclosing.take().unwrap();
        }

        ret_function
    }

    fn number(&mut self, _can_assign: bool) {
        let value: f64 = self.parser.previous.lexeme.parse().unwrap();
        self.emit_constant(Value::Number(value));
    }

    fn string(&mut self, _can_assign: bool) {
        let end = self.parser.previous.lexeme.len() - 2;
        // todo: or create a objects field for the Chunk struct
        self.emit_constant(Value::String(
            self.parser.previous.lexeme[1..=end].to_string(),
        ));
    }

    fn grouping(&mut self, _can_assign: bool) {
        // Assumption: the initial '(' has already been consumed
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn unary(&mut self, _can_assign: bool) {
        let operator_type = self.parser.previous.token_type.clone();

        // Compile the operand
        self.parse_precedence(Precedence::Unary);

        // Emit the operator instruction
        match operator_type {
            TokenType::Bang => self.emit_byte(OpCode::Not),
            TokenType::Minus => self.emit_byte(OpCode::Negate),
            _ => panic!("Unreachable!"),
        }
    }

    fn binary(&mut self, _can_assign: bool) {
        let operator_type = self.parser.previous.token_type.clone();
        let rule = ParseRule::get_rule(operator_type.clone());
        self.parse_precedence(rule.precedence.next());

        match operator_type {
            TokenType::Plus => self.emit_byte(OpCode::Add),
            TokenType::Minus => self.emit_byte(OpCode::Substract),
            TokenType::Star => self.emit_byte(OpCode::Multiply),
            TokenType::Slash => self.emit_byte(OpCode::Divide),
            TokenType::BangEqual => self.emit_bytes(OpCode::Equal, OpCode::Not),
            TokenType::EqualEqual => self.emit_byte(OpCode::Equal),
            TokenType::Greater => self.emit_byte(OpCode::Greater),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::Less, OpCode::Not),
            TokenType::Less => self.emit_byte(OpCode::Less),
            TokenType::LessEqual => self.emit_bytes(OpCode::Greater, OpCode::Not),
            _ => panic!("Unreachable!"),
        }
    }

    /// Return the number of arguments
    /// Each argument expression generates code that leaves its value on the stack
    fn argument_list(&mut self) -> u8 {
        let mut arg_cnt = 0;
        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();
                if arg_cnt == u8::MAX {
                    self.error("Can't have more than 255 arguments.");
                }
                arg_cnt += 1;
                if !self.my_match(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after arguments.");
        arg_cnt
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_cnt = self.argument_list();
        self.emit_bytes(OpCode::Call, arg_cnt);
    }

    fn literal(&mut self, _can_assign: bool) {
        // the parse_precedence function has already consumed the keyword token
        match self.parser.previous.token_type {
            TokenType::True => self.emit_byte(OpCode::True),
            TokenType::False => self.emit_byte(OpCode::False),
            TokenType::Nil => self.emit_byte(OpCode::Nil),
            _ => panic!("Unreachable!"),
        }
    }

    fn and_(&mut self, _can_assign: bool) {
        let end_jump = self.emit_jump(OpCode::JumpIfFalse);

        self.emit_byte(OpCode::Pop);
        self.parse_precedence(Precedence::And);

        self.patch_jump(end_jump);
    }

    fn or_(&mut self, _can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::JumpIfFalse);
        let end_jump = self.emit_jump(OpCode::Jump);

        self.patch_jump(else_jump);
        self.emit_byte(OpCode::Pop);

        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        // Read the next token and look up the corresponding ParseRule
        self.advance();
        let previous_token_type = self.parser.previous.token_type.clone();

        // Look up a prefix parser for the current token, the first token is always going to belong
        // to some kind of prefix expression
        // If there is no prefix parser, then the token must be a syntax error
        let Some(prefix_rule) = ParseRule::get_rule(previous_token_type).prefix else {
           self.error("Expect expression.");
           return;
        };

        let can_assign = precedence <= Precedence::Assignment;
        prefix_rule(self, can_assign);

        while precedence <= ParseRule::get_rule(self.parser.current.token_type.clone()).precedence {
            self.advance();
            // Look up for an infix parser for the next token
            // If we find one, it means the prefix expression we already compiled might be an
            // operand for it
            if let Some(infix_rule) =
                ParseRule::get_rule(self.parser.previous.token_type.clone()).infix
            {
                // Usually, it will consume the right operand
                infix_rule(self, can_assign);
            }
        }

        if can_assign && self.my_match(TokenType::Equal) {
            self.error("Invalid assignment target.")
        }
    }

    /// Return `true` if the current token has the given token type
    fn check(&self, expected: TokenType) -> bool {
        self.parser.current.token_type == expected
    }

    /// Consume the current token and return `true` if it hash the given token type, otherwise
    /// return `false`
    fn my_match(&mut self, expected: TokenType) -> bool {
        if !self.check(expected) {
            false
        } else {
            self.advance();
            true
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
        self.emit_byte(OpCode::Print);
    }

    /// A expression followed by a semicolon
    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit_byte(OpCode::Pop);
    }

    /// To "create" a scope, we just need to increment the current depth
    fn begin_scope(&mut self) {
        self.state.scope_depth += 1;
    }

    /// To "leave" a scope, we just need to decrease the current depth
    fn end_scope(&mut self) {
        self.state.scope_depth -= 1;
        while let Some(v) = self.state.locals.last() {
            if v.depth > self.state.scope_depth {
                self.emit_byte(OpCode::Pop);
                self.state.locals.pop().unwrap();
            } else {
                break;
            }
        }
    }

    /// Emit jump instruction and placeholder(2 bytes) and return the offset of the emitted
    /// instruction
    fn emit_jump<T>(&mut self, instruction: T) -> usize
    where
        T: Into<u8>,
    {
        self.emit_byte(instruction);
        // placeholder for jump offset
        // use 2 bytes for the jump offset operand
        self.emit_byte(std::u8::MAX);
        self.emit_byte(std::u8::MAX);

        self.current_chunk().code.len() - 2
    }

    /// Replace the operand at the given location with the calculated jump offset
    ///
    /// This function should be called before we emit the next instruction that we want the jump to
    /// land on
    fn patch_jump(&mut self, offset: usize) {
        let jump = self.current_chunk().code.len() - offset - 2;
        if jump > std::u16::MAX as usize {
            self.error("Too much code to jump over.");
        }
        self.current_chunk().code[offset] = ((jump >> 8) as u8) & std::u8::MAX;
        self.current_chunk().code[offset + 1] = jump as u8 & std::u8::MAX;
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let then_jump = self.emit_jump(OpCode::JumpIfFalse);
        self.emit_byte(OpCode::Pop); // pop the condition expression bool
        self.statement();

        let else_jump = self.emit_jump(OpCode::Jump);
        // [JumpIfFalse] Jump to the next statement after the body
        self.patch_jump(then_jump);
        self.emit_byte(OpCode::Pop); // pop the condition expression bool
        if self.my_match(TokenType::Else) {
            self.statement();
        }
        // [Jump] Jump to the next statement after the if statement
        self.patch_jump(else_jump);
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().code.len();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(OpCode::JumpIfFalse);
        self.emit_byte(OpCode::Pop); // pop the condition expression bool
        self.statement();

        self.emit_loop(loop_start);

        self.patch_jump(exit_jump); // jump to the next statement after the while body
        self.emit_byte(OpCode::Pop); // pop the condition expression bool, another path
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");
        if self.my_match(TokenType::Semicolon) {
            // no intializer
        } else if self.my_match(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk().code.len();
        let mut exit_jump = None;
        if !self.my_match(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.");

            // Jump out of the loop if the condition is false.
            exit_jump = Some(self.emit_jump(OpCode::JumpIfFalse));
            self.emit_byte(OpCode::Pop); // Pop condition
        }

        if !self.my_match(TokenType::RightParen) {
            let bodyjump = self.emit_jump(OpCode::Jump);
            let increment_start = self.current_chunk().code.len();
            self.expression(); // compile the increment expression, only execute it for its side
                               // effect
            self.emit_byte(OpCode::Pop); // Pop condition
                                         //
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");

            // This loop structure will take us back to the top of the for loop
            self.emit_loop(loop_start);
            // Later, when we emit the loop instruction after the body statement, this will cause
            // it to jump up to the increment expression instead of the top of the for loop
            loop_start = increment_start;
            self.patch_jump(bodyjump);
        }

        self.statement(); // loop body
        self.emit_loop(loop_start);
        if let Some(v) = exit_jump {
            self.patch_jump(v);
            self.emit_byte(OpCode::Pop); // Pop condition
        }
        self.end_scope();
    }

    fn return_statement(&mut self) {
        // We can't use return in the top-level
        if self.state.function_type == FunctionType::Script {
            self.error("Can't return from top-level code.");
        }
        if self.my_match(TokenType::Semicolon) {
            // `emit_return` will implicitly return nil
            self.emit_return();
        } else {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit_byte(OpCode::Return);
        }
    }

    /// Keep parsing declarations and statements and consume the final '}'. It will also
    /// check for the end of the token stream
    fn block(&mut self) {
        // block        -> "{" declarations* "}"
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.declaration()
        }
        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    fn statement(&mut self) {
        // statement    -> exprStmt
        //              |  printStmt
        //              |  ifStmt
        //              |  whileStmt
        //              |  forStmt
        //              |  returnStmt
        //              |  block ;
        if self.my_match(TokenType::Print) {
            self.print_statement();
        } else if self.my_match(TokenType::If) {
            self.if_statement();
        } else if self.my_match(TokenType::While) {
            self.while_statement();
        } else if self.my_match(TokenType::For) {
            self.for_statement();
        } else if self.my_match(TokenType::Return) {
            self.return_statement();
        } else if self.my_match(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }
    /// Try to add the value to constants, return 0 if we got too many constants
    fn make_constant(&mut self, value: Value) -> u8 {
        let Ok(constant_idx) = self.current_chunk().add_constant(value).try_into() else {
            self.error("Too many constants in one chunk.");
            // todo: or return a Result<T, E>?
            return 0;
        };
        constant_idx
    }

    fn identifier_constant(&mut self, name: Token) -> u8 {
        self.make_constant(Value::String(name.lexeme))
    }

    /// Consume the next token, which must be an identifier. Add its lexeme to the chunks's
    /// constants table as a string, and then returns the constant table index where it was added
    fn parse_variable(&mut self, error_msg: &str) -> (String, u8) {
        self.consume(TokenType::Identifier, error_msg);

        self.declare_variable();
        // Exit the function  and return a dummy index if we're in a local scope
        // , because we don't need to store the variable's name into the sontant table.
        if self.state.scope_depth > 0 {
            return (String::new(), 0);
        }

        let identifier_name = self.parser.previous.lexeme.clone();
        let previous_token = std::mem::take(&mut self.parser.previous);
        (identifier_name, self.identifier_constant(previous_token))
    }

    /// Add the local variable to the compilers's list of variables
    fn add_local(&mut self, token: Token) {
        if self.state.locals.len() == std::u8::MAX as usize {
            self.error("Too many local variables in function.");
            return;
        }
        // -1 is a special sentinel value - this local variable is in "unitialized" state
        self.state.locals.push(Local::new(token, -1));
    }

    fn declare_variable(&mut self) {
        // Exit if we are in global scope
        if self.state.scope_depth == 0 {
            return;
        }
        // Prevent redeclaring a variable with the same name as previous declaration
        let name = std::mem::take(&mut self.parser.previous);
        let mut same_name_in_same_scope = false;
        for token in self.state.locals.iter().rev() {
            // It's only an error to have 2 variables with the same name in the same local scope,
            // which means they must have the sanme scope_depth
            if token.depth < self.state.scope_depth {
                break;
            }
            if token.name.lexeme == name.lexeme {
                same_name_in_same_scope = true;
                break;
            }
        }
        if same_name_in_same_scope {
            self.error("Already a variable with this name in this scope.");
        }

        self.add_local(name);
    }

    fn mark_initialized(&mut self) {
        // when we declare a function in the top-level, the function is bound to a global variable.
        // There is no local variable to mark initialized
        if self.state.scope_depth == 0 {
            return;
        }
        if let Some(local) = self.state.locals.last_mut() {
            local.depth = self.state.scope_depth;
        }
    }

    /// Emit the bytecode for storing the variable's value in the global variable hashtable
    /// Emit the bytecode to store a local variable if we're in a local scope(just return)
    fn define_variable(&mut self, global: u8) {
        if self.state.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit_bytes(OpCode::DefineGlobal, global);
    }

    fn var_declaration(&mut self) {
        let (_, global) = self.parse_variable("Expect variable name.");

        // look for an initializer expresssion
        if self.my_match(TokenType::Equal) {
            self.expression();
        } else {
            // if the user doesn't initialize the variable, the compiler implicitly initialize it
            // it nil
            // e.g.           var a;
            // is equal to    var a = nil;
            self.emit_byte(OpCode::Nil);
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    fn function(&mut self, func_name: String, func_type: FunctionType) {
        let old_state = std::mem::take(&mut self.state);
        self.state.function_type = func_type;
        self.state.function.name = func_name;
        self.state.enclosing = Some(Box::new(old_state));
        // now we have a new state to operate on

        self.begin_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after function name.");
        if !self.check(TokenType::RightParen) {
            loop {
                self.state.function.arity += 1;
                if self.state.function.arity > 255 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }
                let (_, constant) = self.parse_variable("Expect parameter name.");
                self.define_variable(constant);

                if !self.my_match(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();

        let function = self.end_compiler();
        let val = self.make_constant(Value::Func(Rc::new(function)));
        self.emit_bytes(OpCode::Closure, val);
    }

    fn func_declaration(&mut self) {
        let (func_name, global) = self.parse_variable("Expect func name");
        self.mark_initialized();
        self.function(func_name, FunctionType::Function);
        self.define_variable(global);
    }

    fn declaration(&mut self) {
        // declaration  -> varDecl
        //              |  funDecl
        //              |  statement ;
        if self.my_match(TokenType::Var) {
            self.var_declaration();
        } else if self.my_match(TokenType::Fun) {
            self.func_declaration();
        } else {
            self.statement();
        }

        if self.parser.panic_mode {
            self.synchronize();
        }
    }

    /// Walk the list of locals that are currently in the scope, This ensures that inner local
    /// variables correctly shadow locals with the same name in surrouding scopes
    /// Return `None` if we can't find the `token` in `self.locals` or it is in "unitialized" state
    fn resolve_local(&mut self, token: &Token) -> Option<u8> {
        let mut use_uninitialized_variable = false;
        let mut local_index = None;
        for (idx, i) in self.state.locals.iter().enumerate().rev() {
            if i.name.lexeme == token.lexeme {
                if i.depth == -1 {
                    use_uninitialized_variable = true;
                } else {
                    local_index = Some(idx as u8);
                }
            }
        }
        if use_uninitialized_variable {
            self.error("Can't read local variable in its own initializer.");
        }
        local_index
    }

    fn named_variable(&mut self, token: Token, can_assign: bool) {
        let mut get_op = OpCode::GetLocal;
        let mut set_op = OpCode::SetLocal;

        let mut arg = 0_u8;
        if let Some(idx) = self.resolve_local(&token) {
            arg = idx;
        } else {
            arg = self.identifier_constant(token);
            get_op = OpCode::GetGlobal;
            set_op = OpCode::SetGlobal;
        }

        if can_assign && self.my_match(TokenType::Equal) {
            // This is an assignment (setter)
            // e.g. var foo = "bar";
            self.expression();
            self.emit_bytes(set_op, arg);
        } else {
            // For access (getter)
            self.emit_bytes(get_op, arg);
        }
    }

    fn variable(&mut self, can_assign: bool) {
        let previous_token = std::mem::take(&mut self.parser.previous);
        self.named_variable(previous_token, can_assign);
    }

    /// Keep skiping tokens until we reach something that looks like a statement boundary
    fn synchronize(&mut self) {
        self.parser.panic_mode = false;

        while self.parser.current.token_type != TokenType::Eof {
            if self.parser.previous.token_type == TokenType::Semicolon {
                return;
            }
            match self.parser.current.token_type {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => {
                    return;
                }
                _ => {} // do nothing
            }
            self.advance();
        }
    }

    pub fn compile(mut self, source: &str) -> Result<Function, InterpretResult> {
        self.scanner.init_scanner(source);
        self.advance();
        while !self.my_match(TokenType::Eof) {
            self.declaration();
        }

        if self.parser.had_error {
            Err(InterpretResult::CompileError)
        } else {
            Ok(self.end_compiler())
        }
    }
}
