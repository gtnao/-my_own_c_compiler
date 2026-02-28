# Step 15.1: Multi-Declarator with Arrays and Pointers

## Overview

Fix multi-variable declarations (comma-separated) to properly handle array dimensions in addition to pointer stars. Previously, `int a, b[3], *c;` would fail to parse the `b[3]` part.

## The Bug

The multi-declarator parsing code handled pointer modifiers (`*`) for each declarator but didn't parse array dimensions (`[N]`). So in `int a = 1, *b, c[3];`, the `c[3]` would not be recognized as an array, and `c` would be declared as plain `int`.

## Fix

Added array dimension parsing after the variable name in both multi-declarator paths (with and without initializer):

```rust
// After parsing the declarator name, check for array dimensions
while self.current().kind == TokenKind::LBracket {
    self.advance();
    if self.current().kind == TokenKind::RBracket {
        self.advance();
        decl_ty = Type { kind: TypeKind::Array(Box::new(decl_ty), 0), is_unsigned: false };
    } else {
        let size = self.eval_const_expr();
        self.expect(TokenKind::RBracket);
        decl_ty = Type { kind: TypeKind::Array(Box::new(decl_ty), size as usize), is_unsigned: false };
    }
}
```

## Also Updated

Added Phase 15-20 to PLAN.md covering all remaining features needed for PostgreSQL compilation:
- Phase 15: Advanced Declarations and Type System
- Phase 16: Preprocessor Extensions
- Phase 17: Standard Library Header Stubs
- Phase 18: GCC Extensions and Builtins
- Phase 19: Advanced Code Generation
- Phase 20: PostgreSQL Integration Testing

## Test Cases

```c
int a = 1, b = 2, c = 3; return a+b+c;           // → 6
int a, b, c; a=1; b=2; c=3; return a+b+c;         // → 6
int a = 1, *b, c[3]; b=&a; c[1]=20; return *b+c[1]; // → 21
```
