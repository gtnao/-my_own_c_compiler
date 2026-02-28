# Step 16.1: Variadic Macros (__VA_ARGS__)

## Overview

Add support for variadic (variable-argument) macros using `...` and `__VA_ARGS__`. This is essential for PostgreSQL's `elog()`, `ereport()`, and many other logging/error-reporting macros.

## How It Works

### Macro Definition

```c
#define LOG(fmt, ...) printf(fmt, __VA_ARGS__)
```

The `...` in the parameter list indicates the macro accepts variable arguments. In the macro body, `__VA_ARGS__` expands to all the extra arguments passed beyond the named parameters.

### Implementation

**MacroDef** now tracks whether a macro is variadic:

```rust
enum MacroDef {
    Object(String),
    Function(Vec<String>, String, bool),  // params, body, is_variadic
}
```

**Parsing**: When parsing `#define`, if the last parameter is `...`, it's removed from the parameter list and `is_variadic` is set to `true`.

**Expansion**: When expanding a variadic macro call:
1. Named parameters are matched to their corresponding arguments as usual
2. Any extra arguments beyond the named parameters are joined with `, ` and substituted for `__VA_ARGS__` in the body

```rust
if is_variadic {
    let va_args = if args.len() > params.len() {
        args[params.len()..].join(", ")
    } else {
        String::new()
    };
    subst_params.push("__VA_ARGS__".to_string());
    subst_args.push(va_args);
}
```

## Examples

```c
// Basic variadic macro
#define LOG(fmt, ...) printf(fmt, __VA_ARGS__)
LOG("x=%d y=%d\n", 10, 20);
// Expands to: printf("x=%d y=%d\n", 10, 20);

// Macro that passes variadic args to another function
#define CALL(fn, ...) fn(__VA_ARGS__)
CALL(add, 20, 22);
// Expands to: add(20, 22);
```

## Test Cases

```c
#define FIRST(a, ...) a
return FIRST(3, 4, 5);  // → 3

#define CALL(fn, ...) fn(__VA_ARGS__)
return CALL(add, 20, 22);  // → 42
```
