# Step 13.3: Register Allocation Improvements

## Overview

Document the current register usage patterns and identify future improvement opportunities. The compiler currently uses a stack-machine model where all intermediate values pass through `%rax` and binary operations use `push`/`pop` for operand handling.

## Current Register Usage

### Dedicated Registers

| Register | Usage |
|---|---|
| `%rax` | Primary accumulator — all expression results |
| `%rdi` | Binary operation RHS (popped from stack) |
| `%rsp` | Stack pointer |
| `%rbp` | Frame pointer |
| `%rcx` | Shift counts, `rep movsb` counter, bit-field masks |
| `%rsi` | `rep movsb` source |
| `%r10` | Function pointer indirect calls |
| `%al` | Variadic function flag (SSE register count) |

### Calling Convention (System V AMD64 ABI)

Arguments: `%rdi`, `%rsi`, `%rdx`, `%rcx`, `%r8`, `%r9`
Return: `%rax`
Callee-saved: `%rbx`, `%r12`-`%r15`, `%rbp`
Caller-saved: `%rax`, `%rcx`, `%rdx`, `%rsi`, `%rdi`, `%r8`-`%r11`

### Available Unused Registers

The following registers are currently unused and could be leveraged:

- `%rbx` (callee-saved) — could cache frequently accessed variables
- `%r11` (caller-saved) — could be used as a second temporary
- `%r12`-`%r15` (callee-saved) — could hold loop variables

## Peephole Optimizations Already Applied (Step 13.2)

- Adjacent `push %rax; pop %reg` → `mov %rax, %reg`
- Adjacent `push %rax; pop %rax` → removed

## Constant Folding Already Applied (Step 13.1)

- Compile-time evaluation of constant expressions eliminates runtime instructions entirely

## Future Register Allocation Strategies

### Simple Variable Caching

For variables accessed multiple times in a basic block, load once into a callee-saved register:

```asm
; Before (current):
  movslq -4(%rbp), %rax    ; load 'a'
  push %rax
  movslq -4(%rbp), %rax    ; load 'a' again (redundant)
  pop %rdi
  add %rdi, %rax            ; a + a

; After (with caching):
  movslq -4(%rbp), %rbx    ; load 'a' into %rbx
  lea (%rbx, %rbx), %rax   ; a + a without stack operations
```

### Linear Scan Register Allocation

A more sophisticated approach would assign variables to registers across their live ranges:

1. Compute live ranges for each variable
2. Sort by start position
3. Assign available registers, spilling to stack when necessary

This would require fundamental changes to the code generation architecture and is deferred to a future phase.

## Impact Assessment

The current stack-machine approach generates correct but not optimal code. For a compiler targeting PostgreSQL compilation (where correctness is paramount), the current approach is acceptable. The peephole pass (Step 13.2) addresses the most common redundancies.
