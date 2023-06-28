use crate::scanner::{Scanner, TokenType};

pub fn compile(source: &str) {
    let mut scanner = Scanner::init_scanner(source);
    let mut line: usize = 0;
    loop {
        let token = scanner.scan_token();
        if token.line != line {
            print!("{:4} ", token.line);
            line = token.line;
        } else {
            print!("   | ")
        }
        println!("{:?} '{}'", token.r#type, token.lexeme);

        if token.r#type == TokenType::Eof {
            break;
        }
    }
}
