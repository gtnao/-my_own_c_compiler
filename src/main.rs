mod lexer;
mod token;

use std::env;
use std::fs;
use std::process;

use crate::lexer::Lexer;
use crate::token::TokenKind;

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

    let mut pos = 0;

    // Expect the first token to be a number
    let val = expect_number(&tokens, &mut pos);

    println!("  .globl main");
    println!("main:");
    println!("  mov ${}, %rax", val);

    while tokens[pos].kind != TokenKind::Eof {
        match tokens[pos].kind {
            TokenKind::Plus => {
                pos += 1;
                let val = expect_number(&tokens, &mut pos);
                println!("  add ${}, %rax", val);
            }
            TokenKind::Minus => {
                pos += 1;
                let val = expect_number(&tokens, &mut pos);
                println!("  sub ${}, %rax", val);
            }
            _ => {
                eprintln!("Unexpected token: {:?}", tokens[pos].kind);
                process::exit(1);
            }
        }
    }

    println!("  ret");
}

fn expect_number(tokens: &[token::Token], pos: &mut usize) -> i64 {
    if let TokenKind::Num(val) = tokens[*pos].kind {
        *pos += 1;
        val
    } else {
        eprintln!("Expected a number, but got {:?}", tokens[*pos].kind);
        process::exit(1);
    }
}
