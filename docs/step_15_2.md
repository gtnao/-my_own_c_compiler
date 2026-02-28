# Step 15.2: long long Type

## Overview

`long long` and `unsigned long long` are already supported by treating them identically to `long`. On x86-64, both `long` and `long long` are 8 bytes, so no separate type variant is needed.

The parser already handles `long long` and `long long int` by consuming two consecutive `Long` tokens and an optional `Int` token:

```rust
TokenKind::Long => {
    self.advance();
    if self.current().kind == TokenKind::Long {
        self.advance();  // skip second "long"
        if self.current().kind == TokenKind::Int {
            self.advance();  // skip optional "int"
        }
    } else if self.current().kind == TokenKind::Int {
        self.advance();  // "long int"
    }
    if is_unsigned { Type::ulong() } else { Type::long_type() }
}
```

This step was already complete from previous implementation.
