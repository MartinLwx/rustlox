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
type ParseFn<'a> = fn(&mut Compiler<'a>) -> (); // function pointer

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
            _ => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        }
    }
}

pub struct Compiler<'a> {
    scanner: Scanner,
    parser: Parser,
    // use a reference to avoid the overhead of copy the whole chunk
    compiling_chunk: &'a mut Chunk,
}

impl<'a> Compiler<'a> {
    pub fn new(chunk: &'a mut Chunk) -> Self {
        Self {
            scanner: Scanner::new(),
            parser: Parser::default(),
            compiling_chunk: chunk,
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

    /// Try to add the value to constants, return 0 if we got too many constants
    fn make_constant(&mut self, value: Value) -> u8 {
        let Ok(constant_idx) = self.current_chunk().add_constant(value).try_into() else {
            self.error("Too many constants in one chunk.");
            // todo: or return a Result<T, E>?
            return 0;
        };
        constant_idx
    }

    fn number(&mut self) {
        let value: f64 = self.parser.previous.lexeme.parse().unwrap();
        self.emit_constant(Value::Number(value));
    }

    fn string(&mut self) {
        let end = self.parser.previous.lexeme.len() - 2;
        // todo: or create a objects field for the Chunk struct
        self.emit_constant(Value::String(
            self.parser.previous.lexeme[1..=end].to_string(),
        ));
    }

    fn grouping(&mut self) {
        // Assumption: the initial '(' has already been consumed
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn unary(&mut self) {
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

    fn binary(&mut self) {
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

    fn literal(&mut self) {
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

        prefix_rule(self);

        while precedence <= ParseRule::get_rule(self.parser.current.token_type.clone()).precedence {
            self.advance();
            // Look up for an infix parser for the next token
            // If we find one, it means the prefix expression we already compiled might be an
            // operand for it
            if let Some(infix_rule) =
                ParseRule::get_rule(self.parser.previous.token_type.clone()).infix
            {
                // Usually, it will consume the right operand
                infix_rule(self);
            }
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

    fn statement(&mut self) {
        // statement    -> exprStmt
        //              |  printStmt ;
        if self.my_match(TokenType::Print) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
    }

    fn identifier_constant(&mut self, name: Token) -> u8 {
        self.make_constant(Value::String(name.lexeme))
    }

    /// Consume the next token, which must be an identifier
    fn parse_variable(&mut self, error_msg: &str) -> u8 {
        self.consume(TokenType::Identifier, error_msg);

        let previous_token = std::mem::take(&mut self.parser.previous);

        self.identifier_constant(previous_token)
    }

    fn define_variable(&mut self, global: u8) {
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
