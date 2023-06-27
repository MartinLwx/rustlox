#[derive(Debug, PartialEq)]
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
    Error,
}

#[derive(Debug)]
pub struct Token {
    pub r#type: TokenType,
    pub lexeme: String,
    pub line: usize,
}

pub struct Scanner {
    source: Vec<char>,
    /// Marks the beginning of the current lexeme being scanned
    start: usize,
    /// Points to the current character being lookat at
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            source: vec![],
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
            source: source.chars().collect(),
        }
    }

    fn make_token(&self, token: TokenType) -> Token {
        Token {
            r#type: token,
            lexeme: String::new(),
            line: self.line,
        }
    }

    fn error_token(&self, msg: &str) -> Token {
        Token {
            r#type: TokenType::Error,
            lexeme: msg.to_string(),
            line: self.line,
        }
    }

    fn is_at_end(&self) -> bool {
        self.current == self.source.len()
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source[self.current - 1]
    }

    fn my_match(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.source[self.current] != expected {
            return false;
        }
        self.current += 1;

        true
    }

    fn peek(&self) -> char {
        self.source[self.current]
    }

    fn peek_next(&self) -> Option<char> {
        // todo: is this correct?
        if self.is_at_end() {
            None
        } else {
            Some(self.source[self.current + 1])
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '/' => {
                    if let Some('/') = self.peek_next() {
                        // A comment goes until the end of the line
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    }
                }
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                _ => (),
            }
        }
    }

    fn make_string(&mut self) -> Token {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }
        if self.is_at_end() {
            return self.error_token("Unterminated string.");
        }

        // for the closing quote
        self.advance();
        self.make_token(TokenType::STRING)
    }

    fn make_number(&mut self) -> Token {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        if let ('.', Some(ch2)) = (self.peek(), self.peek_next()) {
            if ch2.is_ascii_digit() {
                // Consume the "."
                self.advance();
                while self.peek().is_ascii_digit() {
                    self.advance();
                }
            }
        }
        self.make_token(TokenType::Number)
    }

    /// Returns the next token in the source code
    pub fn scan_token(&mut self) -> Token {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        match self.advance() {
            '(' => self.make_token(TokenType::LeftParen),
            ')' => self.make_token(TokenType::RightParen),
            '{' => self.make_token(TokenType::LeftBrace),
            '}' => self.make_token(TokenType::RightBrace),
            ';' => self.make_token(TokenType::Semicolon),
            ',' => self.make_token(TokenType::Comma),
            '.' => self.make_token(TokenType::Dot),
            '-' => self.make_token(TokenType::Minus),
            '+' => self.make_token(TokenType::And),
            '/' => self.make_token(TokenType::Slash),
            '*' => self.make_token(TokenType::Star),
            '!' if self.my_match('=') => self.make_token(TokenType::BangEqual),
            '!' => self.make_token(TokenType::Bang),
            '=' if self.my_match('=') => self.make_token(TokenType::EqualEqual),
            '=' => self.make_token(TokenType::Equal),
            '<' if self.my_match('=') => self.make_token(TokenType::LessEqual),
            '<' => self.make_token(TokenType::Less),
            '>' if self.my_match('=') => self.make_token(TokenType::GreaterEqual),
            '>' => self.make_token(TokenType::Greater),
            ch if ch.is_ascii_digit() => self.make_number(),
            '"' => self.make_string(),
            _ => self.error_token("Unexpcted character."),
        }
    }
}
