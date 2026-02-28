# Step 1.3: トークナイザーの分離とスペース対応

## 概要

`main.rs` に直接書いていたトークナイズ処理を `token.rs` と `lexer.rs` に分離し、スペースを無視できるようにする。コンパイラのモジュール構成の基盤を作るステップ。

## 入出力

**入力**:
```
 12 + 34 - 5
```

**出力** (Step 1.2 と同じ形式):
```asm
  .globl main
main:
  mov $12, %rax
  add $34, %rax
  sub $5, %rax
  ret
```

## モジュール分割

Step 1.2 まで全てが `main.rs` にあったが、このステップで以下のように分割した:

```
src/
  main.rs      -- エントリポイント、トークン列の消費とコード生成
  token.rs     -- Token 構造体と TokenKind enum の定義
  lexer.rs     -- 字句解析器 (Lexer)
```

### なぜモジュール分割するのか

今後のステップで以下が追加される:

- `ast.rs` (AST定義) — Step 1.4
- `parser.rs` (構文解析) — Step 1.4
- `codegen.rs` (コード生成) — Step 1.4

責任を分離しておくことで、各モジュールを独立して変更・テストできる。

## 実装の詳細

### token.rs — トークンの定義

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Num(i64),   // integer literal
    Plus,       // +
    Minus,      // -
    Eof,        // end of input
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,   // what kind of token
    pub pos: usize,        // position in source (for error reporting)
}
```

- **`TokenKind`**: トークンの種類。`Num` は値を保持する。`Eof` は入力の終端を示す番兵 (sentinel)
- **`Token`**: 種類 + ソース中の位置。`pos` は将来のエラー報告に使用する
- `PartialEq` derive は `TokenKind` の比較 (`== Eof` 等) に必要

### lexer.rs — 字句解析器

```rust
pub struct Lexer {
    input: Vec<u8>,   // source as bytes
    pos: usize,       // current read position
}
```

#### tokenize メソッドの流れ

```
入力文字列 → [空白スキップ → 文字を判定 → Tokenを生成] × N → Eof を追加
```

1. **空白スキップ**: `is_ascii_whitespace()` が真の間、`pos` を進める
2. **数字**: `is_ascii_digit()` なら `read_number()` で連続する数字を読む
3. **演算子**: `+` なら `Plus`、`-` なら `Minus`
4. **終端**: ループ終了後に `Eof` トークンを追加

```rust
while self.pos < self.input.len() {
    let ch = self.input[self.pos] as char;

    if ch.is_ascii_whitespace() {
        self.pos += 1;     // skip whitespace
        continue;
    }

    if ch.is_ascii_digit() {
        let val = self.read_number();
        tokens.push(Token { kind: TokenKind::Num(val), pos });
        continue;
    }

    // operator (+, -)
    ...
}
tokens.push(Token { kind: TokenKind::Eof, pos: self.pos });
```

### main.rs — トークン列の消費

`main.rs` はトークン列をインデックスで走査し、アセンブリを生成する:

```rust
let tokens = lexer.tokenize();
let mut pos = 0;

let val = expect_number(&tokens, &mut pos);
println!("  mov ${}, %rax", val);

while tokens[pos].kind != TokenKind::Eof {
    match tokens[pos].kind {
        TokenKind::Plus => { ... println!("  add ${}, %rax", val); }
        TokenKind::Minus => { ... println!("  sub ${}, %rax", val); }
        _ => { error }
    }
}
```

`expect_number` ヘルパー関数は、現在のトークンが `Num` であることを確認して値を返す。

## ユニットテスト

`lexer.rs` 内に `#[cfg(test)]` モジュールを追加:

| テスト名 | 入力 | 検証内容 |
|----------|------|---------|
| `test_single_number` | `"42"` | `[Num(42), Eof]` |
| `test_addition` | `"1+2"` | `[Num(1), Plus, Num(2), Eof]` |
| `test_whitespace` | `" 12 + 34 - 5 "` | スペースが無視されること |

```bash
$ cargo test
running 3 tests
test lexer::tests::test_addition ... ok
test lexer::tests::test_single_number ... ok
test lexer::tests::test_whitespace ... ok
```

## 統合テスト

| 入力 | 期待値 | 目的 |
|------|--------|------|
| ` 12 + 34 - 5 ` | 41 | 前後・中間のスペース |
| ` 5 + 20 - 4 ` | 21 | Step 1.2の式にスペース追加 |
| `  10  ` | 10 | スペースのみの式 |

## このステップで学べること

1. **Rustのモジュールシステム**: `mod` 宣言、`use` によるインポート、`pub` の可視性
2. **字句解析 (lexical analysis)** の基本: 入力文字列 → トークン列
3. **番兵パターン**: `Eof` トークンで入力の終端を表現し、境界チェックを簡潔にする
4. **ユニットテスト**: `#[cfg(test)]` と `#[test]` アトリビュート

## 次のステップ

→ **Step 1.4: 乗除算、優先順位、括弧、単項演算子** — `ast.rs`, `parser.rs`, `codegen.rs` を新規作成し、再帰下降パーサーとスタックマシン方式のコード生成を導入する。
