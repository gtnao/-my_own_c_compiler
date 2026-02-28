# Step 9.2: 構造体初期化子

## 概要

構造体宣言時にブレース初期化子 `{val1, val2, ...}` でメンバを初期化する機能を実装する。

```c
struct { int x; int y; } s = {1, 2};
return s.x + s.y;  // => 3
```

## 実装方針

配列初期化子（Step 9.1）と同じアプローチで、パーサーの段階でメンバごとの代入文に分解（desugar）する。

### 変換例

```c
struct { int x; int y; } s = {1, 2};
```

は以下に変換される：

```c
struct { int x; int y; } s;  // VarDecl (init: None)
s.x = 1;                     // ExprStmt(Assign to Member)
s.y = 2;                     // ExprStmt(Assign to Member)
```

## パーサーの変更

ブレース初期化子のパース部分で型を判定し、`TypeKind::Struct` の場合はメンバ代入を生成：

```rust
if let TypeKind::Struct(ref members) = ty.kind {
    // Struct initializer: assign to each member
    for (i, init_expr) in init_exprs.into_iter().enumerate() {
        if i < members.len() {
            stmts.push(ExprStmt(Assign {
                lhs: Member(Var(name), members[i].name),
                rhs: init_expr,
            }));
        }
    }
} else {
    // Array initializer (existing code)
}
```

配列の場合は `*(a + i) = val` を使い、構造体の場合は `s.member = val` を使う。

## テストケース

```bash
assert 3 'int main() { struct { int x; int y; } s = {1, 2}; return s.x + s.y; }'
assert 10 'int main() { struct { int a; int b; int c; } s = {1, 2, 7}; return s.a + s.b + s.c; }'
assert 42 'int main() { struct { int x; } s = {42}; return s.x; }'
assert 3 'int main() { struct S { int x; int y; }; struct S s = {1, 2}; return s.x + s.y; }'
```
