# Step 17.1–17.6: Standard Library Header Stubs

## Overview

Phase 17 implements a complete set of standard library header stubs that allow C programs using `#include <stdio.h>`, `#include <stdlib.h>`, etc. to compile with our compiler. These are "stubs" — they provide type definitions, function declarations, and macro constants, but the actual function implementations come from the system's libc at link time.

## Problem

When compiling real-world C code (especially PostgreSQL), the source files `#include` many standard headers. Without our own header stubs, the preprocessor would either:
1. Fail to find the header (if we don't search system paths)
2. Include the real system headers (which use GCC-specific extensions our compiler doesn't support)

The solution is to provide minimal, compatible header stubs that declare exactly what's needed.

## Preprocessor Header Search Path

The preprocessor was modified to search for headers in multiple locations:

```
Search order for #include <header>:
1. Binary-relative: ../../include, ../include, include (relative to compiler binary)
2. CARGO_MANIFEST_DIR/include (development mode)
3. ./include (current working directory)
4. Source file directory
5. /usr/include, /usr/local/include (system headers)
```

For `#include "header"`, the source file's directory is searched first (before compiler include paths).

This means our compiler's built-in headers take priority over system headers, allowing us to control exactly what definitions are available.

## Header Files Created

### stddef.h
Core type definitions used by nearly all other headers:
- `size_t` — `unsigned long` (8 bytes on x86-64)
- `ptrdiff_t` — `long` (pointer difference type)
- `wchar_t` — `int` (wide character type)
- `NULL` — `((void *)0)`
- `offsetof(type, member)` — maps to `__builtin_offsetof`

### stdint.h
Fixed-width integer types:
- `int8_t` through `int64_t` (signed)
- `uint8_t` through `uint64_t` (unsigned)
- `intptr_t`, `uintptr_t` — pointer-sized integers
- `intmax_t`, `uintmax_t` — maximum-width integers
- Min/max constants: `INT8_MIN`, `INT32_MAX`, `UINT64_MAX`, etc.

### stdbool.h
Boolean type support (C99):
- `bool` → `_Bool`
- `true` → `1`
- `false` → `0`

### stdio.h
Standard I/O declarations:
- `FILE` type (opaque struct pointer)
- `stdin`, `stdout`, `stderr` — standard streams
- `printf`, `fprintf`, `sprintf`, `snprintf` — formatted output
- `scanf`, `fscanf`, `sscanf` — formatted input
- `fopen`, `fclose`, `fread`, `fwrite` — file operations
- `fgetc`, `fgets`, `fputc`, `fputs` — character/string I/O
- `fseek`, `ftell`, `rewind` — file positioning
- Constants: `EOF`, `SEEK_SET`, `SEEK_CUR`, `SEEK_END`, `BUFSIZ`

### stdlib.h
General utilities:
- `malloc`, `calloc`, `realloc`, `free` — memory allocation
- `exit`, `abort`, `_exit`, `atexit` — program control
- `atoi`, `atol`, `strtol`, `strtoul` — string-to-number conversion
- `qsort`, `bsearch` — sorting and searching
- `rand`, `srand` — random numbers
- `getenv`, `setenv`, `system` — environment
- Constants: `EXIT_SUCCESS`, `EXIT_FAILURE`, `RAND_MAX`

### string.h
String and memory operations:
- `memcpy`, `memmove`, `memset`, `memcmp`, `memchr` — memory operations
- `strlen`, `strcpy`, `strncpy`, `strcmp`, `strncmp` — string operations
- `strcat`, `strncat`, `strchr`, `strrchr`, `strstr` — string manipulation
- `strdup`, `strndup` — string duplication
- `strtok`, `strtok_r` — tokenization
- `strcasecmp`, `strncasecmp` — case-insensitive comparison

### stdarg.h
Variable argument support:
- `va_list` → `__builtin_va_list`
- `va_start(ap, last)` → `__builtin_va_start(ap, last)`
- `va_arg(ap, type)` → `__builtin_va_arg(ap, type)`
- `va_end(ap)` → `__builtin_va_end(ap)`
- `va_copy(dest, src)` → `__builtin_va_copy(dest, src)`

These map to the compiler's built-in variadic argument handling.

### errno.h
Error number support:
- `errno` → `(*__errno_location())` (thread-safe errno on Linux)
- Error constants: `EPERM`, `ENOENT`, `EINTR`, `EINVAL`, `ENOMEM`, etc.

### limits.h
Implementation-defined limits:
- `CHAR_BIT` = 8
- `INT_MIN`, `INT_MAX`, `UINT_MAX` — int limits
- `LONG_MIN`, `LONG_MAX`, `ULONG_MAX` — long limits
- `LLONG_MIN`, `LLONG_MAX`, `ULLONG_MAX` — long long limits
- `PATH_MAX` = 4096, `NAME_MAX` = 255

### assert.h
Assertion macro:
- `assert(expr)` — calls `abort()` if expression is false
- Disabled when `NDEBUG` is defined

### ctype.h
Character classification:
- `isalnum`, `isalpha`, `isdigit`, `isxdigit` — character tests
- `islower`, `isupper`, `isspace`, `isprint` — more tests
- `tolower`, `toupper` — character conversion

### unistd.h (POSIX)
POSIX operating system API:
- `read`, `write`, `close`, `lseek` — basic I/O
- `dup`, `dup2`, `pipe` — file descriptor manipulation
- `fork`, `execv`, `execvp`, `execve` — process control
- `getpid`, `getppid`, `getuid`, `getgid` — process info
- `chdir`, `getcwd`, `access`, `unlink`, `rmdir` — filesystem
- `sleep`, `usleep` — time control
- `symlink`, `readlink` — symbolic links
- Constants: `STDIN_FILENO`, `STDOUT_FILENO`, `STDERR_FILENO`
- Access mode constants: `F_OK`, `R_OK`, `W_OK`, `X_OK`

### fcntl.h
File control:
- `open`, `creat`, `fcntl` — file operations
- Open flags: `O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_CREAT`, `O_TRUNC`, `O_APPEND`, etc.
- `fcntl` commands: `F_DUPFD`, `F_GETFD`, `F_SETFD`, `F_GETFL`, `F_SETFL`

### sys/types.h
POSIX type definitions:
- `pid_t`, `uid_t`, `gid_t` — process/user/group IDs
- `off_t`, `ssize_t` — file offset and signed size
- `mode_t`, `dev_t`, `ino_t`, `nlink_t` — filesystem types
- `blksize_t`, `blkcnt_t` — block types
- `time_t`, `suseconds_t` — time types

## Key Design Decisions

1. **Opaque FILE type**: `typedef struct _IO_FILE FILE;` — we declare FILE as a pointer to an opaque struct. The actual struct definition lives in glibc; we only need the pointer type.

2. **Thread-safe errno**: `#define errno (*__errno_location())` — this matches the Linux/glibc implementation where errno is a per-thread variable accessed through a function.

3. **Include guards**: All headers use `#ifndef _HEADER_H` / `#define _HEADER_H` guards to prevent multiple inclusion.

4. **Header dependencies**: Headers include each other as needed (e.g., `stdio.h` includes `stddef.h` for `size_t` and `stdarg.h` for `va_list`).

5. **Compiler built-in priority**: Our headers are searched before system headers, ensuring consistent behavior across different Linux distributions.

## Test Cases

```c
// stddef.h: size_t type
#include <stddef.h>
int main() { size_t s = 8; return s - 8; }  // => 0

// stdbool.h: bool type
#include <stdbool.h>
int main() { bool b = true; return b; }  // => 1

// stdint.h: fixed-width types
#include <stdint.h>
int main() { int32_t a = 42; uint64_t b = 100; return a + b - 142; }  // => 0

// stdio.h: EOF constant
#include <stdio.h>
int main() { return EOF + 1; }  // => 0

// stdlib.h: EXIT_SUCCESS/EXIT_FAILURE
#include <stdlib.h>
int main() { return EXIT_SUCCESS; }  // => 0

// string.h: strlen, strcmp (linked with libc)
#include <string.h>
int main() { return strlen("hello"); }  // => 5

// errno.h: error constants
#include <errno.h>
int main() { return EINVAL; }  // => 22

// limits.h: integer limits
#include <limits.h>
int main() { return CHAR_BIT; }  // => 8

// unistd.h: POSIX constants
#include <unistd.h>
int main() { return STDOUT_FILENO; }  // => 1

// fcntl.h: file control constants
#include <fcntl.h>
int main() { return O_RDONLY; }  // => 0

// sys/types.h: POSIX types
#include <sys/types.h>
int main() { pid_t p = 0; return p; }  // => 0

// Cross-header test: malloc + strcpy + strcmp + free
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main() {
    char *s = malloc(10);
    strcpy(s, "test");
    int r = strcmp(s, "test");
    free(s);
    return r;  // => 0
}
```

Total: 19 new tests added (537 → 556).
