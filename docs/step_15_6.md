# Step 15.6–15.8: Abstract Declarators, typeof, and _Static_assert

## Step 15.6: Abstract Declarators in Function Prototypes

Function prototypes can omit parameter names (abstract declarators):

```c
// Prototype with no parameter names
int apply(int (*)(int, int), int, int);

// Definition with parameter names
int apply(int (*f)(int, int), int a, int b) { return f(a, b); }
```

### Implementation

Two changes were needed:

1. **Anonymous function pointer parameters**: When parsing `(*` in a parameter and no identifier follows, generate a unique dummy name:
```rust
let param_name = match &self.current().kind {
    TokenKind::Ident(s) => { ... }
    _ => {
        // Anonymous function pointer parameter
        self.unique_counter += 1;
        format!("__anon_fptr.{}", self.unique_counter)
    }
};
```

2. **Anonymous typed parameters**: When a type keyword is followed by `,` or `)` with no identifier:
```rust
TokenKind::Comma | TokenKind::RParen => {
    // Abstract declarator: no parameter name
    self.unique_counter += 1;
    format!("__anon_param.{}", self.unique_counter)
}
```

## Step 15.7: typeof / __typeof__

Already fully implemented. The parser handles `typeof(expr)` and `typeof(type)`:

```c
int x = 42;
typeof(x) y = x;        // y is int
__typeof__(x) z = x;    // same thing (GCC spelling)
```

The lexer maps `typeof`, `__typeof`, and `__typeof__` all to `TokenKind::Typeof`.

## Step 15.8: _Static_assert

Already fully implemented. `_Static_assert(expr, "message")` is parsed at the top level and in function bodies:

```c
_Static_assert(sizeof(int) == 4, "int must be 4 bytes");
```

If the expression evaluates to zero at compile time, the compiler emits an error with the given message.
