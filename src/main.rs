mod ast;
mod codegen;
mod lexer;
mod parser;
mod token;

use std::env;
use std::fs;
use std::process;

use crate::codegen::Codegen;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file>", args[0]);
        process::exit(1);
    }

    let input = fs::read_to_string(&args[1]).unwrap_or_else(|err| {
        eprintln!("Failed to read file '{}': {}", args[1], err);
        process::exit(1);
    });

    let mut lexer = Lexer::new(input.trim());
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let expr = parser.parse();

    let mut codegen = Codegen::new();
    let output = codegen.generate(&expr);

    print!("{}", output);
}
