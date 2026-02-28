# Step 15.4: K&R Style Function Declarations

## Overview

K&R (Kernighan & Ritchie) style function declarations use an older C syntax where parameter types are declared after the parameter list, before the function body:

```c
int add(a, b) int a; int b; { return a + b; }
```

Modern style equivalent:
```c
int add(int a, int b) { return a + b; }
```

## Implementation

The parser detects K&R style by checking if the first token in the parameter list is an identifier that is NOT a type name:

1. **K&R parameter list parsing**: When we see `(ident, ident, ...)` where `ident` is not a type, parse parameter names with default `int` type
2. **Post-paren type declarations**: After `)`, if we see type keywords before `{`, parse `type name;` declarations and update the corresponding parameter types

```rust
// Detect K&R: first token is identifier, not a type name
if let TokenKind::Ident(_) = self.current().kind {
    if !self.is_type_start(&self.current().kind) {
        // K&R: parse (a, b, c) as int-typed parameters
        is_kr_style = true;
        // ... collect parameter names with default int type
    }
}

// After RParen, parse K&R type declarations
// int add(a, b) int a; int b; { ... }
//               ^^^^^^^^^^^^^^^
while self.current().kind != TokenKind::LBrace {
    let kr_ty = self.parse_type();
    // Update matching parameter's type
}
```

## Test Cases

```c
int add(a, b) int a; int b; { return a+b; }
int main() { return add(3, 4); }  // => 7

int mul(x, y) int x; int y; { return x*y; }
int main() { return mul(2, 3); }  // => 6
```
