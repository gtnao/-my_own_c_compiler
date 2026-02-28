# Step 16.5: Predefined Macros

## Overview

Add a comprehensive set of predefined macros that are automatically defined before preprocessing begins. These are essential for PostgreSQL and standard C library headers that use `#ifdef __STDC__`, `#if __GNUC__`, etc.

## Predefined Macros Added

### Standard C
- `__STDC__` = 1
- `__STDC_VERSION__` = 201112L (C11)
- `__STDC_HOSTED__` = 1

### Platform / Architecture
- `__LP64__` = 1 (64-bit long and pointer)
- `__x86_64__`, `__x86_64`, `__amd64__`, `__amd64` = 1
- `__linux__`, `__linux`, `linux` = 1
- `__unix__`, `__unix`, `unix` = 1

### GCC Compatibility
- `__GNUC__` = 4, `__GNUC_MINOR__` = 0, `__GNUC_PATCHLEVEL__` = 0
- This makes the compiler appear as GCC 4.0 to feature-detection macros

### Type Sizes
- `__SIZEOF_SHORT__` = 2, `__SIZEOF_INT__` = 4
- `__SIZEOF_LONG__` = 8, `__SIZEOF_LONG_LONG__` = 8
- `__SIZEOF_POINTER__` = 8, `__SIZEOF_FLOAT__` = 4, `__SIZEOF_DOUBLE__` = 8
- `__CHAR_BIT__` = 8

### Type Names
- `__SIZE_TYPE__` = `unsigned long`
- `__PTRDIFF_TYPE__` = `long`
- `__INTMAX_TYPE__` = `long`
- `__WCHAR_TYPE__` = `int`

### Limits
- `__INT_MAX__` = 2147483647
- `__LONG_MAX__` = 9223372036854775807L
- `__SHRT_MAX__` = 32767
- `__SCHAR_MAX__` = 127

### Endianness
- `__BYTE_ORDER__` = 1234 (little-endian)
- `__ORDER_LITTLE_ENDIAN__` = 1234
- `__ORDER_BIG_ENDIAN__` = 4321

### Convenience
- `NULL` = `((void *)0)`

## Why These Matter

PostgreSQL headers use extensive conditional compilation:
```c
#ifdef __GNUC__
#define pg_attribute_noreturn() __attribute__((noreturn))
#endif

#if __STDC_VERSION__ >= 201112L
#define StaticAssertStmt(condition, errmessage) _Static_assert(condition, errmessage)
#endif
```
