# Step 11.2: Variadic Arguments (`...`, `va_list`, `va_start`, `va_arg`)

## Overview

This step implements variadic function support, allowing functions to accept a variable number of arguments. This is essential for functions like `printf` and custom variadic functions.

The key components are:
- `...` (ellipsis) in function parameter lists to declare variadic functions
- `va_list` type for iterating over variadic arguments
- `va_start(ap, last_param)` to initialize a `va_list`
- `va_arg(ap, type)` to retrieve the next argument
- `va_end(ap)` to clean up (no-op in our implementation)

## System V AMD64 ABI and Register Save Area

On x86-64 Linux (System V AMD64 ABI), the first 6 integer/pointer arguments are passed in registers:

| Register | Argument index |
|----------|---------------|
| `%rdi`   | 0             |
| `%rsi`   | 1             |
| `%rdx`   | 2             |
| `%rcx`   | 3             |
| `%r8`    | 4             |
| `%r9`    | 5             |

For variadic functions, we need to save all register arguments to a contiguous memory area (the **register save area**) so that `va_arg` can iterate through them sequentially.

### Register Save Area Layout

We allocate 48 bytes (6 registers × 8 bytes each) on the stack:

```
Higher addresses (toward %rbp)
┌──────────────────────────────────┐
│ %r9  (arg 5)  [rbp - base + 40] │
├──────────────────────────────────┤
│ %r8  (arg 4)  [rbp - base + 32] │
├──────────────────────────────────┤
│ %rcx (arg 3)  [rbp - base + 24] │
├──────────────────────────────────┤
│ %rdx (arg 2)  [rbp - base + 16] │
├──────────────────────────────────┤
│ %rsi (arg 1)  [rbp - base +  8] │
├──────────────────────────────────┤
│ %rdi (arg 0)  [rbp - base     ] │  ← va_save_area_offset
└──────────────────────────────────┘
Lower addresses (toward %rsp)
```

Where `base` is `va_save_area_offset` — the distance from `%rbp` to the start of the save area (the lowest address, where `%rdi` is stored).

Arguments are stored in ascending address order: `%rdi` at the lowest address, `%r9` at the highest. This means advancing through arguments requires **adding** 8 to the pointer.

### Function Prologue for Variadic Functions

```asm
  push %rbp
  mov %rsp, %rbp
  sub $N, %rsp           # N includes locals + 48 bytes for save area

  # Save all 6 register arguments to the save area
  mov %rdi, -72(%rbp)    # arg 0 (example offset)
  mov %rsi, -64(%rbp)    # arg 1
  mov %rdx, -56(%rbp)    # arg 2
  mov %rcx, -48(%rbp)    # arg 3
  mov %r8,  -40(%rbp)    # arg 4
  mov %r9,  -32(%rbp)    # arg 5
```

## `va_list` Implementation

`va_list` is implemented as `char *` — a simple pointer into the register save area. This is a simplified version of the full ABI's `va_list` structure (which includes `gp_offset`, `fp_offset`, `overflow_arg_area`, and `reg_save_area`), but it works for up to 6 integer arguments.

## `va_start` Implementation

`va_start(ap, last_param)` initializes `ap` to point to the first **unnamed** argument in the register save area.

If the function has `n` named parameters, the first unnamed argument is at register index `n`, which is at offset `va_save_area_offset - n * 8` from `%rbp`.

```asm
  # va_start(ap, last_param) where function has 1 named param
  lea -(va_save_area_offset - 1*8)(%rbp), %rax   # address of arg 1 in save area
  # Store this address into ap variable
  push %rax
  lea -ap_offset(%rbp), %rax    # address of ap
  mov %rax, %rdi
  pop %rax
  mov %rax, (%rdi)              # ap = &save_area[param_count]
```

## `va_arg` Implementation

`va_arg(ap, type)` reads the value at the current `ap` position, then advances `ap` by 8 bytes.

```asm
  # va_arg(ap, int)
  lea -ap_offset(%rbp), %rax    # address of ap variable
  mov %rax, %rcx                # save address of ap in %rcx
  mov (%rcx), %rdi              # load current ap value (pointer to next arg)
  movslq (%rdi), %rax           # load int value from *ap (sign-extended)
  push %rax                     # save the loaded value
  add $8, %rdi                  # advance ap to next argument
  mov %rdi, (%rcx)              # store updated ap back
  pop %rax                      # restore loaded value to %rax
```

The advancement direction is `add $8` because arguments are stored in ascending address order (arg 0 at lowest address, arg 5 at highest).

For different types:
- `int`: `movslq (%rdi), %rax` (sign-extend 32-bit to 64-bit)
- `long`/pointers: `mov (%rdi), %rax` (full 64-bit load)
- `char`: `movsbl (%rdi), %eax` (sign-extend 8-bit)

## `va_end` Implementation

`va_end(ap)` is a no-op — the register save area is part of the stack frame and is automatically cleaned up when the function returns.

In the parser, `va_end(expr)` simply evaluates the expression (for side effects) and discards the result by generating a `Num(0)` node.

## Parser Changes

### Ellipsis Token

A new `Ellipsis` token kind is added for `...`:

```rust
// In lexer: recognize three consecutive dots
if ch == '.' && self.peek_next() == Some('.') && self.peek_at(2) == Some('.') {
    self.pos += 3;
    tokens.push(Token { kind: TokenKind::Ellipsis, pos });
    continue;
}
```

### Function Declaration

The parser checks for `...` after the last named parameter:

```rust
// After parsing named parameters
if self.current().kind == TokenKind::Ellipsis {
    is_variadic = true;
    self.advance();
}
```

### `va_list` as a Type

`va_list` is recognized as a type keyword and maps to `char *` (pointer to char):

```rust
if name == "va_list" {
    Type::ptr_to(Type::char_type())
}
```

### Built-in Function Handling

`va_start`, `va_arg`, and `va_end` are parsed as special built-in expressions in `primary()`:

- `va_start(ap, last_param)` → `Expr::VaStart { ap, last_param }`
- `va_arg(ap, type)` → `Expr::VaArg { ap, ty }`
- `va_end(ap)` → `Expr::Num(0)` (no-op)

## AST Changes

Two new expression nodes:

```rust
pub enum Expr {
    // ...
    VaStart {
        ap: Box<Expr>,
        last_param: String,
    },
    VaArg {
        ap: Box<Expr>,
        ty: Type,
    },
}
```

The `Function` struct gains an `is_variadic` field:

```rust
pub struct Function {
    pub name: String,
    pub return_ty: Type,
    pub params: Vec<(Type, String)>,
    pub is_variadic: bool,   // NEW
    pub body: Vec<Stmt>,
    pub locals: Vec<(Type, String)>,
}
```

## Code Generation Changes

The `Codegen` struct gains two new fields:

```rust
struct Codegen {
    // ...
    va_save_area_offset: usize,       // offset from rbp to register save area
    current_func_param_count: usize,  // number of named parameters
}
```

### Stack Frame Layout for Variadic Functions

```
                    %rbp
┌─────────────────┐  ↑
│ saved %rbp      │  │
├─────────────────┤  │
│ local variables │  │  (normal locals)
├─────────────────┤  │
│ register params │  │  (named params stored on stack)
├─────────────────┤  │
│ save area (48B) │  │  (all 6 register args saved here)
├─────────────────┤  │
│ alignment pad   │  │
└─────────────────┘  ↓  %rsp
```

## Test Cases

```c
// Basic variadic sum
int sum(int n, ...) {
    va_list ap;
    va_start(ap, n);
    int total = 0;
    int i;
    for (i = 0; i < n; i++)
        total += va_arg(ap, int);
    va_end(ap);
    return total;
}
int main() { return sum(3, 10, 20, 30); }  // => 60

int main() { return sum(3, 1, 2, 3); }     // => 6
```

## Limitations

1. **Maximum 5 variadic arguments**: Since we only save 6 register arguments and one is used for the named parameter `n`, only 5 variadic arguments can be accessed. Arguments beyond 6 total are passed on the stack and are not handled.

2. **No floating-point support**: The register save area only saves general-purpose registers. Floating-point arguments passed in `%xmm0`–`%xmm7` are not saved.

3. **Simplified `va_list`**: The real ABI uses a struct with `gp_offset`, `fp_offset`, `overflow_arg_area`, and `reg_save_area`. Our implementation uses a simple `char *` pointer.

4. **No stack-passed argument support**: Arguments 7+ that are passed on the stack are not accessible through `va_arg`.
