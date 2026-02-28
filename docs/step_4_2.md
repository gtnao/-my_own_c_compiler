# Step 4.2: char型

## 概要

1バイトの **`char`** 型を実装する。
`char` はC言語の最小の整数型で、ASCII文字を格納するために使われる。
x86-64 では 1 バイト（8ビット）で表現され、メモリの読み書きには
バイト専用の命令を使用する。

```c
int main() { char a = 65; return a; }  // => 65 (ASCII 'A')
```

## char型のメモリレイアウト

| 特性 | int | char |
|------|-----|------|
| サイズ | 8 bytes | 1 byte |
| アライメント | 8 bytes | 1 byte |
| ロード命令 | `mov` (64-bit) | `movsbq` (sign-extend byte→quad) |
| ストア命令 | `mov` (64-bit) | `movb` (byte) |
| レジスタ | `%rax` | `%al` (low byte of `%rax`) |

## 実装

### 1. トークンとレキサー

`char` キーワードを追加：

```rust
// token.rs
Char,  // "char" keyword

// lexer.rs
"char" => TokenKind::Char,
```

### 2. 型の追加

```rust
// types.rs
pub enum Type {
    Void,
    Char, // 1 byte, signed
    Int,  // 8 bytes
}

impl Type {
    pub fn size(&self) -> usize {
        match self {
            Type::Char => 1,
            // ...
        }
    }
    pub fn align(&self) -> usize {
        match self {
            Type::Char => 1,
            // ...
        }
    }
}
```

### 3. パーサーの変更

#### 型判定の一般化

`is_type_keyword()` ヘルパーで型キーワードを判定：

```rust
fn is_type_keyword(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Int | TokenKind::Char | TokenKind::Void)
}
```

これにより、以下の箇所で char が自動的にサポートされる：
- `is_function()` — 関数 vs グローバル変数の判定
- `stmt()` — 変数宣言のトリガー
- `for` 文の init 句 — 変数宣言のトリガー

#### `parse_type()` の拡張

```rust
TokenKind::Char => {
    self.advance();
    Type::Char
}
```

### 4. コード生成の変更

#### 型に応じた命令選択

コード生成の中核となる変更は、変数のロード・ストア命令を
型に応じて切り替えること。

**ロード（`emit_load_var`）:**

```rust
fn emit_load_var(&mut self, name: &str) {
    let ty = self.get_var_type(name).clone();
    if self.globals.contains(name) {
        match ty {
            Type::Char => self.emit(&format!("  movsbq {}(%rip), %rax", name)),
            Type::Int  => self.emit(&format!("  mov {}(%rip), %rax", name)),
            _ => {}
        }
    } else {
        let offset = self.locals[name];
        match ty {
            Type::Char => self.emit(&format!("  movsbq -{}(%rbp), %rax", offset)),
            Type::Int  => self.emit(&format!("  mov -{}(%rbp), %rax", offset)),
            _ => {}
        }
    }
}
```

**ストア（`emit_store_var`）:**

```rust
fn emit_store_var(&mut self, name: &str) {
    let ty = self.get_var_type(name).clone();
    if self.globals.contains(name) {
        match ty {
            Type::Char => self.emit(&format!("  movb %al, {}(%rip)", name)),
            Type::Int  => self.emit(&format!("  mov %rax, {}(%rip)", name)),
            _ => {}
        }
    } else {
        let offset = self.locals[name];
        match ty {
            Type::Char => self.emit(&format!("  movb %al, -{}(%rbp)", offset)),
            Type::Int  => self.emit(&format!("  mov %rax, -{}(%rbp)", offset)),
            _ => {}
        }
    }
}
```

#### `movsbq` — Sign-Extend Byte to Quadword

```asm
movsbq -1(%rbp), %rax
```

この命令は：
1. メモリから 1 バイトを読み取る
2. **符号拡張**して 64 ビットに変換
3. 結果を `%rax` に格納

符号拡張の例：
- `0x41` (65, 'A') → `0x0000000000000041`
- `0xFF` (-1 as signed byte) → `0xFFFFFFFFFFFFFFFF`

符号拡張を使う理由は、C 言語の「整数昇格」規則により、
`char` が式で使われると自動的に `int` に昇格されるため。

#### `movb` — Move Byte

```asm
movb %al, -1(%rbp)
```

`%al` は `%rax` の最下位 1 バイト。
8 ビットのデータだけをメモリに書き込む。

#### 関数パラメータの型対応

パラメータが `char` の場合、引数レジスタの下位バイトだけを保存：

```rust
let arg_regs_64 = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
let arg_regs_8  = ["%dil", "%sil", "%dl",  "%cl",  "%r8b", "%r9b"];
```

| 64-bit | 8-bit (low byte) |
|--------|-------------------|
| `%rdi` | `%dil` |
| `%rsi` | `%sil` |
| `%rdx` | `%dl` |
| `%rcx` | `%cl` |
| `%r8`  | `%r8b` |
| `%r9`  | `%r9b` |

#### スタックオフセットのアライメント対応

異なるサイズの変数が混在する場合、各変数のアライメントに合わせる：

```rust
let mut offset = 0;
for (ty, name) in &func.locals {
    let size = ty.size();
    let align = ty.align();
    offset = (offset + align - 1) & !(align - 1);  // align first
    offset += size;
    self.locals.insert(name.clone(), offset);
}
```

例：`[(Char, "a"), (Int, "b")]`
- `a`: align 0 to 1 → 0, +1 = 1, offset = 1 → `-1(%rbp)`
- `b`: align 1 to 8 → 8, +8 = 16, offset = 16 → `-16(%rbp)`

#### 型情報の追跡

変数の型を参照するため、`local_types` と `global_types` を追加：

```rust
pub struct Codegen {
    local_types: HashMap<String, Type>,
    global_types: HashMap<String, Type>,
    // ...
}

fn get_var_type(&self, name: &str) -> &Type {
    if let Some(ty) = self.local_types.get(name) {
        return ty;
    }
    if let Some(ty) = self.global_types.get(name) {
        return ty;
    }
    &Type::Int  // fallback
}
```

## 具体例

入力: `int main() { char a = 65; return a; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # char a = 65;
  mov $65, %rax
  movb %al, -1(%rbp)       # 1バイトだけ保存

  # return a;
  movsbq -1(%rbp), %rax    # 符号拡張して64ビットに
  jmp .Lreturn.main

  mov $0, %rax
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret
```

## テスト

ユニットテスト 19 件（1 件追加）+ 統合テスト 132 件（6 件追加）= 151 件
