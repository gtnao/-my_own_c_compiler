# Step 14.6: `signed` Keyword

## Overview

Add support for the `signed` type specifier. `signed` is the default signedness for integer types, so `signed int` is equivalent to `int`, `signed char` to `char`, etc. The bare `signed` keyword (without a following type) is treated as `signed int`.

## Why This Matters

PostgreSQL and system headers occasionally use `signed` explicitly:

```c
signed char sc;
signed int x;
signed long val;
```

While `signed` is redundant for `int`/`short`/`long` (they are signed by default), `signed char` is distinct from `char` on some platforms where `char` may be unsigned by default.

## Implementation

### Token

Added `Signed` variant to `TokenKind`.

### Lexer

Recognizes `signed` as a keyword, mapping to `TokenKind::Signed`.

### Parser

`signed` is handled alongside `unsigned` in `parse_type()`:

```rust
let mut has_signedness = false;
let is_unsigned = if self.current().kind == TokenKind::Unsigned {
    self.advance();
    has_signedness = true;
    true
} else {
    if self.current().kind == TokenKind::Signed {
        self.advance();
        has_signedness = true;
    }
    false
};
```

The `has_signedness` flag is used to handle bare `signed` (without a following type keyword):
- `signed int` → `int` (normal signed int)
- `signed char` → `char` (signed char)
- `signed` alone → `int` (just like bare `unsigned` → `unsigned int`)

Added `TokenKind::Signed` to:
- `is_type_keyword()` — recognized as part of type declarations
- `stmt()` match — recognized as starting a variable declaration

## Behavior

- `signed` is consumed and does not change the resulting type (since all integer types are signed by default in our compiler)
- `signed` can precede `int`, `char`, `short`, `long`, or appear alone
- Bare `signed` defaults to `signed int`

## Test Cases

```c
int main() { signed int a = 5; return a; }    // → 5
int main() { signed char c = 3; return c; }    // → 3
int main() { signed a = 7; return a; }         // → 7 (bare signed = int)
int main() { signed short s = 10; return s; }  // → 10
int main() { signed long l = 42; return l; }   // → 42
```
