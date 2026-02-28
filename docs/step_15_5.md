# Step 15.5: Anonymous Struct/Union Members (C11)

## Overview

C11 allows anonymous struct/union members inside a parent struct/union. The inner members are accessed directly through the parent:

```c
struct S {
    union { int a; long b; };  // anonymous union
    int c;
};

struct S s;
s.a = 42;  // access union member directly
s.c = 10;
```

## Implementation

When parsing struct/union members, if we encounter a struct/union type followed immediately by `;` (no member name), we treat it as an anonymous member and flatten its members into the parent:

```rust
if self.current().kind == TokenKind::Semicolon {
    // Anonymous struct/union member
    if let TypeKind::Struct(inner_members) = &mem_ty.kind {
        for inner in inner_members {
            // Calculate offset relative to parent
            let member_offset = if is_union {
                inner.offset
            } else {
                offset + inner.offset  // (after alignment)
            };
            members.push(StructMember {
                name: inner.name.clone(),
                ty: inner.ty.clone(),
                offset: member_offset,
                ...
            });
        }
        offset += mem_ty.size();  // advance past anonymous member
        self.expect(TokenKind::Semicolon);
        continue;
    }
}
```

## Layout Example

```
struct S {
    union { int a; long b; };  // offset 0, size 8 (union of int + long)
    int c;                      // offset 8
};
// Total size: 16 (8 for union + 4 for int + 4 padding)

// After flattening:
// member "a": offset 0, type int
// member "b": offset 0, type long (same offset — it's a union)
// member "c": offset 8, type int
```

## Test Cases

```c
// Anonymous union inside struct
struct { union { int a; long b; }; int c; } s;
s.a = 42; s.c = 10;
return s.a;  // => 42
return s.c;  // => 10

// Anonymous struct inside struct
struct { struct { int x; int y; }; int z; } s;
s.x = 99; s.z = 1;
return s.x;  // => 99
```
