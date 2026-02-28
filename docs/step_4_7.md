# Step 4.7: unsigned型

## 概要

`unsigned` 修飾子を実装し、符号なし整数型をサポートする。
`unsigned char`, `unsigned short`, `unsigned int`, `unsigned long` および
`unsigned`（`unsigned int` の省略形）が使えるようになる。

```c
unsigned char a = 200;   // 0〜255
unsigned int b = 42;     // 0〜4294967295
unsigned long c = 100;   // 0〜2^64-1
unsigned d = 42;         // unsigned int と同義
```

## 型システムのリファクタリング

### 旧設計（enum）

```rust
pub enum Type {
    Void, Char, Short, Int, Long,
}
```

5つの型をそれぞれ独立した enum variant で表現していた。

### 新設計（struct + enum）

```rust
pub enum TypeKind {
    Void, Char, Short, Int, Long,
}

pub struct Type {
    pub kind: TypeKind,
    pub is_unsigned: bool,
}
```

型の「種類」と「符号の有無」を直交する2軸として分離。
これにより、型のバリエーションが `5 × 2 = 10` に拡張される一方、
サイズやアライメントの計算は `kind` のみに依存するためコードの重複を避けられる。

### コンストラクタ

```rust
impl Type {
    // Signed
    pub fn void() -> Self { ... }
    pub fn char_type() -> Self { ... }
    pub fn short_type() -> Self { ... }
    pub fn int_type() -> Self { ... }
    pub fn long_type() -> Self { ... }

    // Unsigned
    pub fn uchar() -> Self { ... }
    pub fn ushort() -> Self { ... }
    pub fn uint() -> Self { ... }
    pub fn ulong() -> Self { ... }
}
```

## パーサーの変更

### トークン

```rust
// token.rs
Unsigned,  // "unsigned" keyword

// lexer.rs
"unsigned" => TokenKind::Unsigned,
```

### 型のパース

`parse_type()` は `unsigned` キーワードを先読みし、後続の型と組み合わせる：

```rust
fn parse_type(&mut self) -> Type {
    let is_unsigned = if self.current().kind == TokenKind::Unsigned {
        self.advance();
        true
    } else {
        false
    };

    match self.current().kind {
        TokenKind::Int => {
            self.advance();
            if is_unsigned { Type::uint() } else { Type::int_type() }
        }
        // ... Char, Short, Long も同様 ...
        _ => {
            if is_unsigned {
                Type::uint()  // bare "unsigned" = "unsigned int"
            } else {
                // error
            }
        }
    }
}
```

### is_function() の改良

`unsigned int main()` のような複数トークンの型指定子に対応するため、
型キーワードを可変長でスキップしてから識別子と `(` を確認する：

```rust
fn is_function(&self) -> bool {
    let mut i = self.pos;
    while Self::is_type_keyword(&self.tokens[i].kind) {
        i += 1;
    }
    if let TokenKind::Ident(_) = &self.tokens[i].kind {
        return self.tokens[i + 1].kind == TokenKind::LParen;
    }
    false
}
```

## コード生成の変更

signed と unsigned の違いは**ロード時の拡張方法**に集約される。
ストア命令は同じ（単に下位ビットを書き込むだけ）。

### ロード命令の比較

| 型 | signed（符号拡張） | unsigned（ゼロ拡張） |
|---|---|---|
| char (1byte) | `movsbq src, %rax` | `movzbl src, %eax` |
| short (2byte) | `movswq src, %rax` | `movzwl src, %eax` |
| int (4byte) | `movslq src, %rax` | `movl src, %eax` |
| long (8byte) | `mov src, %rax` | `mov src, %rax` |

### 拡張方法の違い

**符号拡張（Sign Extension）**: 最上位ビット（符号ビット）を上位ビットにコピー。
- `movsbq`: 8bit → 64bit、ビット7を複製
- 例: `0xFF`（-1）→ `0xFFFFFFFFFFFFFFFF`（-1）

**ゼロ拡張（Zero Extension）**: 上位ビットをゼロで埋める。
- `movzbl`: 8bit → 32bit、上位ゼロ
- 例: `0xFF`（255）→ `0x000000FF`（255）

### x86-64のゼロ拡張の特性

x86-64 では **32ビットレジスタへの書き込みは自動的に上位32ビットをゼロクリア** する。
そのため：

- `movzbl src, %eax` → `%rax` 全体が正しくゼロ拡張される
- `movzwl src, %eax` → 同上
- `movl src, %eax` → 32bit unsigned int を 64bit に自動ゼロ拡張

この性質により、`movzbl`/`movzwl`/`movl` はすべて
32ビットの宛先レジスタ `%eax` を使えば 64ビットまでゼロ拡張される。

### キャスト命令

| キャスト | signed | unsigned |
|---|---|---|
| `(char)expr` | `movsbq %al, %rax` | `movzbl %al, %eax` |
| `(short)expr` | `movswq %ax, %rax` | `movzwl %ax, %eax` |
| `(int)expr` | `movslq %eax, %rax` | `movl %eax, %eax` |
| `(long)expr` | no-op | no-op |

`movl %eax, %eax` は一見無意味に見えるが、
32ビット操作の暗黙的ゼロ拡張により上位32ビットがクリアされる。
これは `unsigned int` へのキャストとして正しく機能する。

## ストア命令

ストアは signed/unsigned で共通。値の下位ビットをメモリに書き込むだけ：

```asm
movb %al, -1(%rbp)     # char (1byte)
movw %ax, -2(%rbp)     # short (2byte)
movl %eax, -4(%rbp)    # int (4byte)
mov  %rax, -8(%rbp)    # long (8byte)
```

符号の解釈はロード時に決まるため、ストア時には関係ない。

## サイズとアライメント

unsigned 型は signed と同じサイズ・アライメント：

| 型 | サイズ | アライメント |
|---|---|---|
| unsigned char | 1 | 1 |
| unsigned short | 2 | 2 |
| unsigned int | 4 | 4 |
| unsigned long | 8 | 8 |

`Type::size()` と `Type::align()` は `kind` のみで決定するため変更不要。

## テスト

ユニットテスト 22 件 + 統合テスト 163 件（9 件追加）= 185 件
