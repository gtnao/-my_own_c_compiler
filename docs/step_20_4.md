# Step 20.4: PIC Code Generation and Shared Library Support

## Overview

This step adds Position Independent Code (PIC) generation support, enabling compilation of shared libraries (`.so` files) — required for PostgreSQL extensions.

## Changes

### Command Line Flags

- `-fPIC` / `-fpic`: Enable PIC code generation
- `-shared`: Link as shared library (passes `-shared` to gcc linker)

### PIC Code Generation

In PIC mode, the following changes are applied:

#### 1. Extern Global Variable Access via GOT

Non-PIC:
```asm
mov CurrentMemoryContext(%rip), %rax    # Direct RIP-relative access
```

PIC:
```asm
mov CurrentMemoryContext@GOTPCREL(%rip), %rax  # Load address from GOT
mov (%rax), %rax                                # Indirect load through GOT
```

The GOT (Global Offset Table) is filled by the dynamic linker at load time. Extern symbols cannot use direct RIP-relative addressing in shared objects because their address is not known at compile time.

#### 2. Function Calls via PLT

Non-PIC:
```asm
call printf       # Direct call
```

PIC:
```asm
call printf@PLT   # Call through Procedure Linkage Table
```

The PLT (Procedure Linkage Table) provides lazy binding for function calls in shared objects.

#### 3. Function Pointer Decay (Function Name as Value)

Non-PIC:
```asm
lea func_name(%rip), %rax   # Direct address
```

PIC:
```asm
mov func_name@GOTPCREL(%rip), %rax   # Address through GOT
```

### Static Local Variable Visibility

Static local variables (prefixed with `__static.`) now use `.local` directive instead of `.globl`:

```asm
# Before (incorrect for shared objects):
  .globl __static.Pg_magic_data.1
__static.Pg_magic_data.1:

# After (correct):
  .local __static.Pg_magic_data.1
__static.Pg_magic_data.1:
```

This prevents the linker from trying to create PLT/GOT entries for file-local symbols.

### Implementation in Codegen

The `Codegen` struct now has:
- `pic_mode: bool` — whether PIC code generation is enabled
- `extern_names: HashSet<String>` — set of extern symbol names

Key methods modified:
- `gen_addr()`: Uses `@GOTPCREL` for extern globals in PIC mode
- `emit_load_var()`: Loads extern globals through GOT in PIC mode
- `emit_store_var()`: Stores to extern globals through GOT in PIC mode
- `gen_expr()` for `FuncCall`: Uses `@PLT` suffix in PIC mode

## Verification

PostgreSQL extension compiled as shared library:
```bash
$ ./target/debug/my_own_c_compiler -fPIC -S -I/usr/include/postgresql/14/server \
    -o pg_ext.s pg_ext.c
$ gcc -shared -o pg_ext.so pg_ext.s
$ nm -D pg_ext.so | grep add_one
00000000000014f1 T add_one
```

The resulting `.so` file correctly exports `add_one`, `Pg_magic_func`, and `pg_finfo_add_one` symbols.
