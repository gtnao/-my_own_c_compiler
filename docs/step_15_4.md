# Step 15.4: K&Rスタイル関数宣言

## 概要

K&R（Kernighan & Ritchie）スタイルの関数宣言は、古いC言語の構文で、パラメータの型をパラメータリストの後、関数本体の前に宣言する方式です。

```c
int add(a, b) int a; int b; { return a + b; }
```

モダンスタイルで書くと以下と同等です。
```c
int add(int a, int b) { return a + b; }
```

## 実装

パーサーは、パラメータリストの最初のトークンが型名ではない識別子であるかどうかを調べることで、K&Rスタイルを検出します。

1. **K&Rパラメータリストの解析**: `(ident, ident, ...)` のように、`ident` が型名でない場合、パラメータ名をデフォルトの `int` 型として解析します
2. **閉じ括弧後の型宣言**: `)` の後に `{` の前に型キーワードがある場合、`type name;` 形式の宣言を解析し、対応するパラメータの型を更新します

```rust
// Detect K&R: first token is identifier, not a type name
if let TokenKind::Ident(_) = self.current().kind {
    if !self.is_type_start(&self.current().kind) {
        // K&R: parse (a, b, c) as int-typed parameters
        is_kr_style = true;
        // ... collect parameter names with default int type
    }
}

// After RParen, parse K&R type declarations
// int add(a, b) int a; int b; { ... }
//               ^^^^^^^^^^^^^^^
while self.current().kind != TokenKind::LBrace {
    let kr_ty = self.parse_type();
    // Update matching parameter's type
}
```

## テストケース

```c
int add(a, b) int a; int b; { return a+b; }
int main() { return add(3, 4); }  // => 7

int mul(x, y) int x; int y; { return x*y; }
int main() { return mul(2, 3); }  // => 6
```
