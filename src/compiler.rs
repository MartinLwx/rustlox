use crate::chunk::Chunk;
use crate::scanner::{Scanner, Token, TokenType};
use crate::vm::InterpretResult;

struct Parser {
    current: Token,
    previous: Token,
    had_error: bool,
    panic_mode: bool,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            current: Token::default(),
            previous: Token::default(),
            had_error: false,
            panic_mode: false,
        }
    }
}

pub struct Compiler {
    scanner: Scanner,
    parser: Parser,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            scanner: Scanner::new(),
            parser: Parser::new(),
        }
    }
    fn error_at(&mut self, token: Token, msg: &str) {
        // While the panic mode flag is set, we simply suppress any other errors that get detected
        if self.parser.panic_mode {
            return;
        }
        self.parser.panic_mode = true;
        eprint!("[line{}]Error", token.line);
        match token.r#type {
            TokenType::Eof => eprint!(" at end"),
            TokenType::Error => eprint!(""),
            _ => eprint!(" at {} {}", token.lexeme.len(), token.lexeme),
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
        loop {
            self.parser.current = self.scanner.scan_token();
            if self.parser.current.r#type != TokenType::Error {
                break;
            }
            // todo: can we avoid clone() here?
            self.error_at_current(&self.parser.current.lexeme.clone());
        }
    }
    fn expression(&self) {}
    fn consume(&mut self, token_type: TokenType, msg: &str) {
        if self.parser.current.r#type == token_type {
            self.advance();
            return;
        }
        self.error_at_current(msg);
    }
    pub fn compile(&mut self, source: &str, chunk: &mut Chunk) -> InterpretResult {
        self.scanner.init_scanner(source);
        self.advance();
        self.expression();
        self.consume(TokenType::Eof, "Expect end of expression.");
        if self.parser.had_error {
            InterpretResult::CompileError
        } else {
            InterpretResult::Ok
        }
    }
}
