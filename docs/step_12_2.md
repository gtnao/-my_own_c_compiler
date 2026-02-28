# Step 12.2: Struct Value Passing and Returning

## Overview

Enable structs to be passed by value to functions, returned from functions, and copied via assignment. Previously, structs were only passed as pointers (reference semantics). This step implements true value semantics using `rep movsb` for byte-level memory copy.

## Key Changes

### 1. Struct Assignment (`s2 = s1`)

When the left-hand side of an assignment is a struct type, both sides' addresses are computed, and the struct is byte-copied from source to destination:

```rust
// In gen_expr for Expr::Assign
if let TypeKind::Struct(_) = &lhs_ty.kind {
    self.gen_addr(rhs);      // source address → %rax
    self.push();             // save source address
    self.gen_addr(lhs);      // destination address → %rax
    self.emit("mov %rax, %rdi");  // %rdi = dst
    self.pop("%rsi");        // %rsi = src
    self.emit(&format!("mov ${}, %rcx", size));
    self.emit("rep movsb");  // copy size bytes
}
```

### 2. Struct Pass-by-Value (Function Parameters)

When a function receives a struct parameter, the caller passes the struct's address in a register. The callee copies the struct data into its own local stack space, ensuring modifications to the parameter don't affect the caller's original struct:

```asm
; Callee prologue for struct parameter:
  mov %rdi, %rsi         ; src = caller's struct address (from register)
  lea -offset(%rbp), %rdi ; dst = local stack space
  mov $size, %rcx        ; byte count
  rep movsb              ; copy struct into local frame
```

This is critical for value semantics. Without the copy, `modify(s)` would alter the caller's struct `s`, violating C semantics.

### 3. Struct Return from Functions

When a function returns a struct (`return p;`), the struct's address is placed in `%rax`. The caller then copies the returned struct into its local variable using `emit_store_var`, which detects struct types and performs a `rep movsb` copy.

### 4. `emit_store_var` Enhancement for Structs

Previously, `emit_store_var` had a no-op case for `Struct(_)`. Now it performs a full struct copy when the target variable is a struct:

```rust
if let TypeKind::Struct(_) = &ty.kind {
    self.emit("mov %rax, %rsi");    // src = address in %rax
    self.emit(&format!("lea -{}(%rbp), %rdi", offset)); // dst
    self.emit(&format!("mov ${}, %rcx", size));
    self.emit("rep movsb");
    return;
}
```

### 5. `emit_store_indirect` Enhancement for Structs

For indirect struct stores (e.g., through pointers or member access), `emit_store_indirect` now handles struct types by treating `%rax` as the source address and `%rdi` as the destination address:

```rust
if let TypeKind::Struct(_) = &ty.kind {
    self.emit("mov %rax, %rsi"); // src
    self.emit(&format!("mov ${}, %rcx", size));
    self.emit("rep movsb");
    return;
}
```

### 6. Struct Expression Values

When a struct expression appears in a value context (e.g., `Expr::Var`, `Expr::Member`, `Expr::Deref`), the expression evaluates to the struct's **address** rather than loading its contents. This is analogous to array-to-pointer decay:

- `Expr::Var(name)` for a struct → `lea -offset(%rbp), %rax`
- `Expr::Member(base, name)` for a struct member → address computation only
- `Expr::Deref(ptr)` for a struct pointer → leaves pointer value in `%rax`

### 7. Standalone Struct Definitions

Added support for top-level struct definitions without a variable name:

```c
struct P { int x; int y; };  // Just defines the struct tag
```

The parser now handles this by checking for `;` immediately after `parse_type()` in `global_var()`.

## The `rep movsb` Instruction

`rep movsb` is an x86 string instruction that copies bytes from `%rsi` (source) to `%rdi` (destination), decrementing `%rcx` until it reaches zero:

```
; Before: %rsi = src, %rdi = dst, %rcx = count
rep movsb
; After: %rcx = 0, %rsi and %rdi advanced by count
```

This is the simplest approach for arbitrary-size struct copies. Modern CPUs optimize `rep movsb` internally (ERMS - Enhanced REP MOVSB), making it competitive with hand-written copy loops for most struct sizes.

## Value Semantics Verification

The test `modify(a)` confirms that pass-by-value works correctly:

```c
struct P { int x; int y; };
void modify(struct P p) { p.x = 99; }  // modifies local copy only
int main() {
    struct P a; a.x = 3; a.y = 4;
    modify(a);
    return a.x;  // => 3 (unchanged)
}
```

## Test Cases

```c
// Struct assignment
struct { int x; int y; } s1, s2;
s1.x = 1; s1.y = 2; s2 = s1; return s2.x + s2.y;  // => 3

// Tagged struct copy
struct P { int x; int y; };
struct P a; a.x = 3; a.y = 7; struct P b; b = a; return b.x + b.y;  // => 10

// Struct return
struct P make() { struct P p; p.x = 1; p.y = 2; return p; }
struct P r = make(); return r.x + r.y;  // => 3

// Struct pass-by-value
int sum(struct P p) { return p.x + p.y; }
struct P a; a.x = 3; a.y = 4; return sum(a);  // => 7

// Value semantics (no aliasing)
void modify(struct P p) { p.x = 99; }
struct P a; a.x = 3; modify(a); return a.x;  // => 3

// Struct with mixed types
struct S { char c; int n; };
int get(struct S s) { return s.c; }
struct S s; s.c = 97; s.n = 42; return get(s);  // => 97
```
