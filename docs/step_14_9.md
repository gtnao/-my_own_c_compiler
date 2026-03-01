# ステップ 14.9: `register` ストレージクラス

## 概要

`register` ストレージクラス指定子のサポートを追加する。読み取られた後、無視される。すべてのローカル変数は `register` の有無にかかわらずスタック上に配置される。

## なぜ必要か

PostgreSQL や古い C コードでは、変数を CPU レジスタに保持してアクセスを高速化するヒントとして `register` を使用している:

```c
register int i;
register unsigned char *p = buf;
```

現代のコンパイラはこのヒントを無視して独自のレジスタ割り当てを行うが、互換性のためにキーワードを構文解析できる必要がある。

## 実装

### トークン

`TokenKind` に `Register` バリアントを追加した。

### 字句解析

`register` をキーワードとして認識し、`TokenKind::Register` にマッピングする。

### 構文解析

`register` は `parse_type()` において `inline` および `_Noreturn` と並んで型の前で読み取られ、無視される:

```rust
while matches!(self.current().kind,
    TokenKind::Inline | TokenKind::Noreturn | TokenKind::Register) {
    self.advance();
}
```

`is_type_keyword()` および `stmt()` の型開始パターンにも追加した。

## 動作

- `register` は読み取られ、完全に無視される
- `register` 付きで宣言された変数は、他のローカル変数と同様にスタック上に割り当てられる
- これは現代の GCC/Clang が `register` をストレージヒントとして無視する動作と一致する

## テストケース

```c
int main() { register int a = 5; return a; }    // → 5
int main() { register int i; i = 10; return i; } // → 10
```
