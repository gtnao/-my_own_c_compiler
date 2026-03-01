# Step 20.3: タグ付き型による前方宣言された構造体の解決

## 概要

このステップでは、前方宣言された構造体の解決における重大なパフォーマンスおよび正確性の問題を修正します。この問題により、PostgreSQLの複雑なヘッダ（500以上の構造体定義、790以上のtypedef）を処理する際にコンパイラがハングしていました。

## 問題点

### 1. O(n^2) のパフォーマンス

以前の実装では、前方宣言された構造体が定義されるたびに `update_struct_members_with_struct` が呼び出されていました。この関数は前方参照を見つけて置換するために、**すべての** struct_tags と**すべての** typedef をスキャンしていました。

```
各構造体定義ごとに (N ≈ 500):
    すべての struct_tags (≈ 500) + すべての typedefs (≈ 790) をスキャン
合計: 500 × (500 + 790) = 645,000 回の操作
```

各操作には型ツリーのクローンとトラバーサルが含まれており、PostgreSQLの `executor/spi.h` でコンパイラがハングする原因となっていました。

### 2. 正確性のバグ: タグなしの空構造体の置換

`TypeKind::Struct(Vec<StructMember>)` にはタグ名がありませんでした。すべての空の（前方宣言された）構造体は同一に見えていました。

```rust
// Both forward declarations looked the same:
Struct([])  // Could be struct A or struct B!
```

構造体Aが定義された際、`replace_empty_struct_shallow` は**任意の**空構造体（構造体Bの前方宣言を含む）をAの完全な定義で置換してしまい、暗黙のうちに誤った型を生成していました。

## 解決策

### タグ付き構造体バリアント

`TypeKind::Struct` にオプションのタグ名を持たせるように変更しました。

```rust
// Before:
TypeKind::Struct(Vec<StructMember>)

// After:
TypeKind::Struct(Option<String>, Vec<StructMember>)
```

これにより、空構造体がどの前方宣言を表しているかを正確に識別できます。

```rust
Struct(Some("Node"), vec![])     // Forward-declared Node
Struct(Some("List"), vec![])     // Forward-declared List — now distinguishable!
Struct(None, vec![...])          // Anonymous struct
```

### `replace_tagged_empty_struct` によるターゲット指定の解決

置換関数はタグ名を確認してから置換を行うようになりました。

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

### `resolve_forward_refs` による遅延解決

各構造体定義時にすべての型を即座に更新する代わりに、前方参照は解析終了後に一括パスで解決されます。

1. 解析中、`resolved_forward_tags` が前方宣言された後に定義されたタグを追跡する
2. すべての宣言が解析された後、`resolve_forward_refs()` がそれらの特定のタグのみを処理する
3. 解決された各タグについて、実際にそれを参照する struct_tags と typedef のみが更新される

これにより作業量が O(n^2) から O(k * n) に削減されます。ここで k は前方宣言されたタグの数で、通常 n よりもはるかに小さい値です。

### `struct_defs` によるコード生成時の解決

`struct Node { int val; Node *next; }` のような自己参照構造体では、`next` メンバの型は `Ptr(Struct("Node", []))` となります。完全な定義を含めると無限のネストが発生するため、そのままでは格納できません。

解決策: 構造体定義を `Program.struct_defs` 経由でパーサーからコード生成に渡します。

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

これはメンバアクセスポイント（`.`、`->`）および `expr_type()` で呼び出され、前方参照をオンデマンドで解決します。

## 変更されたファイル

- `src/types.rs` -- `TypeKind::Struct(Option<String>, Vec<StructMember>)`: タグ名を追加
- `src/ast.rs` -- `Program.struct_defs`: 構造体定義をコード生成に渡す
- `src/parser.rs` -- タグ付き構造体の生成、`resolve_forward_refs()`、O(n^2) の更新を削除
- `src/codegen.rs` -- 遅延メンバ解決のための `resolve_struct_type()`

## 検証

- 既存の578テストすべてがパス
- PostgreSQL `executor/spi.h`（前処理後29,051行）が数秒でコンパイル（以前は無限にハング）
- 自己参照構造体（`struct Node { Node *next; }`）が正しく動作
