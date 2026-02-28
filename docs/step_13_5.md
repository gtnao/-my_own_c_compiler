# Step 13.5: Debug Information (.file Directive)

## Overview

Add `.file` assembly directive to the generated output, enabling basic debug information for tools like GDB and `objdump`.

## Implementation

### Codegen Changes

The `Codegen` struct now takes a filename parameter:

```rust
pub fn new(filename: &str) -> Self { ... }
```

At the start of `generate()`, a `.file` directive is emitted:

```asm
  .file "example.c"
```

This tells the assembler which source file the assembly was generated from. When compiled with `gcc -g`, this information is embedded in the resulting binary's DWARF debug info.

### Main Entry Point

Updated `main.rs` to pass the filename to `Codegen`:

```rust
let mut codegen = Codegen::new(filename);
```

## Debug Information in Practice

The `.file` directive is the most basic level of debug information. With this directive:

- `objdump -d` can show the source file name
- GDB can display the filename in stack traces
- `addr2line` can map addresses back to the source file

For more detailed debugging (stepping through source lines), `.loc` directives would be needed at each statement, which requires line number tracking in the AST. This is left for future improvement.

## Assembly Output

```asm
  .file "hello.c"
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  ...
```
