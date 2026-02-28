# Step 14.1: Long Long and Compound Type Specifiers

## Overview

Add support for compound type specifiers commonly used in C code:
- `long long` / `long long int` — 8-byte signed integer (same as `long` on x86-64)
- `unsigned long long` — 8-byte unsigned integer
- `long int` — explicit form of `long`
- `short int` — explicit form of `short`
- `unsigned short int` — explicit form of `unsigned short`

These compound specifiers are extremely common in real-world C code, especially in system headers and libraries like PostgreSQL.

## Why This Matters

In C, type specifiers can be combined in various ways:

```c
long long x;          // 8-byte signed integer
long long int x;      // equivalent to long long
unsigned long long x; // 8-byte unsigned integer
long int x;           // equivalent to long
short int x;          // equivalent to short
unsigned short int x; // equivalent to unsigned short
```

The C standard (C11 §6.7.2) defines these as valid type specifier combinations. Many codebases, especially PostgreSQL, use `long long` extensively for 64-bit integer types.

## Implementation

### Parser Changes (`parse_type()`)

The `Long` token handler was extended to check for a following `Long` or `Int` token:

```rust
TokenKind::Long => {
    self.advance();
    // Skip optional "long" (long long) or "int" (long int)
    if self.current().kind == TokenKind::Long {
        self.advance();
        // Skip optional "int" after "long long"
        if self.current().kind == TokenKind::Int {
            self.advance();
        }
    } else if self.current().kind == TokenKind::Int {
        self.advance();
    }
    if is_unsigned { Type::ulong() } else { Type::long_type() }
}
```

Similarly, the `Short` token handler now skips an optional `Int`:

```rust
TokenKind::Short => {
    self.advance();
    // Skip optional "int" after "short"
    if self.current().kind == TokenKind::Int {
        self.advance();
    }
    if is_unsigned { Type::ushort() } else { Type::short_type() }
}
```

### Type Mapping on x86-64

On x86-64 Linux (LP64 model):

| C Type | Size | Alignment | Internal Type |
|--------|------|-----------|---------------|
| `short` / `short int` | 2 | 2 | `Short` |
| `int` | 4 | 4 | `Int` |
| `long` / `long int` | 8 | 8 | `Long` |
| `long long` / `long long int` | 8 | 8 | `Long` |

Note that on x86-64 LP64, `long` and `long long` are both 8 bytes. They are distinct types in the C standard, but for our compiler they map to the same internal `Long` type. This is the same approach GCC and Clang take on this platform.

### The `unsigned` Prefix

The `unsigned` keyword is parsed first as a flag (`is_unsigned`), then the base type specifier determines whether to produce `Type::ulong()` or `Type::long_type()`, etc. This works seamlessly with the compound specifiers:

```
unsigned long long int x;
^^^^^^^^ ^^^^ ^^^^ ^^^
   |       |    |    |
   |       |    |    +-- consumed as trailing "int"
   |       |    +------- consumed as second "long"
   |       +------------ consumed as first "long"
   +-------------------- sets is_unsigned = true
```

## Test Cases

```c
// long long
long long a = 42;              // basic usage
long long int b = 1;           // with trailing int
sizeof(long long) == 8         // size check
sizeof(long long int) == 8     // size check with int
unsigned long long c = 42;     // unsigned variant
sizeof(unsigned long long) == 8

// long int
long int d = 42;               // explicit long int
sizeof(long int) == 8

// short int
short int e = 42;              // explicit short int
sizeof(short int) == 2
unsigned short int f = 100;    // unsigned short int
```
