# ステップ 15.2: long long 型

## 概要

`long long` と `unsigned long long` は、`long` と同一に扱うことで既にサポートされている。x86-64では `long` も `long long` も8バイトであるため、別個の型バリアントは不要である。

パーサーは既に `long long` と `long long int` を処理しており、2つの連続した `Long` トークンとオプションの `Int` トークンを消費する:

```rust
TokenKind::Long => {
    self.advance();
    if self.current().kind == TokenKind::Long {
        self.advance();  // skip second "long"
        if self.current().kind == TokenKind::Int {
            self.advance();  // skip optional "int"
        }
    } else if self.current().kind == TokenKind::Int {
        self.advance();  // "long int"
    }
    if is_unsigned { Type::ulong() } else { Type::long_type() }
}
```

このステップは以前の実装で既に完了していた。
