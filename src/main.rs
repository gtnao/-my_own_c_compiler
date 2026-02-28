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
    let bytes = input.as_bytes();
    let mut pos = 0;

    let val = read_number(bytes, &mut pos);

    println!("  .globl main");
    println!("main:");
    println!("  mov ${}, %rax", val);

    while pos < bytes.len() {
        let ch = bytes[pos] as char;
        if ch == '+' {
            pos += 1;
            let val = read_number(bytes, &mut pos);
            println!("  add ${}, %rax", val);
        } else if ch == '-' {
            pos += 1;
            let val = read_number(bytes, &mut pos);
            println!("  sub ${}, %rax", val);
        } else {
            eprintln!("Unexpected character: '{}'", ch);
            process::exit(1);
        }
    }

    println!("  ret");
}

fn read_number(bytes: &[u8], pos: &mut usize) -> i64 {
    let start = *pos;
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_digit() {
        *pos += 1;
    }
    if start == *pos {
        eprintln!(
            "Expected a number at position {}, but got '{}'",
            start,
            if *pos < bytes.len() {
                bytes[*pos] as char
            } else {
                '\0'
            }
        );
        process::exit(1);
    }
    let s = std::str::from_utf8(&bytes[start..*pos]).unwrap();
    s.parse().unwrap()
}
