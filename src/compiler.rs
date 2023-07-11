use crate::chunk::{Chunk, OpCode};
use crate::disassembler::disassemble_chunk;
use crate::scanner::{Scanner, Token, TokenType};
use crate::value::Value;
use crate::vm::InterpretResult;

#[derive(Default)]
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
type ParseFn<'a> = fn(&mut Compiler<'a>, bool) -> (); // function pointer

/// The three properties which represents a single row in the Pratt parser table
struct ParseRule<'a> {
    prefix: Option<ParseFn<'a>>,
    infix: Option<ParseFn<'a>>,
    precedence: Precedence,
}

impl<'a> ParseRule<'a> {
    fn get_rule(op_type: TokenType) -> ParseRule<'a> {
        match op_type {
            TokenType::LeftParen => ParseRule {
                prefix: Some(Compiler::grouping),
                infix: None,
                precedence: Precedence::None,
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
            TokenType::Slash => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Factor,
            },
            TokenType::Star => ParseRule {
                prefix: None,
                infix: Some(Compiler::binary),
                precedence: Precedence::Factor,
            },
            TokenType::Number => ParseRule {
                prefix: Some(Compiler::number),
                infix: None,
                precedence: Precedence::Factor,
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
    depth: usize,
}

impl Local {
    pub fn new(name: Token, depth: usize) -> Self {
        Self { name, depth }
    }
}

pub struct Compiler<'a> {
    scanner: Scanner,
    parser: Parser,
    // use a reference to avoid the overhead of copy the whole chunk
    compiling_chunk: &'a mut Chunk,
    locals: Vec<Local>,
    scope_depth: usize,
}

impl<'a> Compiler<'a> {
    pub fn new(chunk: &'a mut Chunk) -> Self {
        Self {
            scanner: Scanner::new(),
            parser: Parser::default(),
            compiling_chunk: chunk,
            locals: vec![],
            scope_depth: 0,
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

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.compiling_chunk
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
        self.emit_byte(OpCode::Return)
    }

    fn end_compiler(&mut self) {
        self.emit_return();

        #[cfg(debug_assertions)]
        {
            if !self.parser.had_error {
                disassemble_chunk(self.current_chunk(), "code");
            }
        }
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

    fn literal(&mut self, _can_assign: bool) {
        // the parse_precedence function has already consumed the keyword token
        match self.parser.previous.token_type {
            TokenType::True => self.emit_byte(OpCode::True),
            TokenType::False => self.emit_byte(OpCode::False),
            TokenType::Nil => self.emit_byte(OpCode::Nil),
            _ => panic!("Unreachable!"),
        }
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
        self.scope_depth += 1;
    }

    /// To "leave" a scope, we just need to decrease the current depth
    fn end_scope(&mut self) {
        self.scope_depth -= 1;
        while self.locals.pop().is_some() {
            self.emit_byte(OpCode::Pop);
        }
    }

    /// Keep parsing declarations and statements until it hits the closing brace. It will also
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
        //              |  block ;
        if self.my_match(TokenType::Print) {
            self.print_statement();
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
    fn parse_variable(&mut self, error_msg: &str) -> u8 {
        self.consume(TokenType::Identifier, error_msg);

        self.declare_variable();
        // Exit the function  and return a dummy index if we're in a local scope
        // , because we don't need to store the variable's name into the sontant table.
        if self.scope_depth > 0 {
            return 0;
        }

        let previous_token = std::mem::take(&mut self.parser.previous);
        self.identifier_constant(previous_token)
    }

    /// Add the local variable to the compilers's list of variables
    fn add_local(&mut self, token: Token) {
        if self.locals.len() == std::u8::MAX as usize {
            self.error("Too many local variables in function.");
            return;
        }
        self.locals.push(Local::new(token, self.scope_depth));
    }

    fn declare_variable(&mut self) {
        // Exit if we are in global scope
        if self.scope_depth == 0 {
            return;
        }
        // Prevent redeclaring a variable with the same name as previous declaration
        let name = std::mem::take(&mut self.parser.previous);
        let mut same_name_in_same_scope = false;
        for token in self.locals.iter().rev() {
            // It's only an error to have 2 variables with the same name in the same local scope,
            // which means they must have the sanme scope_depth
            if token.depth < self.scope_depth {
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

    /// Emit the bytecode for storing the variable's value in the global variable hashtable
    /// Emit the bytecode to store a local variable if we're in a local scope(just return)
    fn define_variable(&mut self, global: u8) {
        if self.scope_depth > 0 {
            return;
        }
        self.emit_bytes(OpCode::DefineGlobal, global);
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

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

    fn declaration(&mut self) {
        // declaration  -> varDecl
        //              |  statement ;
        if self.my_match(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.parser.panic_mode {
            self.synchronize();
        }
    }

    fn named_variable(&mut self, token: Token, can_assign: bool) {
        let arg = self.identifier_constant(token);

        if can_assign && self.my_match(TokenType::Equal) {
            // This is an assignment (setter)
            // e.g. var foo = "bar";
            self.expression();
            self.emit_bytes(OpCode::SetGlobal, arg);
        } else {
            self.emit_bytes(OpCode::GetGlobal, arg);
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

    pub fn compile(&mut self, source: &str) -> InterpretResult {
        self.scanner.init_scanner(source);
        self.advance();
        while !self.my_match(TokenType::Eof) {
            self.declaration();
        }
        self.end_compiler();
        if self.parser.had_error {
            InterpretResult::CompileError
        } else {
            InterpretResult::Ok
        }
    }
}
