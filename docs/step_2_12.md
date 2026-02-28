# Step 2.12: ビット演算子

## 概要

`&`（ビットAND）、`|`（ビットOR）、`^`（ビットXOR）、`~`（ビットNOT）、
`<<`（左シフト）、`>>`（右シフト）をサポートする。

## ビット演算と論理演算の違い

| 演算 | ビット演算 | 論理演算 |
|------|-----------|----------|
| AND  | `a & b` — ビットごとにAND | `a && b` — 短絡評価、結果は0か1 |
| OR   | `a \| b` — ビットごとにOR | `a \|\| b` — 短絡評価、結果は0か1 |
| NOT  | `~a` — 全ビット反転 | `!a` — 結果は0か1 |

```c
// ビット演算の例
3 & 1  = 1     // 0b0011 & 0b0001 = 0b0001
1 | 2  = 3     // 0b0001 | 0b0010 = 0b0011
1 ^ 2  = 3     // 0b0001 ^ 0b0010 = 0b0011
~2     = -3    // 全ビット反転（2の補数）
1 << 3 = 8     // 0b0001 → 0b1000
8 >> 2 = 2     // 0b1000 → 0b0010
```

## 演算子の優先順位（完全版）

ビット演算子の追加で、C言語の完全な優先順位チェーンになった：

```
assign        (最低)
  ↓
logical_or    ( || )
  ↓
logical_and   ( && )
  ↓
bitwise_or    ( | )       ← NEW
  ↓
bitwise_xor   ( ^ )       ← NEW
  ↓
bitwise_and   ( & )       ← NEW
  ↓
equality      ( == != )
  ↓
relational    ( < <= > >= )
  ↓
shift         ( << >> )    ← NEW
  ↓
add           ( + - )
  ↓
mul           ( * / % )
  ↓
unary         ( + - ! ~ ++ -- )   ← ~ を追加
  ↓
postfix       ( ++ -- )
  ↓
primary       (最高)
```

重要なポイント：
- ビットAND (`&`) は等値比較 (`==`) より**低い**優先順位
  → `a == b & c` は `a == (b & c)` ではなく `(a == b) & c`
- シフト (`<<`, `>>`) は加算 (`+`) より**高い**優先順位
  → `a + b << c` は `(a + b) << c` ではなく `a + (b << c)`

## トークン

```rust
Tilde,    // ~
Amp,      // &
Pipe,     // |
Caret,    // ^
LShift,   // <<
RShift,   // >>
```

### lexer の曖昧性解決

`&` と `&&`、`|` と `||`、`<` と `<<` と `<=`、`>` と `>>` と `>=` は
先頭文字が共通するため、2文字トークンを先にチェックする：

```rust
// 先に && をチェック
if ch == '&' && self.peek_next() == Some('&') { ... }
// マッチしなければ & 単体
'&' => TokenKind::Amp

// 先に << をチェック
if ch == '<' && self.peek_next() == Some('<') { ... }
// 次に <= をチェック
if ch == '<' && self.peek_next() == Some('=') { ... }
// マッチしなければ < 単体
'<' => TokenKind::Lt
```

## コード生成

### `&`, `|`, `^` — x86-64の対応命令

これらは BinOp の既存パターン（両辺評価→演算）で処理できる：

```rust
BinOp::BitAnd => {
    self.emit("  and %rdi, %rax");   // rax = rax & rdi
}
BinOp::BitOr => {
    self.emit("  or %rdi, %rax");    // rax = rax | rdi
}
BinOp::BitXor => {
    self.emit("  xor %rdi, %rax");   // rax = rax ^ rdi
}
```

### `~` — ビット反転

```rust
UnaryOp::BitNot => {
    self.emit("  not %rax");   // rax = ~rax (全ビット反転)
}
```

`not` 命令はフラグレジスタを変更しない（`neg` はフラグを変更する）。

### `<<`, `>>` — シフト演算

x86-64のシフト命令は、シフト量を `%cl`（`%rcx` の下位8ビット）に入れる必要がある：

```rust
BinOp::Shl => {
    self.emit("  mov %rdi, %rcx");   // シフト量を %rcx に移動
    self.emit("  sal %cl, %rax");     // rax = rax << cl
}
BinOp::Shr => {
    self.emit("  mov %rdi, %rcx");   // シフト量を %rcx に移動
    self.emit("  sar %cl, %rax");     // rax = rax >> cl (算術右シフト)
}
```

#### なぜ `%cl` を使うのか

x86-64のシフト命令は、即値（定数）か `%cl` レジスタのどちらかしかシフト量として
受け付けない。`%rdi` には右辺の値が入っているので、`%rcx` に転送してから使う。

```
%cl = %rcx の下位8ビット
```

8ビットで十分な理由：64ビット値のシフトでも最大63ビットシフトすれば全ビットが
消えるので、8ビット（0〜255）のシフト量で足りる。

#### `sal` vs `shl` / `sar` vs `shr`

- `sal`（Shift Arithmetic Left）= `shl`（Shift Logical Left）：左シフトは同じ
- `sar`（Shift Arithmetic Right）：符号ビットを保持（符号付き整数向け）
- `shr`（Shift Logical Right）：0で埋める（符号なし整数向け）

現時点では `int` 型（符号付き）のみなので `sar` を使用。

### 具体例

入力: `int main() { return 1 << 3; }`

```asm
  # BinOp evaluation
  mov $3, %rax           # rhs: 3 (shift amount)
  push %rax
  mov $1, %rax           # lhs: 1 (value to shift)
  pop %rdi               # rdi = 3

  # Shift left
  mov %rdi, %rcx         # rcx = 3 (shift amount must be in %cl)
  sal %cl, %rax          # rax = 1 << 3 = 8
```

入力: `int main() { int a = 2; return ~a & 255; }`

```asm
  # int a = 2;
  mov $2, %rax
  mov %rax, -8(%rbp)

  # ~a & 255
  # BinOp evaluation (rhs first)
  mov $255, %rax         # rhs: 255
  push %rax

  # lhs: ~a
  mov -8(%rbp), %rax     # rax = 2 = 0b...00000010
  not %rax               # rax = ~2 = 0b...11111101 = -3 (64-bit)

  pop %rdi               # rdi = 255
  and %rdi, %rax         # rax = 0b...11111101 & 0b...11111111 = 253
```

`~2 & 255 = 253` となる理由：`~2` は64ビットで全ビット反転されるため非常に大きな値になるが、
`& 255`（= `& 0xFF`）で下位8ビットだけを取り出すと `0b11111101 = 253` になる。
