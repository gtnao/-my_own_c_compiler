# Step 16.2: Full #if Expression Evaluator with defined()

## Overview

Replace the simple `evaluate_simple_cond` function with a full recursive descent expression evaluator for preprocessor `#if` / `#elif` directives. This supports `defined()`, `&&`, `||`, `!`, comparison operators, arithmetic, bitwise operations, ternary operator, and parenthesized expressions.

## The Problem

The previous implementation only handled:
- Simple `defined(NAME)` at the start of the expression
- Single comparison operators (found via string search)

This failed for compound expressions like:
```c
#if defined(FOO) && !defined(BAR)
#if X + 5 == 10
#if (A > 0) || (B > 0)
```

## Solution: Recursive Descent Evaluator

Implemented `CondEval` struct with a full expression parser following C operator precedence:

```
expr        = ternary
ternary     = logical_or ("?" expr ":" ternary)?
logical_or  = logical_and ("||" logical_and)*
logical_and = bitwise_or ("&&" bitwise_or)*
bitwise_or  = bitwise_xor ("|" bitwise_xor)*
bitwise_xor = bitwise_and ("^" bitwise_and)*
bitwise_and = equality ("&" equality)*
equality    = relational (("==" | "!=") relational)*
relational  = shift (("<" | ">" | "<=" | ">=") shift)*
shift       = add (("<<" | ">>") add)*
add         = mul (("+" | "-") mul)*
mul         = unary (("*" | "/" | "%") unary)*
unary       = "!" unary | "~" unary | "-" unary | "+" unary | primary
primary     = number | "(" expr ")" | "defined" ident | char_literal | ident
```

### Key Features

- **`defined` operator**: Both `defined(NAME)` and `defined NAME` forms
- **Macro expansion**: Unknown identifiers that are macros get their values expanded recursively
- **Number parsing**: Decimal, hex (`0x...`), with suffix skipping (`U`, `L`, `UL`, `ULL`)
- **Character literals**: `'A'` evaluates to 65
- **Unknown identifiers**: Evaluate to 0 (standard C behavior)
- **Short-circuit semantics**: `&&` and `||` evaluate both sides (conservative)

## Test Cases

```c
#define FOO 1
#if defined(FOO) && !defined(BAR)
// Active — FOO is defined, BAR is not
#endif

#define X 5
#if X + 5 == 10
// Active — 5 + 5 == 10
#endif

#if 1 || 0
// Active — logical OR
#endif
```
