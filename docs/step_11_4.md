# Step 11.4: Callback Pattern

## Overview

This step extends function pointer support to work as function parameters, enabling the callback pattern. Callbacks are fundamental to C programming — they power `qsort`, signal handlers, event-driven systems, and any form of higher-order programming.

## Callback Pattern

A callback is a function passed as an argument to another function, which then calls it:

```c
int apply(int (*f)(int), int x) {
    return f(x);  // call the passed function
}

int double(int x) { return x * 2; }

int main() {
    return apply(double, 5);  // => 10
}
```

The key insight: `f` is just a local variable of type `Ptr(Void)` (8-byte pointer). When called as `f(x)`, it generates an indirect call `call *%r10`.

## Parser Changes

### Function Pointer Parameters

The parameter parsing in `function_or_prototype()` was extended to recognize function pointer syntax in parameter positions:

```
parameter = type ident                     // normal parameter
          | type "(" "*" ident ")" "(" param_types ")"  // function pointer parameter
          | type ident "[" "]"             // array parameter
```

After `parse_type()` returns the base type, the parser checks for `(` `*`:

```rust
if current == '(' && next == '*' {
    // Parse function pointer parameter
    // Consume: ( * name ) ( param_types )
    // Type becomes Ptr(Void)
}
```

The parameter type list `(int, int)` is parsed but the types are not stored — the function pointer is simply typed as `Ptr(Void)`.

### Calling Convention

When `f(x)` is called inside a function where `f` is a parameter:
1. The parser sees `f` is a declared variable → generates `FuncPtrCall`
2. Code generation loads `f` from its stack slot
3. Saves the pointer to `%r10`
4. Sets up arguments in registers
5. Calls `call *%r10`

## Example: map_sum with Callback

```c
int map_sum(int *a, int n, int (*f)(int)) {
    int s = 0;
    int i;
    for (i = 0; i < n; i++)
        s += f(a[i]);  // apply callback to each element
    return s;
}

int dbl(int x) { return x * 2; }

int main() {
    int a[3] = {1, 2, 3};
    return map_sum(a, 3, dbl);  // => 2+4+6 = 12
}
```

### Generated Assembly for `f(a[i])`:

```asm
  # Load f (function pointer from parameter)
  mov -24(%rbp), %rax     # f is 3rd parameter
  mov %rax, %r10          # save to %r10

  # Evaluate a[i] (argument for callback)
  # ... array indexing code ...
  # Result in %rax

  push %rax
  pop %rdi                # arg goes to %rdi
  mov $0, %al
  call *%r10              # indirect call through f
```

## Call Flow

```
main()
  │
  ├─ Evaluates &dbl → lea dbl(%rip), %rax
  ├─ Passes as 3rd argument in %rdx
  │
  └─ call map_sum
       │
       ├─ map_sum receives f in %rdx → stores to stack
       │
       └─ Loop body:
            ├─ Loads f from stack → %r10
            ├─ Evaluates a[i] → %rdi
            └─ call *%r10  ─────────→  dbl(a[i])
                                        │
                                        └─ returns x*2
```

## Test Cases

```c
// Basic callback
int apply(int (*f)(int), int x) { return f(x); }
int dbl(int x) { return x * 2; }
int main() { return apply(dbl, 5); }  // => 10

// Callback with different function
int apply(int (*f)(int), int x) { return f(x); }
int sq(int x) { return x * x; }
int main() { return apply(sq, 5); }   // => 25

// Array processing with callback (map + reduce)
int map_sum(int *a, int n, int (*f)(int)) {
    int s = 0; int i;
    for (i = 0; i < n; i++) s += f(a[i]);
    return s;
}
int dbl(int x) { return x * 2; }
int main() {
    int a[3] = {1, 2, 3};
    return map_sum(a, 3, dbl);  // => 12
}
```

## Phase 11 Complete

With this step, Phase 11 (Standard Library Compatibility) is complete:

| Step | Feature | Description |
|------|---------|-------------|
| 11.1 | printf | External function calls via libc linking |
| 11.2 | Variadic args | `va_list`, `va_start`, `va_arg` with register save area |
| 11.3 | Function pointers | Declaration, function-to-pointer decay, `call *%r10` |
| 11.4 | Callbacks | Function pointers as parameters, enabling higher-order patterns |
