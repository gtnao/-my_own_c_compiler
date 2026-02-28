# Step 12.7: Complex Type Declarations

## Overview

Support complex type declarations that combine pointers and arrays:

- **Array of pointers**: `int *arr[3]` — `arr` is an array of 3 `int*` elements
- **Pointer to array**: `int (*p)[3]` — `p` is a pointer to an `int[3]`

## Array of Pointers

`int *arr[3]` was already supported by the existing parser. The `parse_type()` function produces `int*`, and then the variable declaration parsing handles `arr[3]`, creating `Array(Ptr(Int), 3)`.

```c
int a = 1, b = 2, c = 3;
int *arr[3];
arr[0] = &a; arr[1] = &b; arr[2] = &c;
return *arr[0] + *arr[1] + *arr[2];  // => 6
```

Each element of `arr` is an 8-byte pointer on x86-64. `arr[i]` yields a pointer, and `*arr[i]` dereferences it.

## Pointer to Array

`int (*p)[3]` requires special parsing because the parentheses change the binding: without them, `int *p[3]` would be an array of pointers. The `(*p)` groups the pointer declarator.

### Parsing

The existing `parse_func_ptr_decl` was renamed to `parse_func_ptr_or_array_ptr_decl` and extended. After parsing `(*name)`, the next token determines the type:

- `(` → function pointer: `type (*name)(param_types)`
- `[` → pointer to array: `type (*name)[size]`

```rust
if self.current().kind == TokenKind::LBracket {
    // Array pointer: type (*name)[size]
    self.advance();
    let size = /* parse size */;
    self.expect(TokenKind::RBracket);
    let arr_ty = Type::array_of(base_ty, size);
    let ptr_ty = Type::ptr_to(arr_ty);
    // declare variable with ptr_ty
} else {
    // Function pointer: type (*name)(param_types)
    // ... existing logic
}
```

### Type Construction

For `int (*p)[3]`:
1. Base type: `int`
2. Array type: `Array(Int, 3)` — a 12-byte array of 3 ints
3. Pointer type: `Ptr(Array(Int, 3))` — an 8-byte pointer to that array

### Usage

```c
int a[3] = {10, 20, 30};
int (*p)[3] = &a;    // p points to the whole array
return (*p)[1];       // dereference p to get the array, then index → 20
```

`*p` produces the array (which decays to a pointer), then `[1]` indexes it.

## C Declaration Reading Rule

The "clockwise/spiral" rule for reading C declarations:

| Declaration | Reading | Type |
|---|---|---|
| `int *a[3]` | a is array[3] of pointer to int | `Array(Ptr(Int), 3)` |
| `int (*a)[3]` | a is pointer to array[3] of int | `Ptr(Array(Int, 3))` |
| `int (*f)(int)` | f is pointer to function(int) returning int | `Ptr(Void)` (simplified) |

## Test Cases

```c
// Pointer to array
int a[3] = {10, 20, 30};
int (*p)[3] = &a;
return (*p)[1];  // => 20

// Array of pointers
int a=1, b=2, c=3;
int *arr[3]; arr[0]=&a; arr[1]=&b; arr[2]=&c;
return *arr[0] + *arr[1] + *arr[2];  // => 6
```
