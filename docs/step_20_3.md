# Step 20.3: Forward-Declared Struct Resolution with Tagged Types

## Overview

This step fixes a critical performance and correctness issue with forward-declared struct resolution that caused the compiler to hang when processing PostgreSQL's complex headers (500+ struct definitions, 790+ typedefs).

## Problems

### 1. O(n²) Performance

Previously, `update_struct_members_with_struct` was called every time a forward-declared struct was defined. It scanned **all** struct_tags and **all** typedefs to find and replace forward references:

```
For each struct definition (N ≈ 500):
    Scan all struct_tags (≈ 500) + all typedefs (≈ 790)
Total: 500 × (500 + 790) = 645,000 operations
```

Each operation involved cloning and traversing type trees, making the compiler hang on PostgreSQL's `executor/spi.h`.

### 2. Correctness Bug: Untagged Empty Struct Replacement

`TypeKind::Struct(Vec<StructMember>)` had no tag name. All empty (forward-declared) structs looked identical:

```rust
// Both forward declarations looked the same:
Struct([])  // Could be struct A or struct B!
```

When struct A was defined, `replace_empty_struct_shallow` would replace **any** empty struct (including struct B's forward declaration) with A's full definition — silently producing wrong types.

## Solution

### Tagged Struct Variant

Changed `TypeKind::Struct` to carry an optional tag name:

```rust
// Before:
TypeKind::Struct(Vec<StructMember>)

// After:
TypeKind::Struct(Option<String>, Vec<StructMember>)
```

This allows precise identification of which forward declaration an empty struct represents:

```rust
Struct(Some("Node"), vec![])     // Forward-declared Node
Struct(Some("List"), vec![])     // Forward-declared List — now distinguishable!
Struct(None, vec![...])          // Anonymous struct
```

### Targeted Resolution with `replace_tagged_empty_struct`

The replacement function now checks the tag name before replacing:

```rust
fn replace_tagged_empty_struct(ty: &Type, target_tag: &str, full_ty: &Type) -> Option<Type> {
    match &ty.kind {
        TypeKind::Struct(Some(tag), members) if members.is_empty() && tag == target_tag => {
            Some(full_ty.clone())  // Only replace matching tag
        }
        TypeKind::Ptr(base) => { /* recurse */ }
        TypeKind::Array(base, size) => { /* recurse */ }
        _ => None,
    }
}
```

### Lazy Resolution with `resolve_forward_refs`

Instead of eagerly updating all types on each struct definition, forward references are resolved in a single batch pass at the end of parsing:

1. During parsing, `resolved_forward_tags` tracks which tags were forward-declared then defined
2. After all declarations are parsed, `resolve_forward_refs()` processes only those specific tags
3. For each resolved tag, only struct_tags and typedefs that actually reference it are updated

This reduces the work from O(n²) to O(k × n) where k is the number of forward-declared tags (typically much smaller than n).

### Codegen Resolution via `struct_defs`

For self-referential structs like `struct Node { int val; Node *next; }`, the `next` member's type is `Ptr(Struct("Node", []))` — it cannot contain the full definition without creating infinite nesting.

Solution: Pass struct definitions from parser to codegen via `Program.struct_defs`:

```rust
// In codegen, when accessing struct members:
fn resolve_struct_type(&self, ty: &Type) -> Type {
    if let TypeKind::Struct(Some(tag), members) = &ty.kind {
        if members.is_empty() {
            if let Some(full_ty) = self.struct_defs.get(tag) {
                return full_ty.clone();
            }
        }
    }
    ty.clone()
}
```

This is called at member access points (`.`, `->`) and `expr_type()` to resolve forward references on demand.

## Files Changed

- `src/types.rs` — `TypeKind::Struct(Option<String>, Vec<StructMember>)`: added tag name
- `src/ast.rs` — `Program.struct_defs`: pass struct definitions to codegen
- `src/parser.rs` — Tagged struct creation, `resolve_forward_refs()`, removed O(n²) update
- `src/codegen.rs` — `resolve_struct_type()` for lazy member resolution

## Verification

- All 578 existing tests pass
- PostgreSQL `executor/spi.h` (29,051 lines preprocessed) compiles in seconds (previously hung indefinitely)
- Self-referential structs (`struct Node { Node *next; }`) work correctly
