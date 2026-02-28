# Step 20.1: PostgreSQL Build System Integration

## Overview

This step adds the foundational support needed to compile PostgreSQL header files. Multiple compiler subsystems were enhanced to handle the complexity of real-world system headers and PostgreSQL's build infrastructure.

## Changes

### 1. CLI Flag Support: `-I` and `-D`

**`-I` (Include Path)**
```bash
./my_own_c_compiler -I/usr/include/postgresql/14/server source.c
```

Supports two forms:
- `-I<dir>` (no space): `-I/usr/include`
- `-I <dir>` (with space): `-I /usr/include`

Include paths are searched after compiler built-in headers but before system headers (`/usr/include`). This matches GCC's behavior.

**`-D` (Macro Definition)**
```bash
./my_own_c_compiler -DVAL=42 -DFLAG source.c
```

Supports:
- `-D<name>=<value>`: defines macro with specific value
- `-D<name>`: defines macro as `1`
- `-D <name>=<value>`: space-separated form

### 2. Preprocessor: Comment Stripping

Added `strip_comments()` function that removes C-style comments (`/* ... */` and `// ...`) before directive processing. This is critical because:

- System headers like glibc's `features.h` and `sys/cdefs.h` contain complex multi-line comments
- Without stripping, `#` characters inside comments could be mistaken for preprocessor directives
- Block comments preserve newlines to maintain correct line numbering

### 3. Preprocessor: Directive Normalization

C standard allows whitespace between `#` and directive name:
```c
#  define FOO 1      /* valid C */
# ifdef BAR          /* valid C */
```

The preprocessor now strips `#` and leading whitespace to normalize directives before pattern matching, instead of relying on `trimmed.starts_with("#define")`.

### 4. Preprocessor: Correct `#if`/`#elif`/`#else`/`#endif` Chain Tracking

**Bug fixed**: The conditional compilation stack previously only tracked `(active: bool)`. This caused incorrect behavior with `#ifdef ... #elif ... #else` chains:

```c
#ifdef HAVE_LONG_INT_64      // true
typedef long int int64;
#elif defined(HAVE_LONG_LONG) // should be skipped
typedef long long int int64;
#else                          // should be skipped
#error must have 64-bit type  // BUG: was reached!
#endif
```

The fix changes the stack to `Vec<(bool, bool)>` — `(active, any_branch_taken)`:
- `any_branch_taken` tracks whether ANY branch in the current `#if`/`#elif`/`#else` chain was already taken
- `#elif`: if `any_branch_taken`, unconditionally set active=false
- `#else`: if `any_branch_taken`, set active=false; otherwise set active=true

### 5. Lexer: Integer Overflow Safety

Large hex literals like `0xFFFFFFFFFFFFFFFF` caused arithmetic overflow panics in the lexer. Fixed by:
- Using `u64` with `wrapping_mul`/`wrapping_add` for hex, binary, and octal parsing
- Casting to `i64` after computation
- Separating numeric suffix stripping from the parsed string

### 6. Parser: `__int128` Type Support

PostgreSQL uses GCC's 128-bit integer type:
```c
typedef __int128 int128;
typedef unsigned __int128 uint128;
```

Added `__int128`, `__int128_t`, and `__uint128_t` as recognized type identifiers, mapped to `long` (64-bit) internally. Full 128-bit arithmetic is not implemented, but type declarations compile correctly.

### 7. Parser: `__attribute__` After Declarations

Added `skip_attribute()` calls in:
- `extern` declarations: `extern void func(...) __attribute__((noreturn));`
- `typedef` declarations: `typedef __int128 int128 __attribute__((aligned(8)));`

### 8. Parser: Constant Expressions in Array Dimensions

Array dimensions previously only accepted numeric literals. Now they accept full constant expressions:
```c
char padding[128 - sizeof(unsigned short) - sizeof(unsigned long)];
```

Changed all array size parsing locations to use `eval_const_expr()` instead of expecting `TokenKind::Num`.

### 9. System Include Path

Added `/usr/include/x86_64-linux-gnu` to system header search paths for architecture-specific headers on Debian/Ubuntu.

## Test Results

- All 578 existing tests pass
- 5 new tests for `-I` and `-D` flags
- PostgreSQL's `postgres.h` header (which pulls in `c.h`, `pg_config.h`, system headers, etc.) compiles successfully

## Verification

```bash
# Compile a file that includes postgres.h
echo '#include "postgres.h"
int main() { return 0; }' > /tmp/test_pg.c
./target/debug/my_own_c_compiler -I/usr/include/postgresql/14/server /tmp/test_pg.c > /tmp/test_pg.s
# Assembly output generated successfully (7000+ lines)
```
