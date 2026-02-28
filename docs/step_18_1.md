# Phase 18: GCC Extensions and Builtins

## Step 18.1: __attribute__ Semantic Support
Already implemented — `__attribute__((...))` is parsed and skipped at the syntax level. The parser handles it before types, after types, after function parameter lists, and on struct members.

## Step 18.2: Statement Expressions
Already implemented — `({ stmt1; stmt2; expr; })` evaluates all statements and the last expression becomes the value. Parsed in `primary()` when `(` is followed by `{`.

## Step 18.3: Extended __builtin Functions
Added support for additional GCC builtins:

- `__builtin_choose_expr(const_expr, expr1, expr2)` — compile-time conditional
- `__builtin_trap()` — maps to `abort()` function call
- `__builtin_classify_type(expr)` — returns 0 (simplified)
- `__builtin_huge_val()`, `__builtin_inf()`, `__builtin_nan()` — return 0 (simplified)
- `__builtin_clz/ctz/popcount/bswap/ffs/abs` — passed through as function calls to GCC builtins (linked via libgcc)

Previously implemented:
- `__builtin_expect(expr, val)` → returns expr
- `__builtin_constant_p(expr)` → returns 0
- `__builtin_unreachable()` → no-op
- `__builtin_offsetof(type, member)` → byte offset
- `__builtin_types_compatible_p(type1, type2)` → 1 or 0

## Step 18.4: Inline Assembly
`asm()`, `__asm()`, `__asm__()` with optional `volatile` qualifier are parsed and skipped:

```c
__asm__ volatile("" : : : "memory");  // memory barrier — skipped
asm("nop");                            // skipped
```

The implementation parses balanced parentheses and discards the content. This is sufficient for PostgreSQL where inline assembly is primarily used for memory barriers and spinlocks (which have C fallbacks).

## Step 18.5: Computed Goto
GCC extension for indirect jumps:

```c
void *p = &&target;  // &&label — address of label
goto *p;             // goto *expr — computed goto
```

### Implementation

**AST additions:**
- `Expr::LabelAddr(String)` — `&&label` expression
- `Stmt::GotoExpr(Expr)` — `goto *expr` statement

**Parser:**
- `&&` in unary position → parse label name, create `LabelAddr`
- `goto *` → parse expression, create `GotoExpr`

**Code generation:**
- `LabelAddr`: `lea .Lnn(%rip), %rax` — load label address
- `GotoExpr`: evaluate expression, `jmp *%rax` — indirect jump

Labels reuse the same label map as regular `goto`/`label:` for consistent naming.

## Step 18.6: __extension__ Keyword
Already implemented — `__extension__` is recognized as a keyword and skipped before types and expressions.

## Step 18.7: _Thread_local / __thread
Added as keywords that are recognized and skipped as storage class specifiers:

```c
_Thread_local int counter = 0;  // parsed, thread-local ignored
__thread int tls_var;            // same
```

The thread-local storage qualifier is parsed but treated as a regular variable. Full TLS support (fs-segment register, .tbss/.tdata sections) is not implemented as it requires linker cooperation.

## Step 18.8: __builtin_types_compatible_p
Already implemented — returns 1 if two types have the same kind and signedness, 0 otherwise.
