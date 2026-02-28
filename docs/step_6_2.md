# Step 6.2: エスケープシーケンス完全対応

## 概要

文字列リテラルのエスケープシーケンスを完全にサポートする。
Step 6.1の基本的な `\n`, `\t`, `\\`, `\"`, `\0` に加え、
8進エスケープ `\NNN`、16進エスケープ `\xNN`、
その他の制御文字エスケープを追加する。

## サポートするエスケープシーケンス

| エスケープ | 値 (10進) | 意味 |
|-----------|----------|------|
| `\n` | 10 | 改行 (Line Feed) |
| `\t` | 9 | 水平タブ |
| `\r` | 13 | キャリッジリターン |
| `\a` | 7 | ベル (Alert) |
| `\b` | 8 | バックスペース |
| `\f` | 12 | フォームフィード |
| `\v` | 11 | 垂直タブ |
| `\\` | 92 | バックスラッシュ |
| `\'` | 39 | シングルクォート |
| `\"` | 34 | ダブルクォート |
| `\?` | 63 | クエスチョンマーク |
| `\NNN` | 0-255 | 8進エスケープ (1-3桁) |
| `\xNN` | 0-255 | 16進エスケープ |

## 8進エスケープ

`\0`, `\101`, `\377` のように、バックスラッシュの後に
1〜3桁の8進数（0-7）を記述する。

```c
"\101"[0]  // 65 = 0101 (octal) = 'A'
"\0"[0]    // 0  = null
"\177"[0]  // 127 = DEL
```

### パース処理

```rust
b'0'..=b'7' => {
    let mut val = (ch - b'0') as u32;
    self.pos += 1;
    for _ in 0..2 {  // read up to 2 more octal digits
        if self.pos < self.input.len() {
            let d = self.input[self.pos];
            if d >= b'0' && d <= b'7' {
                val = val * 8 + (d - b'0') as u32;
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    s.push(val as u8);
}
```

最初の桁は必ず消費し、続く2桁はオプション。
`\0` は1桁（値0）、`\101` は3桁（値65）になる。

## 16進エスケープ

`\x41`, `\xff` のように、`\x` の後に16進数を記述する。

```c
"\x41"[0]        // 65 = 0x41 = 'A'
"\xff"[0] & 255  // 255 = 0xFF
```

### パース処理

```rust
b'x' => {
    self.pos += 1;
    let mut val = 0u32;
    while self.pos < self.input.len() {
        let d = self.input[self.pos] as char;
        if d.is_ascii_hexdigit() {
            val = val * 16 + d.to_digit(16).unwrap();
            self.pos += 1;
        } else {
            break;
        }
    }
    s.push(val as u8);
}
```

## String → Vec<u8> への変更

### 問題

Rust の `String` は UTF-8 エンコーディング。`\xff` (0xFF = 255) を
Rust の char として格納すると U+00FF（ÿ）になり、
UTF-8 では2バイト `0xC3 0xBF` にエンコードされる。
これではバイト列として `.byte 255` ではなく `.byte 195,191` が出力されてしまう。

### 解決

文字列リテラルの内部表現を `String` から `Vec<u8>` に変更。
C言語の文字列はバイト列であり、UTF-8 の制約を受けない。

```rust
// token.rs
pub enum TokenKind {
    Str(Vec<u8>),  // was: Str(String)
}

// ast.rs
pub enum Expr {
    StrLit(Vec<u8>),  // was: StrLit(String)
}

// codegen.rs
string_literals: Vec<Vec<u8>>,  // was: Vec<String>
```

## テスト

ユニットテスト 22 件 + 統合テスト 217 件（6 件追加）= 239 件
