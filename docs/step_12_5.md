# Step 12.5: volatile Qualifier

## Overview

The `volatile` qualifier tells the compiler that a variable's value may change at any time (e.g., memory-mapped I/O, signal handlers). In our compiler, `volatile` is parsed and ignored — every memory access already goes through load/store instructions without optimization, so volatile semantics are naturally satisfied.

## Implementation

`volatile` is handled identically to `const` — it's consumed during type parsing and has no effect on code generation:

```rust
while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile) {
    self.advance();
}
```

## Why No Special Handling

In an optimizing compiler, `volatile` prevents:
- Caching values in registers across memory accesses
- Reordering reads/writes to volatile variables
- Eliminating "redundant" reads

Since our compiler doesn't optimize (every variable access generates a memory load/store instruction), volatile semantics are already satisfied without any special handling.

## Test Cases

```c
volatile int a = 5; return a;  // => 5
```
