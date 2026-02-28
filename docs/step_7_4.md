# Step 7.4: タグ付き構造体

## 概要

構造体にタグ名を付けて再利用可能にする機能を実装する。

```c
struct Point { int x; int y; };  // define tagged struct
struct Point p;                  // use tag to declare variable
p.x = 1;
p.y = 2;
```

## タグ付き構造体の仕組み

### 定義と参照

C言語では構造体にタグ名を付けることで、同じ型を複数箇所で使える：

```c
// 定義：struct Tag { members };
struct Point { int x; int y; };

// 参照：struct Tag
struct Point a;
struct Point b;
struct Point *p = &a;
```

タグ名は構造体の型情報（メンバ名、型、オフセット）を辞書に登録し、参照時に検索する。

### 定義のみの文

`struct Tag { ... };` はタグを定義するだけで変数は宣言しない。パーサーでは型パース後にセミコロンが来たら、空のブロック文として処理する。

## パーサーの変更

### タグ辞書

```rust
pub struct Parser<'a> {
    // ...
    struct_tags: HashMap<String, Type>,
}
```

### parse_type() の変更

構造体の型パースは3パターンに対応：

1. **`struct { ... }`** — 無名構造体（従来通り）
2. **`struct Tag { ... }`** — タグ付き定義（タグ辞書に登録 + 型を返す）
3. **`struct Tag`** — タグ参照（タグ辞書から検索）

```rust
TokenKind::Struct => {
    self.advance();
    // Check for tag name
    let tag_name = if Ident then Some(name) else None;

    if LBrace {
        // Parse body
        let ty = parse_members();
        // Register tag if present
        if tag_name { struct_tags.insert(tag, ty); }
        ty
    } else if tag_name {
        // Look up tag
        struct_tags.get(tag).clone()
    } else {
        error("expected struct tag or body")
    }
}
```

### var_decl() の変更

型パース後にセミコロンが来た場合（タグ定義のみ）、変数宣言なしとして空のブロック文を返す：

```rust
fn var_decl(&mut self) -> Stmt {
    let ty = self.parse_type();
    // Allow struct tag definition without variable declaration
    if self.current().kind == TokenKind::Semicolon {
        self.advance();
        return Stmt::Block(vec![]);
    }
    // ... normal variable declaration
}
```

### is_function() の改良

タグ付き構造体を返り値型とする関数を正しく識別するため、`is_function()` で `struct` の後のタグ名と本体 `{ ... }` をスキップする処理を追加：

```rust
if tokens[i].kind == TokenKind::Struct {
    i += 1;
    // Skip tag name
    if Ident { i += 1; }
    // Skip body { ... }
    if LBrace { skip_matching_braces(); }
}
```

これにより `struct S { int x; } func() { ... }` のような関数定義も正しく認識される。

## コード生成

コード生成の変更は不要。タグ付き構造体はパーサーの段階で `TypeKind::Struct(Vec<StructMember>)` に解決されるため、コード生成から見ると無名構造体と同一。

## テストケース

```bash
# basic tagged struct
assert 3 'int main() { struct Point { int x; int y; }; struct Point p;
  p.x = 1; p.y = 2; return p.x + p.y; }'

# single member
assert 10 'int main() { struct Foo { int a; }; struct Foo f; f.a = 10; return f.a; }'

# multiple variables of same tag
assert 7 'int main() { struct S { int x; int y; }; struct S a; struct S b;
  a.x = 3; b.x = 4; return a.x + b.x; }'

# tag with pointer
assert 42 'int main() { struct S { int val; }; struct S s; s.val = 42;
  struct S *p = &s; return p->val; }'
```
