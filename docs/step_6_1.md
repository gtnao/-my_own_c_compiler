# Step 6.1: 文字列リテラル

## 概要

ダブルクォートで囲まれた文字列リテラル `"hello"` を実装する。
文字列は `.rodata` セクションに配置され、式としてはその先頭アドレス
（`char *`）として扱われる。

```c
char *s = "hello";
s[0]; // 104 ('h')
s[5]; // 0   (null terminator)
```

## 文字列リテラルのメモリ配置

### .rodata セクション

文字列定数は読み取り専用データとして `.rodata` セクションに配置される。
各文字列にラベル `.LC0`, `.LC1`, ... を割り当てる。

```asm
  .section .rodata
.LC0:
  .byte 104,101,108,108,111,0    # "hello\0"
.LC1:
  .byte 97,98,99,0               # "abc\0"
```

`.byte` ディレクティブを使い、各文字のASCIIコードを列挙する。
最後に null terminator (0) を付加する。

### なぜ .rodata なのか

- `.data`: 読み書き可能なデータ（グローバル変数）
- `.rodata`: 読み取り専用データ（文字列定数、定数テーブル）
- `.bss`: 初期値なし（ゼロ初期化）のデータ

文字列リテラルは変更されるべきでないため `.rodata` に配置する。
（Cの仕様上、文字列リテラルの変更は未定義動作）

## 実装

### トークン

```rust
pub enum TokenKind {
    // ...
    Str(String),  // string literal
}
```

### レキサー

ダブルクォートの開始を検出し、閉じクォートまでの文字を読み取る。
基本的なエスケープシーケンスもこの段階で処理する。

```rust
fn read_string(&mut self) -> String {
    self.pos += 1; // skip opening '"'
    let mut s = String::new();
    while self.pos < self.input.len() {
        let c = self.input[self.pos] as char;
        if c == '"' {
            self.pos += 1; // skip closing '"'
            return s;
        }
        if c == '\\' {
            self.pos += 1;
            let escaped = match self.input[self.pos] as char {
                'n' => '\n',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                '0' => '\0',
                other => other,
            };
            s.push(escaped);
            self.pos += 1;
            continue;
        }
        s.push(c);
        self.pos += 1;
    }
    s
}
```

サポートするエスケープシーケンス（Step 6.1）:
- `\n` → 改行 (0x0A)
- `\t` → タブ (0x09)
- `\\` → バックスラッシュ
- `\"` → ダブルクォート
- `\0` → null (0x00)

### AST

```rust
pub enum Expr {
    // ...
    StrLit(String),  // string literal
}
```

### パーサー

`primary()` で `Str` トークンを `StrLit` ノードに変換。

```rust
TokenKind::Str(s) => {
    self.advance();
    Expr::StrLit(s)
}
```

### コード生成

文字列定数を収集し、`.rodata` セクションに出力する。

```rust
// gen_expr
Expr::StrLit(s) => {
    let idx = self.string_literals.len();
    self.string_literals.push(s.clone());
    self.emit(&format!("  lea .LC{}(%rip), %rax", idx));
}

// generate() の末尾
let strings = self.string_literals.clone();
if !strings.is_empty() {
    self.emit("  .section .rodata");
    for (i, s) in strings.iter().enumerate() {
        self.emit(&format!(".LC{}:", i));
        let mut bytes: Vec<String> = s.as_bytes()
            .iter().map(|b| format!("{}", b)).collect();
        bytes.push("0".to_string()); // null terminator
        self.emit(&format!("  .byte {}", bytes.join(",")));
    }
}
```

### 型

文字列リテラルの型は `char *`（char へのポインタ）として扱う。

```rust
Expr::StrLit(_) => Type::ptr_to(Type::char_type()),
```

## 生成されるアセンブリ例

```c
char *s = "hello";
return s[0];
```

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # s = "hello"
  lea .LC0(%rip), %rax     # string address
  mov %rax, -8(%rbp)       # store pointer to s

  # s[0] = *(s + 0)
  mov $0, %rax
  push %rax
  mov -8(%rbp), %rax       # load s (pointer)
  pop %rdi
  # imul $1, %rdi skipped (sizeof(char) == 1)
  add %rdi, %rax           # s + 0
  movsbq (%rax), %rax      # load char (sign-extend)
  jmp .Lreturn.main

  ...

  .section .rodata
.LC0:
  .byte 104,101,108,108,111,0   # "hello\0"
```

## 文字列への添字アクセス

文字列リテラルは `char *` 型なので、添字演算子 `[i]` が使える。
`"abc"[0]` は `*("abc" + 0)` に脱糖され、ポインタ算術 + デリファレンスとして処理される。

```c
"abc"[0]  // 97 ('a')
"abc"[2]  // 99 ('c')
"abc"[3]  // 0  (null terminator)
```

## テスト

ユニットテスト 22 件 + 統合テスト 208 件（7 件追加）= 230 件
