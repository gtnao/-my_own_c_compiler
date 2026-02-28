# Step 14.3: `__attribute__` Support (GCC Extension)

## Overview

Add support for the GCC `__attribute__` syntax by silently consuming and ignoring attribute annotations. This is essential for compiling real-world C code, especially PostgreSQL and system headers, which use `__attribute__` extensively.

## Common Uses in Real Code

```c
__attribute__((unused)) int x;
__attribute__((noreturn)) void fatal(const char *msg);
__attribute__((format(printf, 1, 2))) void log_msg(const char *fmt, ...);
__attribute__((packed)) struct S { ... };
__attribute__((aligned(16))) int data[4];
__attribute__((noinline)) int compute(int x);
```

## Implementation

### Token

Added `Attribute` token kind recognized by the lexer for the `__attribute__` keyword.

### Parser: `skip_attribute()`

A new method that consumes `__attribute__((...))` by tracking parenthesis depth:

```rust
fn skip_attribute(&mut self) {
    while self.current().kind == TokenKind::Attribute {
        self.advance(); // __attribute__
        if self.current().kind == TokenKind::LParen {
            self.advance(); // outer (
            let mut depth = 1;
            while depth > 0 {
                match self.current().kind {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    _ => {}
                }
                self.advance();
            }
        }
    }
}
```

The double parentheses `((...))` are naturally handled by the depth counter — the outer `(` sets depth to 1, the inner `(` to 2, and the closing `))`s bring it back to 0.

### Attribute Skip Points

`skip_attribute()` is called at these locations:
1. **Before `parse_type()`** — handles `__attribute__((unused)) int x`
2. **After pointer stars in `parse_type()`** — handles `int * __attribute__((may_alias)) p`
3. **After function parameter list `)` in `function_or_prototype()`** — handles `void f(int x) __attribute__((noreturn))`
4. **In `is_function()` lookahead** — correctly skips attributes when determining if a declaration is a function

### `is_type_keyword` and `is_function`

`Attribute` is added to `is_type_keyword` so that `__attribute__` before a type is recognized as part of a type declaration. The `is_function` lookahead also handles `Attribute` like `Alignas` — skipping the parenthesized content.

## Behavior

All attribute annotations are consumed and silently ignored. The compiler does not enforce or act on any attribute semantics (alignment, format checking, noreturn, etc.). This matches the behavior needed for compatibility — the attributes are informational to GCC/Clang but not required for correct compilation in most cases.

## Test Cases

```c
int main() __attribute__((unused)) { return 42; }  // after function name
__attribute__((unused)) int main() { return 5; }   // before return type
int __attribute__((noinline)) add(int a, int b) { return a + b; }  // between type and name
```
