# Step 12.4: const Qualifier

## Overview

The `const` qualifier indicates that a variable's value should not be modified after initialization. In our compiler, `const` is parsed and recognized but not enforced — it has no effect on code generation.

## Implementation

`const` is treated as a type qualifier that is simply consumed and ignored during type parsing:

```rust
// At the start of parse_type()
while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile) {
    self.advance();
}
```

It can also appear after pointer stars:
```c
int *const p;   // const pointer to int
const int *p;   // pointer to const int
```

Both positions are handled:
```rust
// After each * in pointer parsing
while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile) {
    self.advance();
}
```

## Why No Enforcement

Real C compilers use `const` for:
1. Compile-time error checking (assigning to a const variable)
2. Optimization (placing const globals in read-only sections)

Our compiler doesn't enforce const-correctness because:
- It would require tracking constness in the type system
- The primary goal is correct code generation, not error checking
- Adding enforcement can be done later without changing code generation

## Test Cases

```c
const int a = 42; return a;          // => 42
const int *p; int a = 3; p = &a; return *p;  // => 3
```
