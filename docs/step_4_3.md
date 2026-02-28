# Step 4.3: short型、long型

## 概要

**`short`**（2バイト）と **`long`**（8バイト）型を追加し、
同時に **`int`** を正しい4バイトサイズに変更する。
これにより、C言語の標準的な整数型が全て揃う。

## 整数型の全体像

| 型 | サイズ | アライメント | ロード命令 | ストア命令 | レジスタ |
|------|--------|------------|-----------|-----------|---------|
| `char` | 1 byte | 1 byte | `movsbq` | `movb` | `%al` |
| `short` | 2 bytes | 2 bytes | `movswq` | `movw` | `%ax` |
| `int` | 4 bytes | 4 bytes | `movslq` | `movl` | `%eax` |
| `long` | 8 bytes | 8 bytes | `mov` | `mov` | `%rax` |

### `int` のサイズ変更

Step 4.2 までは `int` を 8 バイト（64ビット）として扱っていたが、
x86-64 Linux の標準に合わせて 4 バイト（32ビット）に変更。

これは重要な変更で、以下に影響する：
- スタック上の変数サイズ
- ロード/ストア命令の選択
- `.comm` ディレクティブのサイズ
- 関数パラメータの保存命令

## 実装

### 1. 型定義の更新

```rust
pub enum Type {
    Void,
    Char,  // 1 byte
    Short, // 2 bytes
    Int,   // 4 bytes (changed from 8!)
    Long,  // 8 bytes
}
```

### 2. 命令選択

#### ロード命令（符号拡張）

各型のロードでは、値を **符号拡張** して 64 ビットレジスタに格納する：

```asm
movsbq -1(%rbp), %rax     # char:  sign-extend byte → quad
movswq -2(%rbp), %rax     # short: sign-extend word → quad
movslq -4(%rbp), %rax     # int:   sign-extend long → quad
mov    -8(%rbp), %rax     # long:  already quad, no extension
```

**符号拡張の意味：**
- `movsbq`: Move Sign-extend Byte to Quadword
  - 1バイトの符号ビット（bit 7）を 64 ビットに拡張
- `movswq`: Move Sign-extend Word to Quadword
  - 2バイトの符号ビット（bit 15）を 64 ビットに拡張
- `movslq`: Move Sign-extend Long to Quadword
  - 4バイトの符号ビット（bit 31）を 64 ビットに拡張

符号拡張が必要な理由：コンパイラは全ての演算を 64 ビットレジスタで行う。
小さい型の値をロードする際に符号拡張しないと、負の値が正の大きな値に
変わってしまう。

#### ストア命令

ストアでは、`%rax` の下位ビットだけを書き出す：

```asm
movb %al,  -1(%rbp)       # char:  store low byte
movw %ax,  -2(%rbp)       # short: store low word (2 bytes)
movl %eax, -4(%rbp)       # int:   store low long (4 bytes)
mov  %rax, -8(%rbp)       # long:  store full quad (8 bytes)
```

レジスタのサブレジスタ名：

```
%rax (64-bit)
├── %eax  (lower 32 bits)
│   ├── %ax   (lower 16 bits)
│   │   ├── %ah (bits 15-8)
│   │   └── %al (bits 7-0)
```

### 3. 関数パラメータのレジスタ名

各引数レジスタのサイズ別名：

| 64-bit | 32-bit | 16-bit | 8-bit |
|--------|--------|--------|-------|
| `%rdi` | `%edi` | `%di` | `%dil` |
| `%rsi` | `%esi` | `%si` | `%sil` |
| `%rdx` | `%edx` | `%dx` | `%dl` |
| `%rcx` | `%ecx` | `%cx` | `%cl` |
| `%r8` | `%r8d` | `%r8w` | `%r8b` |
| `%r9` | `%r9d` | `%r9w` | `%r9b` |

パラメータ保存時、型に応じた適切なサブレジスタを使用：

```rust
match ty {
    Type::Char  => emit("movb %dil, -N(%rbp)"),
    Type::Short => emit("movw %di, -N(%rbp)"),
    Type::Int   => emit("movl %edi, -N(%rbp)"),
    Type::Long  => emit("mov %rdi, -N(%rbp)"),
}
```

### 4. アライメント対応のスタックレイアウト

異なるサイズの変数が混在する場合の正しいレイアウト：

```rust
let mut offset = 0;
for (ty, name) in &func.locals {
    let align = ty.align();
    offset = (offset + align - 1) & !(align - 1);  // align first
    offset += ty.size();
    self.locals.insert(name.clone(), offset);
}
```

例：`char a; short b; int c; long d;`

```
%rbp
  -1: char a   (1 byte)
  -2: [padding 1 byte for short alignment]
  -4: short b  (2 bytes, aligned to 2)
  -8: int c    (4 bytes, aligned to 4)
 -16: long d   (8 bytes, aligned to 8)
```

### 5. グローバル変数の型対応

`.comm` ディレクティブが型のサイズとアライメントを反映：

```asm
  .comm g_char, 1, 1      # char:  1 byte,  1-byte aligned
  .comm g_short, 2, 2     # short: 2 bytes, 2-byte aligned
  .comm g_int, 4, 4       # int:   4 bytes, 4-byte aligned
  .comm g_long, 8, 8      # long:  8 bytes, 8-byte aligned
```

## 具体例

入力: `int main() { char a = 1; short b = 2; int c = 3; return a + b + c; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # char a = 1;
  mov $1, %rax
  movb %al, -1(%rbp)        # store 1 byte

  # short b = 2;
  mov $2, %rax
  movw %ax, -4(%rbp)        # store 2 bytes (aligned to 2)

  # int c = 3;
  mov $3, %rax
  movl %eax, -8(%rbp)       # store 4 bytes (aligned to 4)

  # return a + b + c;
  # a + b
  movswq -4(%rbp), %rax     # load b, sign-extend to 64-bit
  push %rax
  movsbq -1(%rbp), %rax     # load a, sign-extend to 64-bit
  pop %rdi
  add %rdi, %rax            # a + b in 64-bit

  # (a + b) + c
  push %rax
  movslq -8(%rbp), %rax     # load c, sign-extend to 64-bit
  ... (reversed: rhs pushed first)
  add %rdi, %rax            # result = 6

  jmp .Lreturn.main
```

## テスト

ユニットテスト 19 件 + 統合テスト 140 件（8 件追加）= 159 件
