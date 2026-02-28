mod ast;
mod codegen;
mod error;
mod lexer;
mod parser;
mod preprocess;
mod token;
mod types;

use std::env;
use std::fs;
use std::process;

use crate::codegen::Codegen;
use crate::error::ErrorReporter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::preprocess::preprocess;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    let input = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Failed to read file '{}': {}", filename, err);
        process::exit(1);
    });

    // Preprocess (handle #include)
    let preprocessed = preprocess(&input, filename);
    let source = preprocessed.trim();

    let reporter = ErrorReporter::new(filename, source);

    let mut lexer = Lexer::new(source, &reporter);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens, &reporter);
    let program = parser.parse();

    let mut codegen = Codegen::new(filename);
    let output = codegen.generate(&program);

    print!("{}", output);
}
