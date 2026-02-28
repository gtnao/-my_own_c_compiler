# Step 14.2: Float and Double Types

## Overview

Add `float` (32-bit IEEE 754) and `double` (64-bit IEEE 754) floating-point type support, including:

- Type declarations (`float f`, `double d`)
- Float literals (`3.14`, `1.5f`)
- Arithmetic operations (`+`, `-`, `*`, `/`)
- Comparison operators (`==`, `!=`, `<`, `<=`, `>`, `>=`)
- Type conversions (int↔float, int↔double, float↔double)
- Cast expressions (`(int)3.14`, `(double)5`)
- `sizeof(float)` = 4, `sizeof(double)` = 8

## x86-64 Floating-Point Architecture

### XMM Registers

x86-64 provides 16 128-bit SSE registers (`%xmm0`–`%xmm15`) for floating-point operations. Unlike the general-purpose registers (`%rax`, etc.), XMM registers use dedicated SSE instructions:

| Register | Purpose |
|---|---|
| `%xmm0` | Primary accumulator for float/double operations |
| `%xmm1` | Secondary operand in binary operations |
| `%xmm0`–`%xmm7` | Function argument passing (System V ABI) |
| `%xmm0` | Function return value |

### SSE Instruction Suffixes

- `ss` — Scalar Single-precision (float, 32-bit)
- `sd` — Scalar Double-precision (double, 64-bit)

### Key Instructions

| Operation | Float | Double |
|---|---|---|
| Load from memory | `movss (%rax), %xmm0` | `movsd (%rax), %xmm0` |
| Store to memory | `movss %xmm0, (%rdi)` | `movsd %xmm0, (%rdi)` |
| Add | `addss %xmm1, %xmm0` | `addsd %xmm1, %xmm0` |
| Subtract | `subss %xmm1, %xmm0` | `subsd %xmm1, %xmm0` |
| Multiply | `mulss %xmm1, %xmm0` | `mulsd %xmm1, %xmm0` |
| Divide | `divss %xmm1, %xmm0` | `divsd %xmm1, %xmm0` |
| Compare | `ucomiss %xmm1, %xmm0` | `ucomisd %xmm1, %xmm0` |

### Type Conversion Instructions

| Conversion | Instruction |
|---|---|
| int → float | `cvtsi2ss %rax, %xmm0` |
| int → double | `cvtsi2sd %rax, %xmm0` |
| float → int (truncate) | `cvttss2si %xmm0, %rax` |
| double → int (truncate) | `cvttsd2si %xmm0, %rax` |
| float → double | `cvtss2sd %xmm0, %xmm0` |
| double → float | `cvtsd2ss %xmm0, %xmm0` |

Note: `cvttss2si` and `cvttsd2si` use **truncation** (toward zero), matching C's cast semantics. The non-truncating variants (`cvtss2si`, `cvtsd2si`) use the current rounding mode (default: round-to-nearest), which is NOT what C's `(int)` cast does.

## Implementation

### Dual-Register Convention

The compiler maintains two accumulator conventions:
- **Integer expressions** → result in `%rax`
- **Float/double expressions** → result in `%xmm0`

The `expr_type()` method determines which register convention an expression uses. At boundaries (assignment, cast, function call), conversion instructions bridge between the two.

### Float Literal Loading

Float literals are stored as `f64` in the AST (C default: bare literals are `double`). Loading uses the integer register as an intermediary:

```asm
  movabs $4614253070214989087, %rax   # f64 bit pattern of 3.14
  movq %rax, %xmm0                   # move to XMM register
```

### Stack-Based Float Operations

Float/double values use the same stack-machine approach as integers, but with different push/pop:

```rust
fn push_float(&mut self) {
    self.emit("  sub $8, %rsp");
    self.emit("  movsd %xmm0, (%rsp)");
    self.stack_depth += 1;
}

fn pop_float(&mut self, reg: &str) {
    self.emit(&format!("  movsd (%rsp), {}", reg));
    self.emit("  add $8, %rsp");
    self.stack_depth -= 1;
}
```

### Binary Operation Type Promotion

When one operand is `double` and the other is `float` or `int`, the operation is performed in double precision:

```
int + double → cvtsi2sd → addsd (result: double)
float + double → cvtss2sd → addsd (result: double)
float + float → addss (result: float)
```

### Float Comparisons with `ucomiss`/`ucomisd`

The `ucomiss` and `ucomisd` instructions set CPU flags differently from integer `cmp`:

| Condition | Flag State | Set Instruction |
|---|---|---|
| xmm0 > xmm1 | CF=0, ZF=0 | `seta` |
| xmm0 >= xmm1 | CF=0 | `setae` |
| xmm0 < xmm1 | CF=1 | `setb` |
| xmm0 <= xmm1 | CF=1 or ZF=1 | `setbe` |
| xmm0 == xmm1 | ZF=1, PF=0 | `sete` + `setnp` |
| Unordered (NaN) | PF=1 | — |

For equality, both `ZF` and `PF` must be checked because NaN comparisons set `PF=1`.

### Lexer Changes

The number lexer was extended to detect floating-point literals:
- Decimal point: `3.14`, `.5`, `3.`
- Exponent: `1e10`, `1.5e-3`
- Float suffix: `3.14f`, `1.0F`
- Integer suffixes (`L`, `l`, `U`, `u`) are also now consumed and ignored.

## Test Cases

```c
double a = 3.14; return (int)a;           // → 3 (truncation)
double a = 2.5; double b = 1.5; return (int)(a + b);  // → 4
float a = 1.5; float b = 1.5; return (int)(a + b);    // → 3
double a = 10.7; return (int)a;           // → 10 (truncation, not rounding)
sizeof(float) == 4; sizeof(double) == 8;
double a = 1.5; double b = 1.5; return a == b;  // → 1
double a = 1.0; double b = 2.0; return a < b;   // → 1
```
