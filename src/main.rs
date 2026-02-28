use std::env;
use std::fs;
use std::process;

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

    let input = input.trim();

    let val: i64 = input.parse().unwrap_or_else(|_| {
        eprintln!("Expected a number, but got '{}'", input);
        process::exit(1);
    });

    println!("  .globl main");
    println!("main:");
    println!("  mov ${}, %rax", val);
    println!("  ret");
}
