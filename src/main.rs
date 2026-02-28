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
use std::process::Command;

use crate::codegen::Codegen;
use crate::error::ErrorReporter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::preprocess::preprocess;

enum OutputMode {
    Preprocess, // -E: preprocess only
    Assembly,   // -S: compile to assembly
    Object,     // -c: compile to object file
    Executable, // default: compile and link to executable
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut mode = OutputMode::Executable;
    let mut output_file: Option<String> = None;
    let mut input_files: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-E" => mode = OutputMode::Preprocess,
            "-S" => mode = OutputMode::Assembly,
            "-c" => mode = OutputMode::Object,
            "-o" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                } else {
                    eprintln!("error: -o requires an argument");
                    process::exit(1);
                }
            }
            arg if arg.starts_with('-') => {
                // Ignore unknown flags for compatibility
            }
            _ => {
                input_files.push(args[i].clone());
            }
        }
        i += 1;
    }

    if input_files.is_empty() {
        eprintln!("Usage: {} [options] <file>...", args[0]);
        eprintln!("Options:");
        eprintln!("  -E          Preprocess only");
        eprintln!("  -S          Compile to assembly");
        eprintln!("  -c          Compile to object file");
        eprintln!("  -o <file>   Output file");
        process::exit(1);
    }

    match mode {
        OutputMode::Preprocess => {
            for filename in &input_files {
                let input = read_file(filename);
                let preprocessed = preprocess(&input, filename);
                if let Some(ref out) = output_file {
                    fs::write(out, &preprocessed).unwrap_or_else(|err| {
                        eprintln!("Failed to write '{}': {}", out, err);
                        process::exit(1);
                    });
                } else {
                    print!("{}", preprocessed);
                }
            }
        }
        OutputMode::Assembly => {
            for filename in &input_files {
                let asm = compile_to_assembly(filename);
                let out = output_file.clone().unwrap_or_else(|| {
                    filename.replace(".c", ".s")
                });
                fs::write(&out, &asm).unwrap_or_else(|err| {
                    eprintln!("Failed to write '{}': {}", out, err);
                    process::exit(1);
                });
            }
        }
        OutputMode::Object => {
            for filename in &input_files {
                let asm = compile_to_assembly(filename);
                let asm_file = format!("/tmp/mycc_{}.s", std::process::id());
                fs::write(&asm_file, &asm).unwrap_or_else(|err| {
                    eprintln!("Failed to write temp file: {}", err);
                    process::exit(1);
                });
                let out = output_file.clone().unwrap_or_else(|| {
                    filename.replace(".c", ".o")
                });
                let status = Command::new("gcc")
                    .args(["-c", &asm_file, "-o", &out])
                    .status()
                    .unwrap_or_else(|err| {
                        eprintln!("Failed to run assembler: {}", err);
                        process::exit(1);
                    });
                let _ = fs::remove_file(&asm_file);
                if !status.success() {
                    process::exit(1);
                }
            }
        }
        OutputMode::Executable => {
            if input_files.len() == 1 && output_file.is_none() {
                // Legacy mode: single file, output to stdout
                let asm = compile_to_assembly(&input_files[0]);
                print!("{}", asm);
            } else {
                // Multi-file: compile each to .o, then link
                let mut obj_files = Vec::new();
                for filename in &input_files {
                    let asm = compile_to_assembly(filename);
                    let asm_file = format!("/tmp/mycc_{}_{}.s", std::process::id(), obj_files.len());
                    fs::write(&asm_file, &asm).unwrap_or_else(|err| {
                        eprintln!("Failed to write temp file: {}", err);
                        process::exit(1);
                    });
                    let obj_file = format!("/tmp/mycc_{}_{}.o", std::process::id(), obj_files.len());
                    let status = Command::new("gcc")
                        .args(["-c", &asm_file, "-o", &obj_file])
                        .status()
                        .unwrap_or_else(|err| {
                            eprintln!("Failed to run assembler: {}", err);
                            process::exit(1);
                        });
                    let _ = fs::remove_file(&asm_file);
                    if !status.success() {
                        process::exit(1);
                    }
                    obj_files.push(obj_file);
                }
                let out = output_file.unwrap_or_else(|| "a.out".to_string());
                let mut gcc_args: Vec<String> = obj_files.clone();
                gcc_args.push("-o".to_string());
                gcc_args.push(out);
                let status = Command::new("gcc")
                    .args(&gcc_args)
                    .status()
                    .unwrap_or_else(|err| {
                        eprintln!("Failed to run linker: {}", err);
                        process::exit(1);
                    });
                for f in &obj_files {
                    let _ = fs::remove_file(f);
                }
                if !status.success() {
                    process::exit(1);
                }
            }
        }
    }
}

fn read_file(filename: &str) -> String {
    fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Failed to read file '{}': {}", filename, err);
        process::exit(1);
    })
}

fn compile_to_assembly(filename: &str) -> String {
    let input = read_file(filename);
    let preprocessed = preprocess(&input, filename);
    let source = preprocessed.trim();

    let reporter = ErrorReporter::new(filename, source);

    let mut lexer = Lexer::new(source, &reporter);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens, &reporter);
    let program = parser.parse();

    let mut codegen = Codegen::new(filename);
    codegen.generate(&program)
}
