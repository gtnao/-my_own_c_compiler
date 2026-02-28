# Step 12.6: for Loop Scope

## Overview

Fix the scoping of variables declared in `for` loop init clauses. In C99+, a variable declared in a for loop's initialization is scoped to the loop body:

```c
int i = 100;
for (int i = 0; i < 5; i++) {
    // inner i is 0..4
}
return i;  // should be 100, not 5
```

## Problem

Before this fix, `for (int i = 0; ...)` declared `i` in the enclosing scope. When an outer variable `i` existed, the for-loop's `i` would shadow it permanently, and after the loop, the outer `i` would have the value from the loop.

## Fix

Wrap the for-loop's init clause in a new scope when it contains a variable declaration:

```rust
let has_decl_init = self.is_type_start(&self.current().kind.clone());
if has_decl_init {
    self.enter_scope();
}

// ... parse init, cond, inc, body ...

if has_decl_init {
    self.leave_scope();
}
```

This ensures that `int i` declared in `for(int i = ...)` gets a unique mangled name that doesn't conflict with the outer `i`. When the scope ends, the inner `i` is removed from the scope map, and subsequent references to `i` resolve to the outer variable.

## How Scoping Works

The compiler uses a scope stack with name mangling:
- `enter_scope()` pushes a new HashMap onto the scope stack
- `declare_var("i", ...)` creates a unique name like `i__2` and maps `"i"` → `"i__2"`
- `resolve_var("i")` searches scopes from innermost to outermost
- `leave_scope()` pops the innermost scope

Without the for-loop scope fix:
```
Scope: { i → i__0 }
for (int i = 0; ...) → declares i__1, but in the SAME scope!
After loop: i still resolves to i__1 (the loop variable)
```

With the fix:
```
Scope: { i → i__0 }
enter_scope() → new scope: { }
for (int i = 0; ...) → declares i__1 in inner scope: { i → i__1 }
leave_scope() → removes inner scope
After loop: i resolves to i__0 (the outer variable) ✓
```

## Test Cases

```c
int main() { int i = 100; for (int i = 0; i < 5; i++) {} return i; }  // => 100
int main() { int s = 0; for (int i = 0; i < 10; i++) s += i; return s; }  // => 45
```
