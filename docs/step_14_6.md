# Step 14.6: `signed`キーワード

## 概要

`signed`型指定子のサポートを追加します。`signed`は整数型のデフォルトの符号付き属性であるため、`signed int`は`int`と等価、`signed char`は`char`と等価、というように対応します。ベアの`signed`キーワード（後続の型なし）は`signed int`として扱われます。

## なぜ必要か

PostgreSQLやシステムヘッダでは、`signed`が明示的に使用されることがあります:

```c
signed char sc;
signed int x;
signed long val;
```

`int`/`short`/`long`に対して`signed`は冗長ですが（これらはデフォルトで符号付き）、`signed char`は一部のプラットフォームで`char`がデフォルトで符号なしとなりうるため、`char`とは区別されます。

## 実装

### トークン

`TokenKind`に`Signed`バリアントを追加しました。

### レキサー

`signed`をキーワードとして認識し、`TokenKind::Signed`にマッピングします。

### パーサー

`signed`は`parse_type()`内で`unsigned`と並んで処理されます:

```rust
let mut has_signedness = false;
let is_unsigned = if self.current().kind == TokenKind::Unsigned {
    self.advance();
    has_signedness = true;
    true
} else {
    if self.current().kind == TokenKind::Signed {
        self.advance();
        has_signedness = true;
    }
    false
};
```

`has_signedness`フラグは、ベアの`signed`（後続の型キーワードなし）を処理するために使用されます:
- `signed int` → `int`（通常の符号付きint）
- `signed char` → `char`（符号付きchar）
- `signed`単独 → `int`（ベアの`unsigned` → `unsigned int`と同様）

`TokenKind::Signed`を以下に追加しました:
- `is_type_keyword()` — 型宣言の一部として認識
- `stmt()`のマッチ — 変数宣言の開始として認識

## 動作

- `signed`は消費され、結果の型を変更しない（本コンパイラではすべての整数型がデフォルトで符号付きであるため）
- `signed`は`int`、`char`、`short`、`long`の前に置くか、単独で使用可能
- ベアの`signed`は`signed int`にデフォルト設定

## テストケース

```c
int main() { signed int a = 5; return a; }    // → 5
int main() { signed char c = 3; return c; }    // → 3
int main() { signed a = 7; return a; }         // → 7 (bare signed = int)
int main() { signed short s = 10; return s; }  // → 10
int main() { signed long l = 42; return l; }   // → 42
```
