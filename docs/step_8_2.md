# Step 8.2: typedef

## 概要

`typedef` で既存の型に別名を付ける機能を実装する。

```c
typedef int MyInt;
MyInt a = 42;

typedef struct { int x; int y; } Point;
Point p;
p.x = 1;
```

## typedefの仕組み

### 基本

```c
typedef existing_type new_name;
```

パーサーの段階で `new_name` を `existing_type` にマッピングする辞書を管理する。以降、`new_name` が型名として使われた場合、辞書から元の型を取得する。

### 実行時のコスト

typedefは完全にコンパイル時の機能。実行時のオーバーヘッドはゼロ。パーサーが型名を解決した時点で元の型に置き換わるため、コード生成から見ると直接型名を書いた場合と完全に同一。

## パーサーの変更

### typedef辞書

```rust
pub struct Parser<'a> {
    // ...
    typedefs: HashMap<String, Type>,
}
```

### トップレベルのtypedef

`parse()` ループでtypedefを処理：

```rust
if self.current().kind == TokenKind::Typedef {
    self.advance();
    let ty = self.parse_type();
    let name = parse_ident();
    expect(Semicolon);
    self.typedefs.insert(name, ty);
    continue;
}
```

### ローカルスコープのtypedef

`stmt()` で `Typedef` トークンを処理：

```rust
TokenKind::Typedef => {
    self.advance();
    let ty = self.parse_type();
    let name = parse_ident();
    expect(Semicolon);
    self.typedefs.insert(name, ty);
    Stmt::Block(vec![])  // no runtime effect
}
```

### 型名の解決

**parse_type()**: デフォルトケースでtypedef名を解決

```rust
_ => {
    if let TokenKind::Ident(name) = &self.current().kind {
        if let Some(ty) = self.typedefs.get(name).cloned() {
            self.advance();
            return ty;
        }
    }
    error("expected type");
}
```

**stmt()**: 式文の前にtypedef名チェック

```rust
_ => {
    // Check for typedef name as type → var_decl
    if let TokenKind::Ident(name) = &self.current().kind {
        if self.typedefs.contains_key(name) {
            return self.var_decl();
        }
    }
    // ... normal expression statement
}
```

**is_type_start()**: typedef名も型の先頭として認識

```rust
fn is_type_start(&self, kind: &TokenKind) -> bool {
    if Self::is_type_keyword(kind) { return true; }
    if let TokenKind::Ident(name) = kind {
        return self.typedefs.contains_key(name);
    }
    false
}
```

## テストケース

```bash
# basic typedef
assert 42 'int main() { typedef int MyInt; MyInt a = 42; return a; }'

# typedef pointer
assert 3 'int main() { typedef int *IntPtr; int a = 3; IntPtr p = &a; return *p; }'

# sizeof with typedef
assert 4 'int main() { typedef int MyInt; return sizeof(MyInt); }'

# top-level typedef for function params/return
assert 5 'typedef int MyInt; MyInt add(MyInt a, MyInt b) { return a + b; }
  int main() { return add(2, 3); }'

# typedef struct
assert 3 'int main() { typedef struct { int x; int y; } Point;
  Point p; p.x = 1; p.y = 2; return p.x + p.y; }'
```
