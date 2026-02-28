# Step 13.6: Multi-File Compilation and CLI Options

## Overview

Add GCC-compatible command-line options for controlling the compilation pipeline:

- `-E` — Preprocess only (output preprocessed source)
- `-S` — Compile to assembly (`.s` file)
- `-c` — Compile to object file (`.o` file)
- `-o <file>` — Specify output file name
- Default (no flag) — Compile and link to executable, or output assembly to stdout for single files

## CLI Usage

```bash
# Preprocess only
mycc -E input.c              # output to stdout
mycc -E input.c -o output.i  # output to file

# Compile to assembly
mycc -S input.c              # creates input.s
mycc -S input.c -o output.s  # creates output.s

# Compile to object file
mycc -c input.c              # creates input.o
mycc -c input.c -o output.o  # creates output.o

# Compile and link
mycc input.c -o program      # creates executable 'program'
mycc file1.c file2.c -o prog # multi-file compilation

# Legacy mode (single file, stdout)
mycc input.c                  # assembly to stdout (backwards compatible)
```

## Implementation

### Output Modes

```rust
enum OutputMode {
    Preprocess, // -E
    Assembly,   // -S
    Object,     // -c
    Executable, // default
}
```

### Argument Parsing

Simple flag-based parsing that processes arguments left-to-right:

```rust
match args[i].as_str() {
    "-E" => mode = OutputMode::Preprocess,
    "-S" => mode = OutputMode::Assembly,
    "-c" => mode = OutputMode::Object,
    "-o" => { output_file = Some(args[i+1].clone()); i += 1; }
    arg if arg.starts_with('-') => {} // ignore unknown flags
    _ => input_files.push(args[i].clone()),
}
```

Unknown flags are silently ignored for GCC compatibility.

### Compilation Pipeline

The `compile_to_assembly()` function encapsulates the full pipeline:

```
read file → preprocess → lex → parse → codegen → assembly string
```

For `-c` and executable modes, the assembly is written to a temp file and assembled using `gcc -c`. For linking, all object files are linked with `gcc`.

### Multi-File Compilation

When multiple input files are provided:
1. Each file is independently compiled to assembly
2. Each assembly file is assembled to an object file (via gcc -c)
3. All object files are linked together (via gcc)
4. Temporary files are cleaned up

## Backward Compatibility

When invoked with a single file and no `-o` flag (the legacy mode used by `test.sh`), assembly is output to stdout. This maintains compatibility with existing test infrastructure.

## Output File Naming

| Mode | Default Output |
|---|---|
| `-S input.c` | `input.s` |
| `-c input.c` | `input.o` |
| `input.c -o prog` | `prog` |
| `input.c` (no flags) | stdout |
