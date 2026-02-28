# Step 1.7: エラー報告の改善

## 概要

`error.rs` を新規作成し、ソース位置付きのエラーメッセージを表示できるようにする。lexer と parser のエラーメッセージが、ファイル名・行番号・列番号とソース行の該当位置を示すキャレット (`^`) 付きで表示されるようになる。

## エラー表示の例

入力ファイル `test.c`:
```
1 + @ + 2
```

エラー出力:
```
test.c:1:5: error: unexpected character '@'
1 + @ + 2
    ^
```

## 実装

### error.rs — ErrorReporter

```rust
pub struct ErrorReporter {
    source: String,     // source code
    filename: String,   // filename for error messages
}
```

#### error_at メソッド

バイト位置 (`pos`) を受け取り、行番号・列番号を計算してエラーメッセージを表示する:

```rust
pub fn error_at(&self, pos: usize, msg: &str) -> ! {
    let (line_num, col, line_str) = self.get_location(pos);
    eprintln!("{}:{}:{}: error: {}", self.filename, line_num, col + 1, msg);
    eprintln!("{}", line_str);
    eprintln!("{}^", " ".repeat(col));
    std::process::exit(1);
}
```

戻り値型 `-> !` は「この関数は決して戻らない」ことを示す (発散型)。`process::exit(1)` で終了するため。

#### get_location メソッド

バイト位置から行番号・列番号・その行の文字列を算出する:

```rust
fn get_location(&self, pos: usize) -> (usize, usize, &str) {
    let mut line_num = 1;
    let mut line_start = 0;

    for i in 0..pos.min(bytes.len()) {
        if bytes[i] == b'\n' {
            line_num += 1;
            line_start = i + 1;
        }
    }

    let col = pos - line_start;
    // ... find line_end ...
    (line_num, col, line_str)
}
```

### lexer.rs, parser.rs の変更

`ErrorReporter` への参照をライフタイム `'a` 付きで保持する:

```rust
pub struct Lexer<'a> {
    input: Vec<u8>,
    pos: usize,
    reporter: &'a ErrorReporter,
}
```

エラー時に `self.reporter.error_at(pos, msg)` を呼ぶ。

### main.rs の変更

```rust
let reporter = ErrorReporter::new(filename, source);
let mut lexer = Lexer::new(source, &reporter);
let tokens = lexer.tokenize();
let mut parser = Parser::new(tokens, &reporter);
let expr = parser.parse();
```

`ErrorReporter` は `main` 関数で生成し、lexer と parser に参照を渡す。

## Rustのライフタイム

`Lexer<'a>` の `'a` は「`reporter` の参照が少なくとも `Lexer` と同じだけ生きている」ことを保証する。これにより、`Lexer` が使われている間 `ErrorReporter` が解放されないことをコンパイル時に検証できる。

## テスト

### ユニットテスト

| テスト | 検証内容 |
|--------|---------|
| `test_get_location_single_line` | 単一行での位置計算 |
| `test_get_location_multi_line` | 複数行での行番号・列番号 |

### 統合テスト

既存の46テストが全てパスすることを確認 (リグレッションテスト)。

## Phase 1 完了

Step 1.7 を以て **Phase 1: 基礎** が完了。現在の状態:

- 整数リテラル、四則演算 (`+`,`-`,`*`,`/`,`%`)
- 比較演算子 (`==`,`!=`,`<`,`<=`,`>`,`>=`)
- 括弧、単項演算子 (`+`,`-`)
- ファイル入力、位置付きエラー報告
- モジュール構成: `token.rs`, `lexer.rs`, `ast.rs`, `parser.rs`, `codegen.rs`, `error.rs`

## 次のステップ

→ **Phase 2, Step 2.1: return文と式文** — `int main() { return 42; }` の形式をサポートする。
