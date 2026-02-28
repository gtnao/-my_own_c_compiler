# Step 13.4: Improved Error Messages

## Overview

Enhance error messages with GCC-style colored output and add a warning facility.

## Error Format

Errors follow the GCC format with ANSI color codes:

```
file.c:3:14: error: expected semicolon
    int a = 5
             ^
```

- **`error:`** is displayed in bold red (`\x1b[1;31m`)
- **`^`** caret marker is displayed in bold green (`\x1b[1;32m`)
- Location is `file:line:column` (1-based)

## Warning Support

Added `warn_at()` method to `ErrorReporter`:

```rust
pub fn warn_at(&self, pos: usize, msg: &str) {
    let (line_num, col, line_str) = self.get_location(pos);
    eprintln!("{}:{}:{}: \x1b[1;35mwarning:\x1b[0m {}", ...);
    eprintln!("{}", line_str);
    eprintln!("{}\x1b[1;32m^\x1b[0m", " ".repeat(col));
}
```

- **`warning:`** is displayed in bold magenta (`\x1b[1;35m`)
- Unlike `error_at()`, `warn_at()` does not terminate the program

## Error vs Warning

| Method | Color | Exits? |
|---|---|---|
| `error_at()` | Red | Yes (`exit(1)`) |
| `warn_at()` | Magenta | No (continues) |

## Location Calculation

The `get_location()` method converts a byte offset into `(line, column, line_text)`:

1. Count newlines before `pos` to determine line number
2. Track the last newline position to compute column
3. Extract the full line text for display

This provides precise error locations for any position in the source file, including preprocessed multi-file inputs.
