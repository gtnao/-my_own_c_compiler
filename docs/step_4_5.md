# Step 4.5: 明示的型キャスト

## 概要

C言語の **明示的型キャスト** `(type)expr` を実装する。
プログラマが意図的に型を変換する操作で、暗黙の変換では起きない
truncation を明示的に行う場合などに使用する。

```c
int main() { return (char)256; }  // => 0 (256の下位8ビットは0)
```

## 構文

```
cast_expression = "(" type ")" unary_expression
```

キャスト式は単項式（unary）と同じ優先順位を持つ。

## パーサーでの判別

`(` に続くトークンが型キーワードかどうかで、キャストか括弧式かを判定：

```rust
// unary() 内
if self.current().kind == TokenKind::LParen
    && Self::is_type_keyword(&self.tokens[self.pos + 1].kind)
{
    self.advance();             // consume "("
    let ty = self.parse_type(); // parse the type
    self.expect(TokenKind::RParen);
    let operand = self.unary();
    return Expr::Cast { ty, expr: Box::new(operand) };
}
```

判別の例：
- `(int)42` → `(` の次が `int`（型キーワード）→ **キャスト**
- `(a + b)` → `(` の次が `a`（識別子）→ **括弧式**

## AST

```rust
pub enum Expr {
    // ...
    Cast {
        ty: Type,
        expr: Box<Expr>,
    },
}
```

## コード生成

キャストは「truncate（切り詰め）→ sign-extend（符号拡張）」の2段階：

```rust
Expr::Cast { ty, expr } => {
    self.gen_expr(expr);  // 値を %rax に
    match ty {
        Type::Char  => self.emit("  movsbq %al, %rax"),
        Type::Short => self.emit("  movswq %ax, %rax"),
        Type::Int   => self.emit("  movslq %eax, %rax"),
        Type::Long | Type::Void => {}  // no-op
    }
}
```

### 命令の動作

#### `movsbq %al, %rax` — char へのキャスト

1. `%al`（`%rax` の下位 8 ビット）だけを取り出す
2. ビット 7（符号ビット）を 64 ビットに拡張
3. 結果を `%rax` に格納

```
%rax = 0x0000000000000100 (256)
%al  = 0x00 (0)
movsbq %al, %rax
%rax = 0x0000000000000000 (0)
```

```
%rax = 0x0000000000000101 (257)
%al  = 0x01 (1)
movsbq %al, %rax
%rax = 0x0000000000000001 (1)
```

#### `movswq %ax, %rax` — short へのキャスト

1. `%ax`（下位 16 ビット）だけを取り出す
2. ビット 15（符号ビット）を 64 ビットに拡張

```
%rax = 0x0000000000010000 (65536)
%ax  = 0x0000 (0)
movswq %ax, %rax
%rax = 0x0000000000000000 (0)
```

#### `movslq %eax, %rax` — int へのキャスト

1. `%eax`（下位 32 ビット）を取り出す
2. ビット 31（符号ビット）を 64 ビットに拡張

#### long へのキャスト — no-op

すでに 64 ビットなので何もしない。

## テスト

ユニットテスト 21 件 + 統合テスト 150 件（6 件追加）= 171 件
