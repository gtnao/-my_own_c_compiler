# Step 14.5: Hex, Octal, and Binary Integer Literals

## Overview

Add support for hexadecimal (`0xFF`), octal (`077`), and binary (`0b101`) integer literal syntax. Previously, only decimal integer literals were supported.

## Why This Matters

PostgreSQL and system headers use hex literals extensively for bit masks, flags, and memory constants:

```c
#define PG_DETOAST_DATUM(datum) \
    ((Datum) (VARATT_IS_1B(datum) ? (datum) : pg_detoast_datum(datum)))
#define TYPEALIGN(ALIGNVAL, LEN) \
    (((uintptr_t)(LEN) + ((ALIGNVAL)-1)) & ~((uintptr_t)((ALIGNVAL)-1)))
int flags = 0xFF;
int permissions = 0755;
```

## Implementation

### Lexer Changes

The `read_number_or_float()` method was refactored to detect prefix-based number formats:

1. **Hex (`0x`/`0X`)**: After consuming the `0x` prefix, read hex digits (`0-9`, `a-f`, `A-F`) and accumulate the value with base-16 arithmetic.

2. **Binary (`0b`/`0B`)**: After consuming the `0b` prefix, read binary digits (`0` or `1`) and accumulate with base-2 arithmetic.

3. **Octal (`0` followed by `0-7`)**: When a leading `0` is followed by an octal digit, read octal digits and accumulate with base-8 arithmetic. A bare `0` is just zero (decimal).

4. **Decimal/Float**: Handled by the extracted `read_decimal_float()` helper — same logic as before.

### Code Structure

The method was split into three parts for clarity:

- `read_number_or_float()` — Entry point. Detects prefix and dispatches.
- `read_decimal_float()` — Handles decimal integers and floating-point numbers (`.`, exponent, `f` suffix).
- `skip_int_suffix()` — Consumes trailing `L`/`l`/`U`/`u` suffixes (shared by all integer formats).

### Number Parsing Flow

```
read_number_or_float()
  ├── starts with '.' → read_decimal_float(starts_with_dot=true)
  ├── starts with '0x'/'0X' → parse hex digits → skip_int_suffix()
  ├── starts with '0b'/'0B' → parse binary digits → skip_int_suffix()
  ├── starts with '0' + octal digit → parse octal digits → skip_int_suffix()
  └── otherwise → read_decimal_float(starts_with_dot=false)
```

### Value Computation

For hex, the accumulation loop:
```rust
let mut val: i64 = 0;
while self.pos < self.input.len() && (self.input[self.pos] as char).is_ascii_hexdigit() {
    val = val * 16 + (self.input[self.pos] as char).to_digit(16).unwrap() as i64;
    self.pos += 1;
}
```

For octal:
```rust
let mut val: i64 = 0;
while self.pos < self.input.len() && self.input[self.pos] >= b'0' && self.input[self.pos] <= b'7' {
    val = val * 8 + (self.input[self.pos] - b'0') as i64;
    self.pos += 1;
}
```

For binary:
```rust
let mut val: i64 = 0;
while self.pos < self.input.len() && (self.input[self.pos] == b'0' || self.input[self.pos] == b'1') {
    val = val * 2 + (self.input[self.pos] - b'0') as i64;
    self.pos += 1;
}
```

All three formats produce a `TokenKind::Num(i64)` — the token representation is the same regardless of the source notation. Integer suffixes (`L`, `U`, `ULL`, etc.) are consumed and ignored after all integer formats.

## C Standard Notes

- Hex literals: C89/C90 and later. Prefix `0x` or `0X`.
- Octal literals: C89/C90 and later. Leading `0` prefix.
- Binary literals: Not in C standard (C23 proposal), but widely supported as a GCC/Clang extension.
- The literal `0` is technically octal but evaluates to zero either way.

## Test Cases

```c
int main() { return 0xFF; }       // → 255
int main() { return 0x0F; }       // → 15
int main() { return 0xAB; }       // → 171
int main() { return 0x0; }        // → 0
int main() { return 07; }         // → 7
int main() { return 077; }        // → 63
int main() { return 010; }        // → 8
int main() { return 00; }         // → 0
int main() { return 0b101; }      // → 5
int main() { return 0b1010; }     // → 10
int main() { int x = 0xFF; return x; }  // → 255
int main() { return 0x10; }       // → 16
int main() { return 0x0A; }       // → 10
int main() { return 0xf; }        // → 15 (lowercase)
```
