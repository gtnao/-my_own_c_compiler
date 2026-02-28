# Step 14.8: `restrict` Type Qualifier

## Overview

Add support for `restrict`, `__restrict`, and `__restrict__` type qualifiers. These are consumed and ignored — the compiler does not perform any alias analysis optimization.

## Why This Matters

PostgreSQL uses `restrict` (or `__restrict__` via GCC) in performance-critical pointer parameters to hint that pointers don't alias:

```c
void memcpy(void * restrict dest, const void * restrict src, size_t n);
int * __restrict__ pg_ptr;
```

System headers (especially `<string.h>`, `<stdlib.h>`) also use `restrict` extensively.

## Implementation

### Token

Added `Restrict` variant to `TokenKind`.

### Lexer

Recognizes three spellings:
- `restrict` — C99 standard keyword
- `__restrict` — GCC extension
- `__restrict__` — GCC extension (double underscore form)

### Parser

`restrict` is treated as a type qualifier alongside `const` and `volatile`. It is consumed and ignored in all qualifier-skipping loops:

1. **`parse_type()`** — skipped before the base type (qualifiers before type)
2. **Pointer qualifiers** — skipped after `*` (e.g., `int * restrict p`)
3. **Parameter qualifiers** — skipped in function parameter type parsing
4. **`is_type_keyword()`** — recognized as part of type declarations
5. **`stmt()`** — recognized as starting a variable declaration

All four qualifier-skipping locations in the parser now include `Restrict`:
```rust
while matches!(self.current().kind,
    TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict | TokenKind::Alignas) {
    // ...
}
```

## Behavior

- `restrict` is a C99 optimization hint for pointer aliasing
- Our compiler ignores this hint — no alias analysis is performed
- The qualifier is simply consumed during parsing
- `int * restrict p` is equivalent to `int *p` in our compiler

## Test Cases

```c
int main() { int a = 5; int * restrict p = &a; return *p; }      // → 5
int main() { int a = 7; int * __restrict p = &a; return *p; }    // → 7
int main() { int a = 9; int * __restrict__ p = &a; return *p; }  // → 9
```
