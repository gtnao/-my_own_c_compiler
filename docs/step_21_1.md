# Step 21.1: GCC Preprocessor Compatibility — Parsing Fixes

## Overview

This step fixes multiple parsing issues discovered when compiling PostgreSQL backend source files preprocessed with `gcc -E`. The goal is to successfully parse all 331 PostgreSQL backend `.c` files through our compiler.

## Changes

### 1. Extern Function Definitions

GCC preprocessor output can produce `extern` function definitions (not just prototypes):

```c
extern tuplehash_hash *tuplehash_create(MemoryContext ctx, uint32 nelements,
    void *private_data)
{
    // function body
}
```

In C, `extern` on a function definition is legal (it's the default linkage), but our parser only handled extern prototypes (expecting `;` after parameter list).

**Fix**: After the existing extern prototype handler skips the parameter list with depth matching, check if the current token is `{` (function body). If so, restore the parser position and re-parse using `function_or_prototype()`.

Key insight: We preserve the robust brace-depth-based parameter skipping for prototypes (which handles complex glibc parameter declarations), and only fall back to full function parsing when `{` is detected after the parameters.

### 2. Non-Extern Comma-Separated Global Variables

PostgreSQL declares multiple global variables in a single declaration:

```c
sigset_t UnBlockSig, BlockSig, StartupBlockSig;
```

The extern comma-separated handler already existed, but non-extern global variables used the same pattern without handling.

**Fix**: Added comma-separated handling in `global_var()` after the initial variable name is parsed. Supports pointer stars and array dimensions for subsequent names.

### 3. GCC Statement Expressions `__extension__ ({...})`

GCC statement expressions appear in PostgreSQL headers:

```c
__extension__ ({ __typeof__(a) _a = (a); __typeof__(b) _b = (b); _a > _b ? _a : _b; })
```

Two issues were found:

#### Cast Detection False Positive

The cast detection in `unary()` checks for `(type)expr`. Since `__extension__` is in `is_type_start()`, the pattern `(__extension__ ({...}))` was misidentified as a cast expression.

**Fix**: Added exclusion in cast detection: if `tokens[pos+1]` is `Extension` followed by `LParen LBrace`, skip cast parsing.

#### Statement Context

`__extension__` followed by `({` in statement context was being treated as a variable declaration (because `__extension__` triggers `is_type_start`).

**Fix**: Added `Extension` case in `stmt()` that checks for the `__extension__ ({` pattern and parses it as an expression statement.

### 4. Static Local Variable Brace Initializer Double-Consume Bug

Static local variables with brace initializers caused "expected RBrace" errors:

```c
static const Oid funcargs[] = {23, 23, 2275, 2281, 23, 16};
```

**Root cause**: `parse_global_brace_init()` already consumes the closing `}`, but `static_local_var()` called `self.expect(TokenKind::RBrace)` again after it returned.

**Fix**: Removed the redundant `expect(RBrace)` call in `static_local_var()`.

## Implementation Details

### Extern Function Definition Detection Strategy

The approach uses a "try-skip, then decide" pattern:

```
extern handler:
  1. Save position (extern_start)
  2. Skip 'extern', qualifiers
  3. Parse type
  4. Get function name
  5. Skip parameter list (brace-depth matching)
  6. Skip attributes, __asm__
  7. Check current token:
     - If '{' → restore to extern_start, call function_or_prototype()
     - If ';' → register as extern prototype (existing behavior)
```

This is more robust than calling `is_function()` + `function_or_prototype()` directly, because `function_or_prototype()` requires fully parsing parameter declarations — which fails on complex glibc prototypes with `char[20]` parameters, `__restrict` qualifiers, etc.

## Results

- **Integration tests**: 578 PASS, 0 FAIL
- **PostgreSQL backend files**: 331/331 PASS (100%)

Previous: 318/331 (96.1%)

## Files Modified

- `src/parser.rs` — All four fixes described above
