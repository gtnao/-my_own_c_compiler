# Step 13.2: Peephole Optimization — Redundant Push/Pop Elimination

## Overview

Add a post-generation peephole optimization pass that eliminates redundant `push`/`pop` instruction pairs in the generated assembly.

## Stack-Machine Code Generation

The compiler uses a stack-machine approach where binary operations work as:

```asm
; Compute a + b
  <compute rhs>       ; result in %rax
  push %rax            ; save rhs on stack
  <compute lhs>       ; result in %rax
  pop %rdi             ; restore rhs to %rdi
  add %rdi, %rax       ; %rax = lhs + rhs
```

This generates many `push`/`pop` pairs. When they are adjacent, they can be optimized.

## Optimization Patterns

### Pattern 1: `push %rax` + `pop %rax` → remove both

When the same register is pushed and immediately popped, both instructions are dead code:

```asm
; Before:
  push %rax
  pop %rax
; After:
  (removed)
```

### Pattern 2: `push %rax` + `pop %reg` → `mov %rax, %reg`

A push followed by a pop into a different register is equivalent to a register-to-register move, which is faster (no memory access):

```asm
; Before:
  push %rax
  pop %rdi
; After:
  mov %rax, %rdi
```

## Implementation

The optimization is applied as a post-processing pass on the generated assembly text in `peephole_optimize()`:

```rust
fn peephole_optimize(&mut self) {
    let lines: Vec<&str> = self.output.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if i + 1 < lines.len() {
            let cur = lines[i].trim();
            let next = lines[i + 1].trim();

            if cur == "push %rax" && next == "pop %rax" {
                i += 2; continue;
            }
            if cur == "push %rax" {
                if let Some(reg) = next.strip_prefix("pop ") {
                    result.push(format!("  mov %rax, {}", reg));
                    i += 2; continue;
                }
            }
        }
        result.push(lines[i].to_string());
        i += 1;
    }
}
```

## Safety

The optimization only applies to **adjacent** `push`/`pop` pairs. Non-adjacent pairs (where other instructions intervene) are left as-is, since the stack state between the push and pop may be needed for intermediate computations.

This conservative approach is safe because:
1. It only transforms pairs that are provably equivalent
2. It doesn't change the stack depth at any point where it matters
3. The optimization runs after all code generation is complete

## Example

```c
return 2 + 3 * 4;
```

With constant folding, this becomes `mov $14, %rax` — no push/pop to optimize. For non-constant expressions, adjacent push/pop pairs are converted to `mov` instructions.

## Limitations

- Only optimizes adjacent `push %rax; pop %reg` pairs
- Does not optimize `push/pop` with intervening instructions
- Does not optimize `push %rdi; pop %rdi` or other register pairs
- No register allocation — still uses the stack-machine model
