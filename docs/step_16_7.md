# Step 16.7: Backslash Continuation Lines

## Overview

Add support for backslash-newline continuation lines in the preprocessor. This allows multi-line macro definitions, which are ubiquitous in PostgreSQL headers.

## The Problem

Multi-line macros like:
```c
#define ADD(a, b) \
    ((a) + (b))
```

Were not being joined. The preprocessor processed line-by-line, so `#define ADD(a, b) \` would define a macro with body `\`, and the next line `((a) + (b))` was treated as a separate C statement.

## Fix

Added `join_continuation_lines()` as a pre-processing pass before the main preprocessor loop. When a line ends with `\`, the backslash is stripped and the next line is appended without a newline separator:

```rust
fn join_continuation_lines(source: &str) -> String {
    let mut result = String::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        if line.ends_with('\\') {
            result.push_str(&line[..line.len() - 1]);
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}
```

This runs before any directive parsing, so `#define`, `#if`, and all other directives see the already-joined lines.

## Test Cases

```c
#define ADD(a, b) \
    ((a) + (b))
return ADD(10, 20);  // → 30
```
