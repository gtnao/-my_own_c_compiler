# Step 6.4: 文字列連結

## 概要

C言語の文字列連結を実装する。隣接する文字列リテラルは
自動的に1つの文字列に結合される。

```c
"hello" " " "world"  // "hello world" と同じ
"a" "b" "c"          // "abc" と同じ
```

## C言語の文字列連結規則

C89以降の仕様で、翻訳フェーズ6において隣接する文字列リテラルは
1つの文字列に連結される。これはコンパイル時に処理され、
実行時のオーバーヘッドはない。

主な用途：
- 長い文字列を複数行に分割
- マクロ展開との組み合わせ
- `#include` で挿入されたヘッダ内の文字列との結合

## 実装

パーサーの `primary()` で、最初の `Str` トークンを読んだ後、
続く `Str` トークンがあればバイト列を連結する。

```rust
TokenKind::Str(s) => {
    self.advance();
    // String concatenation: "hello" " " "world"
    let mut bytes = s;
    while let TokenKind::Str(next) = &self.current().kind {
        bytes.extend_from_slice(next);
        self.advance();
    }
    Expr::StrLit(bytes)
}
```

連結されたバイト列は単一の `StrLit` ノードになり、
コード生成では1つの文字列定数として `.rodata` に配置される。

### 例

`"hel" "lo"` の処理：
1. `Str([104, 101, 108])` を読む → `bytes = [104, 101, 108]`
2. 次が `Str([108, 111])` → `bytes = [104, 101, 108, 108, 111]`
3. 次が `Str` でない → 終了
4. `StrLit([104, 101, 108, 108, 111])` ノードを生成
5. コード生成: `.byte 104,101,108,108,111,0`

## テスト

ユニットテスト 22 件 + 統合テスト 228 件（5 件追加）= 250 件
