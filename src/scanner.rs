#[derive(Hash, Eq, Clone, Debug, PartialEq, Default)]
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
    Super,
    This,
    True,
    Var,
    While,
    #[default]
    Eof,
    Error,
}

#[derive(Debug, Default)]
pub struct Token {
    pub token_type: TokenType,
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
    pub fn init_scanner(&mut self, source: &str) {
        self.source = source.chars().collect();
    }

    fn make_token(&self, token_type: TokenType) -> Token {
        Token {
            lexeme: self.source[self.start..self.current].iter().collect(),
            line: self.line,
            token_type,
        }
    }

    fn error_token(&self, msg: &str) -> Token {
        Token {
            token_type: TokenType::Error,
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
        // todo: or change self.peek() to return Option<char>
        if self.is_at_end() {
            return '\0';
        }
        self.source[self.current]
    }

    fn peek_next(&self) -> Option<char> {
        // todo: is this correct?
        if self.current + 1 >= self.source.len() {
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
                    return;
                }
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                _ => return,
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

    fn check_keyword(
        &self,
        start: usize,
        length: usize,
        rest: &str,
        token_type: TokenType,
    ) -> TokenType {
        if self.current - self.start == start + length
            && self.source[self.start + start..self.current]
                .iter()
                .collect::<String>()
                == rest
        {
            token_type
        } else {
            TokenType::Identifier
        }
    }

    /// By using the Trie data structure to decide if an identifier is a keyword
    fn identifier_type(&self) -> TokenType {
        match self.source[self.start] {
            'a' => self.check_keyword(1, 2, "nd", TokenType::And),
            'c' => self.check_keyword(1, 4, "lass", TokenType::Class),
            'e' => self.check_keyword(1, 3, "lse", TokenType::Else),
            'i' => self.check_keyword(1, 1, "f", TokenType::If),
            'f' if self.current - self.start > 1 => match self.source[self.start + 1] {
                'a' => self.check_keyword(2, 3, "lse", TokenType::False),
                'o' => self.check_keyword(2, 1, "r", TokenType::For),
                'u' => self.check_keyword(2, 1, "n", TokenType::Fun),
                _ => TokenType::Identifier,
            },
            'n' => self.check_keyword(1, 2, "il", TokenType::Nil),
            'o' => self.check_keyword(1, 1, "r", TokenType::Or),
            'p' => self.check_keyword(1, 4, "rint", TokenType::Print),
            'r' => self.check_keyword(1, 5, "eturn", TokenType::Return),
            's' => self.check_keyword(1, 4, "uper", TokenType::Super),
            't' if self.current - self.start > 1 => match self.source[self.start + 1] {
                'h' => self.check_keyword(2, 2, "is", TokenType::This),
                'r' => self.check_keyword(2, 2, "ue", TokenType::True),
                _ => TokenType::Identifier,
            },
            'v' => self.check_keyword(1, 2, "ar", TokenType::Var),
            'w' => self.check_keyword(1, 4, "hile", TokenType::While),
            _ => TokenType::Identifier,
        }
    }

    fn make_identifier(&mut self) -> Token {
        while self.peek() == '_'
            || self.peek().is_ascii_alphabetic()
            || self.peek().is_ascii_digit()
        {
            self.advance();
        }
        self.make_token(self.identifier_type())
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
            '+' => self.make_token(TokenType::Plus),
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
            ch if ch.is_ascii_alphabetic() || ch == '_' => self.make_identifier(),
            '"' => self.make_string(),
            _ => self.error_token("Unexpcted character."),
        }
    }
}
