# Step 7.1: 構造体定義とメンバアクセス

## 概要

Phase 7（構造体とユニオン）の最初のステップ。無名構造体（anonymous struct）の定義とメンバアクセス演算子 `.` を実装する。

```c
struct { int x; int y; } s;
s.x = 1;
s.y = 2;
return s.x + s.y;  // => 3
```

## 構造体のメモリレイアウト

構造体はメンバを順番にメモリ上に配置する。各メンバは自身のアラインメント要件に従って配置される。

### 例: `struct { int x; int y; }`

```
offset 0: x (int, 4 bytes)
offset 4: y (int, 4 bytes)
total size: 8 bytes
alignment: 4 (max of members)
```

### 例: `struct { char a; int b; }`

```
offset 0: a (char, 1 byte)
offset 1-3: padding (3 bytes, for int alignment)
offset 4: b (int, 4 bytes)
total size: 8 bytes
alignment: 4 (max of members)
```

charは1バイトアラインメントだが、次のintは4バイトアラインメントが必要。
そのため、charの後に3バイトのパディングが挿入される。

### アラインメント計算

メンバのオフセットは以下の式で計算する：

```
offset = (current_offset + align - 1) & !(align - 1)
```

これはビットマスクによるアラインメント切り上げ。例えば `align = 4` の場合：
- `!(4 - 1) = !0b11 = ...11111100`
- `current_offset = 1` → `(1 + 3) & !3 = 4 & !3 = 4`

構造体全体のサイズは、最後のメンバの `offset + size` を構造体のアラインメント（全メンバの最大アラインメント）で切り上げた値になる。

## 型システムの変更 (`types.rs`)

### StructMember 構造体

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct StructMember {
    pub name: String,
    pub ty: Type,
    pub offset: usize,  // pre-computed offset within struct
}
```

各メンバにはパーサーがオフセットを事前計算して格納する。コード生成時にはこのオフセットをそのまま使える。

### TypeKind::Struct

```rust
pub enum TypeKind {
    // ...
    Struct(Vec<StructMember>),
}
```

- `size()`: 最後のメンバの `offset + size` を構造体アラインメントで切り上げ
- `align()`: 全メンバの `ty.align()` の最大値

## パーサーの変更 (`parser.rs`)

### 構造体型のパース

`parse_type()` に `struct { ... }` のパースを追加：

```
struct_type = "struct" "{" (type name ";")* "}"
```

パース中にメンバのオフセットを計算する：

```rust
let align = mem_ty.align();
offset = (offset + align - 1) & !(align - 1);  // align up
members.push(StructMember { name, ty, offset });
offset += mem_ty.size();
```

### メンバアクセスのパース

`postfix()` に `.` 演算子を追加：

```rust
TokenKind::Dot => {
    self.advance();
    let name = /* member name */;
    self.advance();
    expr = Expr::Member(Box::new(expr), name);
}
```

## AST の変更 (`ast.rs`)

```rust
pub enum Expr {
    // ...
    Member(Box<Expr>, String),  // base_expr.member_name
}
```

`Member` を独立したExprノードにする理由：
- メンバアクセスはアドレス計算（`gen_addr`）と値ロード（`gen_expr`）の両方が必要
- 代入の左辺値（lvalue）としても使える（`s.x = 10;`）

## コード生成の変更 (`codegen.rs`)

### gen_addr: メンバのアドレス計算

```rust
Expr::Member(base, name) => {
    self.gen_addr(base);  // base struct address → %rax
    // Find member and add offset
    let member = /* find member by name */;
    if member.offset > 0 {
        emit "add $offset, %rax"
    }
}
```

構造体変数のアドレスを `gen_addr` で取得し、メンバのオフセットを加算する。

例: `s.y`（`s` は `struct { int x; int y; }` で `rbp-8` に配置）

```asm
lea -8(%rbp), %rax    # s のアドレス
add $4, %rax          # y のオフセット (4) を加算
```

### gen_expr: メンバの値をロード

```rust
Expr::Member(_, _) => {
    self.gen_addr(expr);  // member address → %rax
    let ty = self.expr_type(expr);
    self.emit_load_indirect(&ty);  // load value from address
}
```

メンバのアドレスを計算してから、型に応じた間接ロード命令を実行する。

### Assign: メンバへの代入

```rust
Expr::Deref(_) | Expr::Member(_, _) => {
    self.push();          // save rhs value
    self.gen_addr(lhs);   // lhs address → %rax
    emit "mov %rax, %rdi" // address → %rdi
    self.pop("%rax");     // restore rhs value → %rax
    self.emit_store_indirect(&ty);  // store based on type
}
```

メンバへの代入は `Deref` への代入と同じパターン。右辺値をスタックに退避し、左辺のアドレスを計算、型に応じたストア命令を実行する。

### emit_load_var: 構造体変数のロード

構造体変数を式として参照すると、配列と同様にアドレスが返る（`lea` 命令）。これは構造体のアドレスを通じてメンバにアクセスするため。

```asm
# local struct variable
lea -16(%rbp), %rax

# global struct variable
lea s(%rip), %rax
```

### expr_type: メンバの型推論

```rust
Expr::Member(base, name) => {
    let base_ty = self.expr_type(base);
    if let TypeKind::Struct(members) = &base_ty.kind {
        members.iter().find(|m| m.name == *name)
            .map(|m| m.ty.clone())
            .unwrap_or(Type::int_type())
    }
}
```

ベース式の型から構造体定義を取得し、メンバ名で検索して型を返す。

## 具体的なコード生成例

### `s.x = 1; s.y = 2; return s.x + s.y;`

構造体 `struct { int x; int y; }` がスタック上の `rbp-8` に配置されている場合：

```asm
# s.x = 1
  mov $1, %rax          # rhs = 1
  push %rax             # save rhs
  lea -8(%rbp), %rax    # address of s
  # offset of x is 0, no add needed
  mov %rax, %rdi        # address → %rdi
  pop %rax              # value → %rax
  movl %eax, (%rdi)     # store int to s.x

# s.y = 2
  mov $2, %rax          # rhs = 2
  push %rax             # save rhs
  lea -8(%rbp), %rax    # address of s
  add $4, %rax          # offset of y = 4
  mov %rax, %rdi        # address → %rdi
  pop %rax              # value → %rax
  movl %eax, (%rdi)     # store int to s.y

# s.x + s.y
  # load s.y
  lea -8(%rbp), %rax    # address of s
  add $4, %rax          # offset of y
  movslq (%rax), %rax   # load int (sign-extend)
  push %rax             # push s.y
  # load s.x
  lea -8(%rbp), %rax    # address of s
  movslq (%rax), %rax   # load int (sign-extend)
  pop %rdi              # pop s.y → %rdi
  add %rdi, %rax        # s.x + s.y → %rax
```

## テストケース

```bash
# basic member access
assert 3 'int main() { struct { int x; int y; } s; s.x = 1; s.y = 2; return s.x + s.y; }'
assert 10 'int main() { struct { int x; int y; } s; s.x = 10; return s.x; }'
assert 20 'int main() { struct { int x; int y; } s; s.y = 20; return s.y; }'

# sizeof struct
assert 8 'int main() { return sizeof(struct { int x; int y; }); }'
assert 8 'int main() { return sizeof(struct { char a; int b; }); }'

# three members
assert 5 'int main() { struct { int a; int b; int c; } s; s.a = 1; s.b = 2; s.c = 2; return s.a + s.b + s.c; }'

# mixed types with padding
assert 1 'int main() { struct { char a; int b; } s; s.a = 1; return s.a; }'
assert 42 'int main() { struct { char a; int b; } s; s.b = 42; return s.b; }'

# pointer to struct member
assert 3 'int main() { struct { int x; } s; s.x = 3; int *p = &s.x; return *p; }'
assert 7 'int main() { struct { int a; int b; } s; s.a = 3; s.b = 4; int *p = &s.b; return s.a + *p; }'
```
