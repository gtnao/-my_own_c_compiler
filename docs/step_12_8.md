# Step 12.8: Multiple Variable Declarations on One Line

## Overview

Support declaring multiple variables of the same base type in a single statement:

```c
int a = 1, b = 2, c = 3;
int x, y;
```

## Implementation

After parsing the first variable declarator (with or without initializer), check for `,`. If found, continue parsing additional declarators until `;`:

```rust
if self.current().kind == TokenKind::Comma {
    let mut stmts = vec![first_var_decl];
    while self.current().kind == TokenKind::Comma {
        self.advance();
        // Parse pointer stars for this declarator
        // Parse name
        // Parse optional initializer
        stmts.push(next_var_decl);
    }
    self.expect(TokenKind::Semicolon);
    return Stmt::Block(stmts);
}
```

Each additional declarator can have its own pointer stars. For example:
```c
int a, *b, **c;  // a: int, b: int*, c: int**
```

The base type (`int` in this case) is shared, but each declarator independently adds pointer indirection.

## Desugaring

`int a = 1, b = 2;` is desugared into a Block:
```
Block([
    VarDecl { name: "a", ty: int, init: Some(1) },
    VarDecl { name: "b", ty: int, init: Some(2) },
])
```

## Test Cases

```c
int a = 1, b = 2; return a + b;                // => 3
int a = 1, b = 2, c = 3; return a + b + c;     // => 6
int a, b; a = 1; b = 2; return a + b;           // => 3
```
