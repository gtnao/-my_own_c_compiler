# Step 20.2: PostgreSQL Extension Compilation

## Overview

This step enables compilation of PostgreSQL extension source files by adding support for struct initializers in static local variables.

## Changes

### Static Variable Struct Initializers

PostgreSQL's `PG_MODULE_MAGIC` macro expands to a function with a static const struct initializer:

```c
static const Pg_magic_struct Pg_magic_data = {
    sizeof(Pg_magic_struct),  // struct size
    14020 / 100,              // major version
    100,                      // minor version
    32,                       // float size
    64,                       // datum size
    1                         // something else
};
```

Previously, static variable initializers only supported a single numeric literal. Now they support:

1. **Brace-enclosed initializers** `{ val1, val2, ... }`:
   - Each value is evaluated as a constant expression (supports `sizeof`, arithmetic, etc.)
   - Values are packed into bytes according to the struct's field layout
   - Field sizes and offsets are read from the struct type's member definitions

2. **Constant expression initializers**:
   - Changed from `Num(n)` literal matching to `eval_const_expr()` evaluation
   - Supports `sizeof`, arithmetic, casts, and other compile-time expressions

### How Struct Initialization Works

For a struct type, the initializer:
1. Allocates a zero-filled byte array of `ty.size()` bytes
2. For each value in the initializer list:
   - Evaluates the constant expression
   - Determines the field size from the struct's member list (matching by offset)
   - Writes the value bytes at the current offset
   - Advances to the next field's offset (respecting alignment)
3. The byte array is emitted in `.data` section as `.byte` directives

## Verification

PostgreSQL extension source files now compile successfully:

```c
#include "postgres.h"
#include "fmgr.h"
#include "utils/builtins.h"

PG_MODULE_MAGIC;           // ← struct initializer in static variable
PG_FUNCTION_INFO_V1(add_one);

Datum add_one(PG_FUNCTION_ARGS) {
    int32 arg = PG_GETARG_INT32(0);
    PG_RETURN_INT32(arg + 1);
}
```

Generates correct assembly with static data section for the magic struct.
