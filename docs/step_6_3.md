# Step 6.3: 文字リテラル

## 概要

シングルクォートで囲まれた文字リテラル `'a'` を実装する。
C言語では文字リテラルの型は `int` で、値はASCIIコードの整数値。

```c
'a'   // 97
'A'   // 65
'0'   // 48
'\n'  // 10
'\0'  // 0
```

## 実装

### レキサーでの処理

文字リテラルはレキサーの段階で整数値（`TokenKind::Num`）に変換される。
新しいトークン種別やASTノードは不要。

```rust
if ch == '\'' {
    let pos = self.pos;
    let val = self.read_char_literal();
    tokens.push(Token {
        kind: TokenKind::Num(val as i64),
        pos,
    });
    continue;
}
```

### read_char_literal

1. 開きクォート `'` をスキップ
2. 次の文字がバックスラッシュならエスケープ処理
3. そうでなければ文字のバイト値をそのまま取得
4. 閉じクォート `'` をスキップ

```rust
fn read_char_literal(&mut self) -> u8 {
    self.pos += 1; // skip opening '\''
    let val = if self.input[self.pos] == b'\\' {
        self.pos += 1;
        match self.input[self.pos] {
            b'n' => { self.pos += 1; b'\n' }
            b't' => { self.pos += 1; b'\t' }
            b'\\' => { self.pos += 1; b'\\' }
            b'\'' => { self.pos += 1; b'\'' }
            b'0' => { self.pos += 1; 0 }
            // ... hex, octal も対応
        }
    } else {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    };
    self.pos += 1; // skip closing '\''
    val
}
```

### なぜ文字リテラルの型は int なのか

C言語の仕様上、`'a'` の型は `char` ではなく `int`。
歴史的理由として、C言語の初期には `char` 型が引数として渡せなかったため、
文字リテラルは暗黙的に `int` に昇格していた。この仕様が現在も残っている。

ただし、実用上は `Num(97)` として整数定数に変換するため、
型の違いはコード生成に影響しない。

## テスト

ユニットテスト 22 件 + 統合テスト 223 件（6 件追加）= 245 件
