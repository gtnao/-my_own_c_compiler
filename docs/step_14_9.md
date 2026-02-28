# Step 14.9: `register` Storage Class

## Overview

Add support for the `register` storage class specifier. It is consumed and ignored — all local variables are stored on the stack regardless.

## Why This Matters

PostgreSQL and older C code use `register` to hint that a variable should be kept in a CPU register for faster access:

```c
register int i;
register unsigned char *p = buf;
```

Modern compilers ignore this hint and perform their own register allocation, but the keyword must be parsed for compatibility.

## Implementation

### Token

Added `Register` variant to `TokenKind`.

### Lexer

Recognizes `register` as a keyword, mapping to `TokenKind::Register`.

### Parser

`register` is consumed alongside `inline` and `_Noreturn` before the type in `parse_type()`:

```rust
while matches!(self.current().kind,
    TokenKind::Inline | TokenKind::Noreturn | TokenKind::Register) {
    self.advance();
}
```

Added to `is_type_keyword()` and `stmt()` type-start patterns.

## Behavior

- `register` is consumed and completely ignored
- Variables declared with `register` are allocated on the stack like any other local variable
- This matches the behavior of modern GCC/Clang which also ignore `register` as a storage hint

## Test Cases

```c
int main() { register int a = 5; return a; }    // → 5
int main() { register int i; i = 10; return i; } // → 10
```
