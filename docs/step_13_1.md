# Step 13.1: Constant Folding

## Overview

Constant folding is a compile-time optimization that evaluates constant expressions during parsing instead of generating code to compute them at runtime.

Before:
```asm
  mov $3, %rax    # load 3
  push %rax
  mov $2, %rax    # load 2
  pop %rdi
  add %rdi, %rax  # compute 2+3 at runtime
```

After:
```asm
  mov $5, %rax    # result computed at compile time
```

## Implementation

A helper method `make_binop` is used instead of directly constructing `Expr::BinOp` nodes. When both operands are `Expr::Num`, the result is computed at compile time and returned as `Expr::Num`:

```rust
fn make_binop(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
    if let (Expr::Num(l), Expr::Num(r)) = (&lhs, &rhs) {
        let result = match op {
            BinOp::Add => l.wrapping_add(*r),
            BinOp::Sub => l.wrapping_sub(*r),
            BinOp::Mul => l.wrapping_mul(*r),
            // ... all operators ...
            _ => return Expr::BinOp { ... };
        };
        return Expr::Num(result);
    }
    Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }
}
```

## Supported Operations

All binary operations are folded:
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Bitwise: `&`, `|`, `^`, `<<`, `>>`

Division and modulo by zero are not folded (they produce runtime code that would crash, matching GCC behavior for constant division by zero).

## Cascading Folding

Because folding happens during parsing (in the recursive descent), it cascades naturally:

```c
return 1 + 2 + 3;
```

1. Parse `1 + 2` → `make_binop(Add, Num(1), Num(2))` → `Num(3)`
2. Parse `Num(3) + 3` → `make_binop(Add, Num(3), Num(3))` → `Num(6)`

Result: `mov $6, %rax`

## Wrapping Arithmetic

All operations use Rust's `wrapping_*` methods to match C's behavior for signed integer overflow (implementation-defined but typically wrapping on two's complement machines).

## What's NOT Folded

- Unary operations (e.g., `-1` is already parsed as `Num(-1)` by the lexer for negative literals, or as `UnaryOp(Neg, Num(1))` which is not folded)
- Expressions involving variables
- Short-circuit operators (`&&`, `||`)
- Ternary operator (`? :`)
