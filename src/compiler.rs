use crate::scanner::Scanner;

pub fn compile(source: &str) {
    let mut scanner = Scanner::init_scanner(source);
    let mut line: usize = 0;
    loop {
        let token = scanner.scan_token();
        if token.line != line {
            println!("{:04}", token.line);
            line = token.line;
        } else {
            println!(" | ")
        }
        println!("{:?} '{}{}", token.r#type, token.length, token.start);
    }
}
