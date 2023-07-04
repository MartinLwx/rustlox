## What's Lox

Lox is a dynamically typed, interpreted script language, which is designed by Robert Nystorm for his book [*Crafting interpreters*](https://craftinginterpreters.com/).

## What's rustlox

A virtual machine(VM) for the Lox programming language implemented in Rust. The original book utilizes C to build the VM, referred to as `clox`. To maintain consistency, I chose the name `rustlox` for the Rust implementation.

## Usage
```sh
# REPL
# debug build with debug info
$ cargo run

# execute a lox file
$ cargo run -- <file>
```

## Notes
- 17.2 Parsing Tokens - Use `std::mem::take` to handle `self.parser.previous = self.parser.current;` and derive `Default` for the `Token` type.
- 17.6 A Pratt Parser - Impl a `next` associated function for `Precedence` struct to get the next enum item.
- 18.1 Tagged Unions  - Use `enum` instead of `union`, as the `enum` type is quite powerful in Rust.
