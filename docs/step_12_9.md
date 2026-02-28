# Step 12.9: Struct Bit-Fields

## Overview

Support bit-field declarations in structs, allowing multiple values to be packed within a single storage unit (e.g., a 32-bit `int`):

```c
struct {
    int a : 4;   // 4 bits (values 0-15)
    int b : 4;   // 4 bits, packed into same int
} s;
s.a = 5;
s.b = 3;
// Both fields share a single 4-byte int
```

## Implementation

### 1. StructMember Extension (types.rs)

Added `bit_width` and `bit_offset` fields to `StructMember`:

```rust
pub struct StructMember {
    pub name: String,
    pub ty: Type,
    pub offset: usize,
    pub bit_width: usize,   // 0 = normal member
    pub bit_offset: usize,  // bit offset within storage unit
}
```

### 2. Parsing (parser.rs)

After parsing the member name, check for `: width`:

```c
struct { int a : 4; int b : 4; }
```

The parser tracks the current bit offset within the storage unit. When a bit-field doesn't fit in the current unit, it moves to the next aligned storage unit.

**Layout algorithm:**
1. For the first bit-field, align `offset` to the type's alignment
2. Check if `bit_offset + bit_width > storage_bits`
3. If it fits, pack into current unit at `bit_offset`
4. If not, advance `offset` to next storage unit, reset `bit_offset = 0`
5. When a normal (non-bit-field) member follows, finish the current storage unit

### 3. Bit-Field Read (codegen.rs)

Reading a bit-field value:
1. Compute the address of the storage unit (`gen_addr`)
2. Load the full storage unit (`emit_load_indirect`)
3. Right-shift by `bit_offset` to move the field to bit 0
4. AND with a mask of `(1 << bit_width) - 1` to extract only the field bits

```asm
; Read s.b where bit_width=4, bit_offset=4
  lea -4(%rbp), %rax      ; address of storage unit
  movslq (%rax), %rax      ; load full 32-bit int
  shr $4, %rax             ; shift field to bit 0
  and $15, %rax            ; mask to 4 bits (0xF)
```

### 4. Bit-Field Write (codegen.rs)

Writing a bit-field value requires read-modify-write:
1. Evaluate the new value (RHS)
2. Mask it to `bit_width` bits
3. Shift it to the correct `bit_offset`
4. Load the current storage unit value
5. Clear the old field bits (AND with inverted mask)
6. Set the new field bits (OR with shifted value)
7. Store the result back

```asm
; Write s.b = 3 where bit_width=4, bit_offset=4
  mov $3, %rax             ; new value
  and $15, %rax            ; mask to 4 bits
  shl $4, %rax             ; shift to position
  push %rax                ; save shifted value
  lea -4(%rbp), %rax       ; storage unit address
  mov %rax, %rdi           ; save address for store
  movslq (%rax), %rax      ; load current value
  mov $-241, %rcx          ; clear mask: ~(0xF << 4) = ~0xF0
  and %rcx, %rax           ; clear old bits
  pop %rcx                 ; get shifted new value
  or %rcx, %rax            ; set new bits
  movl %eax, (%rdi)        ; store back
```

## Storage Layout Example

```c
struct {
    int x : 3;   // bits [0:2], offset=0, bit_offset=0
    int y : 5;   // bits [3:7], offset=0, bit_offset=3
} s;
```

Both `x` and `y` share a single 4-byte `int` at offset 0:
```
bit:  7 6 5 4 3 2 1 0
      [  y      ][x  ]
```

## Limitations

- Only unsigned extraction is implemented (no sign extension for signed bit-fields)
- Zero-width bit-fields (`int : 0`) for forcing alignment are not supported
- Anonymous bit-fields are not supported
- Bit-fields wider than the storage unit are not validated

## Test Cases

```c
struct { int a : 4; int b : 4; } s;
s.a = 5; s.b = 3; return s.a;  // => 5
s.a = 5; s.b = 3; return s.b;  // => 3

struct { int x : 3; int y : 5; } s;
s.x = 7; s.y = 31; return s.x;  // => 7
s.x = 7; s.y = 31; return s.y;  // => 31
```
