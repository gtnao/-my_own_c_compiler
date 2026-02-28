# Step 4.6: sizeof演算子

## 概要

**`sizeof`** 演算子を実装する。`sizeof(type)` はコンパイル時に
型のバイトサイズを定数として評価する。

```c
sizeof(char)  // => 1
sizeof(short) // => 2
sizeof(int)   // => 4
sizeof(long)  // => 8
```

## 構文

現時点では `sizeof(type)` の形式のみサポート：

```
sizeof_expr = "sizeof" "(" type ")"
```

将来的に `sizeof expr`（式に対する sizeof）も追加予定。

## 実装

### 1. トークンとレキサー

```rust
// token.rs
Sizeof,  // "sizeof" keyword

// lexer.rs
"sizeof" => TokenKind::Sizeof,
```

### 2. AST

```rust
pub enum Expr {
    // ...
    SizeofType(Type),  // sizeof(type)
}
```

sizeof は式ではなく型に対して適用されるため、
`SizeofType(Type)` としてモデル化する。

### 3. パーサー

`unary()` のレベルでパース：

```rust
TokenKind::Sizeof => {
    self.advance();
    self.expect(TokenKind::LParen);
    let ty = self.parse_type();
    self.expect(TokenKind::RParen);
    return Expr::SizeofType(ty);
}
```

sizeof は単項演算子と同じ優先順位。

### 4. コード生成

sizeof はコンパイル時定数。`Type::size()` を呼ぶだけ：

```rust
Expr::SizeofType(ty) => {
    self.emit(&format!("  mov ${}, %rax", ty.size()));
}
```

生成されるアセンブリは即値のロードと同じ：

```asm
mov $1, %rax    # sizeof(char)
mov $2, %rax    # sizeof(short)
mov $4, %rax    # sizeof(int)
mov $8, %rax    # sizeof(long)
```

## sizeof の本質

sizeof は**コンパイル時演算子**であり、実行時に評価されない。
コンパイラが型情報から即座にサイズを計算し、定数として埋め込む。

これは前のステップで構築した型システム（`Type::size()`）が
活きる場面：型の定義さえ正しければ、sizeof は自動的に正しい値を返す。

## テスト

ユニットテスト 21 件 + 統合テスト 154 件（4 件追加）= 175 件
