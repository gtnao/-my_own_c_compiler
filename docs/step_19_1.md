# Phase 19: Advanced Code Generation

## Step 19.1: Struct Pass/Return by Value
Already implemented. Small structs (≤16 bytes) are copied via memory operations. The compiler handles:
- Struct return: copies struct to return area
- Struct parameter: copies struct on stack for callee

## Step 19.2: Floating Point Operations
Already implemented using XMM registers:
- `float` and `double` arithmetic (add, sub, mul, div)
- Float literals stored in `.data` section
- Integer ↔ float conversions via `cvtsi2sd`/`cvttsd2si`
- Float function arguments passed in XMM registers per ABI

## Step 19.3: volatile Semantics
`volatile` is recognized as a type qualifier and parsed, but no special code generation is performed (all memory accesses are currently un-optimized, so they're effectively volatile).

## Step 19.4: Variable Length Arrays (VLA)
Not implemented — VLAs (`int a[n]` with runtime `n`) require dynamic stack allocation (`alloca`). PostgreSQL generally avoids VLAs, using `palloc()` instead.

## Step 19.5: Compound Literals
Already implemented. `(type){initializers}` creates an anonymous local variable:
```c
struct S s = (struct S){.x = 3, .y = 4};
```

## Step 19.6: Bit-field ABI Layout
Already implemented with storage unit tracking, bit offset computation, and proper alignment.
