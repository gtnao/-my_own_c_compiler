# Step 14.7: `_Noreturn` Keyword

## Overview

Add support for `_Noreturn` and `__noreturn__` function specifiers. These are consumed and ignored — the compiler does not perform any special optimization or verification based on the noreturn attribute.

## Why This Matters

PostgreSQL uses `_Noreturn` (via the `pg_noreturn` macro) to annotate functions like `ereport(ERROR, ...)` and `ExceptionalCondition()` that never return to their caller. System headers also use `__noreturn__` in `__attribute__` forms.

```c
_Noreturn void ExceptionalCondition(const char *conditionName, ...);
__noreturn__ void abort(void);
```

## Implementation

### Token

Added `Noreturn` variant to `TokenKind`.

### Lexer

Recognizes two spellings:
- `_Noreturn` — C11 standard keyword
- `__noreturn__` — GCC extension (double underscore form)

### Parser

`_Noreturn` is handled alongside `inline` as a function specifier — consumed and ignored before the type in `parse_type()`:

```rust
while matches!(self.current().kind, TokenKind::Inline | TokenKind::Noreturn) {
    self.advance();
}
```

Added to `is_type_keyword()` and `stmt()` type-start patterns so that declarations beginning with `_Noreturn` are properly recognized.

## Behavior

- `_Noreturn` is a hint to the compiler that the function does not return
- Our compiler ignores this hint — all functions are compiled with normal return paths
- This is sufficient for compatibility since `_Noreturn` only affects optimization and warnings

## Test Cases

```c
_Noreturn void exit_fn() { return; } int main() { return 5; }   // → 5
__noreturn__ void die() { return; } int main() { return 3; }    // → 3
```
