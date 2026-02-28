# Step 15.3: Complex Type Declarators

## Overview

This step enhances the parser to handle complex C type declarations:
- **Pointer arrays**: `int *arr[3]` — array of 3 pointers to int
- **Array pointers**: `int (*p)[3]` — pointer to array of 3 ints
- **Function pointer arrays**: `int (*ops[2])(int, int)` — array of 2 function pointers

## C Declaration Syntax

C's declaration syntax follows the "declaration mirrors use" principle. The declarator has two forms when parentheses are involved:

### `int *arr[3]` — Pointer Array (already worked)
`*` binds to the base type, `[3]` is part of the declarator. `parse_type()` reads `int *` as `Ptr(Int)`, then `var_decl()` parses `arr[3]` creating `Array(Ptr(Int), 3)`.

### `int (*p)[3]` — Array Pointer
The parentheses group `*p` together, so `p` is a pointer to `int[3]`. `parse_func_ptr_or_array_ptr_decl()` handles this: after `(*name)`, it sees `[3]` and creates `Ptr(Array(Int, 3))`.

### `int (*ops[2])(int, int)` — Function Pointer Array
The key insight: `ops[2]` is inside the parens with `*`, meaning each element of `ops` is a pointer. The `(int, int)` suffix makes it a function pointer. Result: `Array(Ptr(Void), 2)` — array of 2 function pointers.

## Implementation

The `parse_func_ptr_or_array_ptr_decl()` method was extended to handle array dimensions inside the parentheses:

```rust
fn parse_func_ptr_or_array_ptr_decl(&mut self, base_ty: Type) -> Stmt {
    self.expect(TokenKind::LParen);  // (
    self.expect(TokenKind::Star);    // *
    // ... parse name ...

    // Check for array dimension inside parens: (*name[N])
    let mut array_size: Option<usize> = None;
    if self.current().kind == TokenKind::LBracket {
        self.advance();
        let size = self.eval_const_expr();
        self.expect(TokenKind::RBracket);
        array_size = Some(size as usize);
    }

    self.expect(TokenKind::RParen);  // )

    // Then dispatch based on what follows:
    // - [N] → array pointer (without inner array_size)
    // - (params) → function pointer or function pointer array (with inner array_size)
}
```

When `array_size` is `Some(N)` and `(params)` follows, we create `Array(Ptr(Void), N)` — a function pointer array. Each 8-byte element is a generic function pointer.

## Test Cases

```c
// Pointer array
int main() {
    int a=1, b=2, c=3;
    int *arr[3];
    arr[0]=&a; arr[1]=&b; arr[2]=&c;
    return *arr[1];  // => 2
}

// Array pointer
int main() {
    int arr[3] = {10, 20, 30};
    int (*p)[3] = &arr;
    return (*p)[1];  // => 20
}

// Function pointer array
int add(int a, int b) { return a+b; }
int sub(int a, int b) { return a-b; }
int main() {
    int (*ops[2])(int, int);
    ops[0] = add;
    ops[1] = sub;
    return ops[0](10, 3);  // => 13
}
```
