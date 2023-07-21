## What's Lox

Lox is a dynamically typed, interpreted script language, which is designed by Robert Nystrom for his book [*Crafting interpreters*](https://craftinginterpreters.com/).

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

## Benchmark
A naive benchmark in my MBP Intel i5-8257U @1.40GHz:

| operations | Rust(`-O`)    | Python 3.10.9 | rustlox | [PyLox](https://github.com/MartinLwx/pylox) |
| ---------- | ------------- | ------------- | ------- | ------------------------------------------- |
| `fib(35)`  | ~ 0.03s       | ~ 3s          | ~ 7s    | ~ 600s                                      |


## Notes
- 17.2 Parsing Tokens - Use `std::mem::take` to handle `self.parser.previous = self.parser.current;` and derive `Default` for the `Token` type.
- 17.6 A Pratt Parser - Impl a `next` associated function for the `Precedence` struct to get the next enum item.
- 18.1 Tagged Unions  - Use `enum` instead of `union`, as the `enum` type is quite powerful in Rust.
- 24.4 Function declaration
    - Set a `CompilerState` field for the `Compiler` struct, which contains local variables, scope depth, function, enclosing, and function_type
    - Before compiling the function declaration, use `std::mem::take` to remember the old state and store it in the `enclosing` field
