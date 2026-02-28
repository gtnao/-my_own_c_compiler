# Step 14.4: Inline and Static Inline Functions

## Overview

Add support for `inline`, `static inline`, `__inline`, and `__inline__` function qualifiers. These are consumed and ignored — functions are always compiled normally (never actually inlined at the compiler level).

## Why This Matters

PostgreSQL and system headers use `static inline` extensively for small utility functions in headers:

```c
static inline int Max(int a, int b) { return a > b ? a : b; }
static inline void *palloc(size_t size) { ... }
```

GCC also uses `__inline` and `__inline__` variants in its built-in headers.

## Implementation

### Token

Added `Inline` token kind.

### Lexer

Recognizes three spellings:
- `inline` — C99 standard
- `__inline` — GCC extension
- `__inline__` — GCC extension (double underscore form)

### Parser

`inline` is treated as a type qualifier and consumed/ignored in:

1. **`parse_type()`** — skipped alongside `__attribute__` before the base type
2. **`is_type_keyword()`** — recognized as part of type declarations
3. **`stmt()`** — recognized as starting a variable/function declaration
4. **Top-level `parse()`** — `static` at top level is consumed, then `inline` is handled by `parse_type()` → the function is parsed normally

### Top-level `static`

Previously, `static` at the top level was only handled for local static variables. Now, a top-level `static` keyword is simply consumed so that `static inline int foo()` and `static int x` work correctly — they're treated the same as their non-static counterparts.

## Behavior

- `inline` is a hint to the compiler, not a requirement
- Our compiler never actually inlines functions — all functions are compiled as separate symbols
- `static inline` functions are emitted as global symbols (not truly static/local linkage)
- This matches the minimum behavior needed for compatibility

## Test Cases

```c
static inline int add(int a, int b) { return a + b; }
int main() { return add(2, 4); }  // → 6

inline int dbl(int x) { return x * 2; }
int main() { return dbl(5); }  // → 10
```
