#[derive(Debug)]
pub enum TokenType {
    // Single-character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    // One or two character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Identifier,
    STRING,
    Number,
    // keywords
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Supser,
    This,
    True,
    Var,
    White,
    Eof,
    Error(String),
}

#[derive(Debug)]
pub struct Token {
    pub r#type: TokenType,
    pub start: usize,
    pub length: usize,
    pub line: usize,
}

pub struct Scanner {
    source: String,
    /// Marks the beginning of the current lexeme being scanned
    start: usize,
    /// Points to the current character being lookat at
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            source: String::new(),
            start: 0,
            current: 0,
            line: 1,
        }
    }
    pub fn init_scanner(source: &str) -> Self {
        Self {
            start: 0,
            current: 0,
            line: 1,
            source: source.to_string(),
        }
    }

    fn make_token(&self, token: TokenType) -> Token {
        Token {
            r#type: token,
            start: self.start,
            length: self.current - self.start,
            line: self.line,
        }
    }

    fn error_token(&self) -> Token {
        self.make_token(TokenType::Error("Unexpected character.".to_string()))
    }

    fn is_at_end(&self) -> bool {
        self.current < self.source.len()
    }

    /// Returns the next token in the source code
    pub fn scan_token(&mut self) -> Token {
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        self.error_token()
    }
}
