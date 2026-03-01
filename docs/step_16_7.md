# Step 16.7: バックスラッシュによる行継続

## 概要

プリプロセッサにバックスラッシュ+改行による行継続のサポートを追加します。これにより複数行にわたるマクロ定義が可能になります。PostgreSQLのヘッダでは広く使われている機能です。

## 問題

以下のような複数行マクロが結合されていませんでした。
```c
#define ADD(a, b) \
    ((a) + (b))
```

プリプロセッサは行単位で処理していたため、`#define ADD(a, b) \` は本体が `\` であるマクロとして定義され、次の行 `((a) + (b))` は独立したC文として扱われていました。

## 修正

メインのプリプロセッサループの前に、前処理パスとして `join_continuation_lines()` を追加しました。行末が `\` で終わっている場合、バックスラッシュを削除し、次の行を改行なしで結合します。

```rust
fn join_continuation_lines(source: &str) -> String {
    let mut result = String::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        if line.ends_with('\\') {
            result.push_str(&line[..line.len() - 1]);
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}
```

この処理はディレクティブの解析よりも前に実行されるため、`#define`、`#if` などすべてのディレクティブは既に結合された行を見ることになります。

## テストケース

```c
#define ADD(a, b) \
    ((a) + (b))
return ADD(10, 20);  // → 30
```
