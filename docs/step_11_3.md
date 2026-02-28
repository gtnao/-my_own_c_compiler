# Step 11.3: Function Pointers

## Overview

This step implements function pointers, enabling functions to be stored in variables and called indirectly. Function pointers are essential for implementing callbacks, dispatch tables, and higher-order programming patterns in C.

Key features:
- Function pointer declaration: `int (*fp)(int, int)`
- Function-to-pointer decay: `fp = add` (bare function name becomes a pointer)
- Indirect function call: `fp(3, 4)` calls via the pointer

## Function Pointer Declaration Syntax

C's function pointer syntax is notoriously complex:

```c
int (*fp)(int, int);
```

This reads as: `fp` is a **pointer** (`*`) to a **function** taking `(int, int)` and returning `int`.

The parentheses around `*fp` are critical — without them, `int *fp(int, int)` would declare a function returning `int *`.

## Implementation

### Type Representation

Function pointers are internally represented as `Ptr(Void)` — a simple 8-byte pointer. We don't track the full function signature in the type system. This is a simplification that works because:

1. x86-64 function pointers are always 8 bytes regardless of signature
2. The calling convention is the same for all function types
3. Type checking for function pointer arguments is not yet implemented

### AST Changes

A new `FuncPtrCall` expression node is added to distinguish indirect calls from direct calls:

```rust
pub enum Expr {
    // Direct function call: call label
    FuncCall { name: String, args: Vec<Expr> },
    // Indirect function call: call *%r10
    FuncPtrCall { fptr: Box<Expr>, args: Vec<Expr> },
    // ...
}
```

### Parser Changes

#### Function Pointer Declaration Parsing

In `var_decl()`, after parsing the base type, we check for the `(*` pattern to identify function pointer declarations:

```rust
// After parse_type() returns the return type
if current == '(' && next == '*' {
    parse_func_ptr_decl(return_ty);
}
```

The `parse_func_ptr_decl` method:
1. Consumes `(` `*` `name` `)`
2. Parses the parameter type list `(type, type, ...)`
3. Creates a variable with type `Ptr(Void)`
4. Handles optional initializer `= expr`

#### Function-to-Pointer Decay

When a function name appears as an expression (e.g., `fp = add`), and `add` is not a declared variable, `emit_load_var` treats it as a function name and generates:

```asm
lea add(%rip), %rax
```

This loads the function's address into `%rax` using RIP-relative addressing.

#### Distinguishing Direct vs Indirect Calls

In `primary()`, when parsing `name(args)`:
- If `name` is a declared variable → `FuncPtrCall` (indirect call)
- Otherwise → `FuncCall` (direct call)

The `is_var_declared()` method checks local scopes and global variables, but excludes `extern` declarations (which are function prototypes, not pointer variables).

### Code Generation

#### Direct Call (existing)

```asm
call function_name
```

#### Indirect Call via Function Pointer

```asm
  mov -8(%rbp), %rax    # load function pointer from variable
  mov %rax, %r10        # save to %r10 (caller-saved, not used for args)

  # ... set up arguments in registers ...

  mov $0, %al           # clear AL (no vector register args)
  call *%r10            # indirect call through %r10
```

We use `%r10` to hold the function pointer because:
- It's a caller-saved register (callee can clobber it)
- It's NOT used for argument passing (args use `%rdi`–`%r9`)
- It's not clobbered by our argument evaluation code (which only uses `%rax`, `%rdi`, and the stack)

The `call *%r10` instruction performs an indirect call — it reads the address from `%r10` and jumps to it.

### Stack Alignment

The indirect call follows the same 16-byte stack alignment requirements as direct calls. The alignment check uses `stack_depth` to determine whether an extra 8-byte pad is needed before the call.

## Example

### C Source

```c
int add(int a, int b) { return a + b; }
int main() {
    int (*fp)(int, int) = add;
    return fp(3, 4);
}
```

### Generated Assembly

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # int (*fp)(int, int) = add;
  lea add(%rip), %rax     # get address of 'add' function
  mov %rax, -8(%rbp)      # store in fp variable

  # fp(3, 4)
  mov -8(%rbp), %rax      # load fp
  mov %rax, %r10          # save to %r10
  mov $3, %rax            # arg 1
  push %rax
  mov $4, %rax            # arg 2
  push %rax
  pop %rsi                # arg 2 → %rsi
  pop %rdi                # arg 1 → %rdi
  mov $0, %al
  call *%r10              # indirect call

  jmp .Lreturn.main
```

## Test Cases

```c
// Basic function pointer call
int add(int a, int b) { return a + b; }
int main() { int (*fp)(int, int) = add; return fp(3, 4); }  // => 7

// Function pointer with different function
int sub(int a, int b) { return a - b; }
int main() { int (*fp)(int, int) = sub; return fp(5, 3); }  // => 2

// Nullary function pointer
int ret42() { return 42; }
int main() { int (*fp)() = ret42; return fp(); }  // => 42
```

## Limitations

1. **No signature tracking**: The type system stores function pointers as `Ptr(Void)`, so there's no compile-time type checking of argument types or count.

2. **No `typedef` for function pointer types**: Patterns like `typedef int (*BinOp)(int, int);` are not yet supported.

3. **`%r10` clobbering**: If a function pointer call has arguments that themselves involve function calls, the `%r10` register could be clobbered. This doesn't occur in typical usage since argument expressions are usually simple.
