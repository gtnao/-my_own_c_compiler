# Step 16.3–16.8: Preprocessor Extensions (Remaining Steps)

## Step 16.3: Complex #if Expressions
Already implemented in Step 16.2 via the full `CondEval` recursive descent evaluator. Supports all C preprocessor expression operators including arithmetic, comparison, logical, bitwise, ternary, and `defined()`.

## Step 16.4: #undef Directive
Already implemented. The preprocessor handles `#undef NAME` by removing the macro from the definitions map.

## Step 16.6: #pragma once and #pragma pack
- `#pragma` lines are parsed and silently ignored
- `#pragma once` behavior is effectively handled by the `included` set which tracks canonicalized file paths and prevents re-inclusion
- `#pragma pack` is not needed for our use case (struct layout follows standard ABI rules)

## Step 16.8: #include_next
`#include_next` is a GCC extension that searches for headers starting from the next directory in the search path (after the directory containing the current file).

Our implementation treats `#include_next` identically to `#include` — this is a simplification but works for most practical cases since our compiler-provided headers don't shadow system headers that need `#include_next` chaining.

```rust
if trimmed.starts_with("#include_next") || trimmed.starts_with("#include") {
    let directive_len = if trimmed.starts_with("#include_next") {
        "#include_next".len()
    } else {
        "#include".len()
    };
    // ... rest of include processing
}
```
