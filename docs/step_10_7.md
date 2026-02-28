# Step 10.7: 事前定義マクロ (`__FILE__`, `__LINE__`)

## 概要

C標準で定義されている事前定義マクロのうち、`__FILE__` と `__LINE__` を実装する。これらはプリプロセッサが自動的に展開する特殊なマクロで、デバッグやエラー報告に広く使われる。

```c
int main() {
    return __LINE__;  // → 2 (この行の行番号)
}
```

## 事前定義マクロ一覧

| マクロ | 展開結果 | 型 |
|---|---|---|
| `__FILE__` | 現在のソースファイル名 | 文字列リテラル `"filename.c"` |
| `__LINE__` | 現在の行番号（1始まり） | 整数リテラル |

### `__func__` について

C99で追加された `__func__` は、現在の関数名を表す事前定義識別子だが、厳密にはプリプロセッサマクロではなくコンパイラ組み込みの識別子である。プリプロセッサの段階では関数スコープが不明なため、本ステップでは実装しない。

## 実装方法

### 行番号の追跡

`preprocess_recursive()` のメインループで `enumerate()` を使い、行番号を追跡する：

```rust
for (line_no, line) in source.lines().enumerate() {
    // line_no は 0 始まり
    // ...
    let with_predefined = replace_predefined(line, file_path, line_no + 1);
    // ...
}
```

`line_no + 1` とすることで、C標準の「1始まりの行番号」に合わせる。

### `replace_predefined()` 関数

通常行の処理で、マクロ展開(`expand_macros`)の前に事前定義マクロの置換を行う：

```rust
fn replace_predefined(line: &str, file_path: &str, line_no: usize) -> String {
    // Scan the line character by character
    // Skip string/char literals (don't replace inside them)
    // When encountering __FILE__, replace with "filename"
    // When encountering __LINE__, replace with line number
}
```

### 処理の順序

```
ソース行 → replace_predefined() → expand_macros() → 結果
```

事前定義マクロの置換を先に行うことで、ユーザー定義マクロと競合しない。また、文字列リテラル内では置換しない（通常のマクロ展開と同じルール）。

### 文字列/文字リテラルのスキップ

`__FILE__` や `__LINE__` が文字列リテラル内に出現した場合は置換しない：

```c
printf("__LINE__");  // → そのまま "__LINE__" が出力される
int x = __LINE__;    // → int x = 5; (行番号に置換)
```

`replace_predefined()` 内で、`"..."` と `'...'` 内の文字はスキップする：

```rust
if bytes[i] == b'"' {
    // Copy string literal verbatim
    result.push('"');
    i += 1;
    while i < bytes.len() && bytes[i] != b'"' {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            result.push(bytes[i] as char);
            i += 1;
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    if i < bytes.len() {
        result.push('"');
        i += 1;
    }
}
```

### `__FILE__` の展開

`__FILE__` は文字列リテラルに展開される。ファイルパスをダブルクォートで囲む：

```rust
"__FILE__" => {
    result.push('"');
    result.push_str(file_path);
    result.push('"');
}
```

例: `__FILE__` → `"test.c"`

### `__LINE__` の展開

`__LINE__` は整数リテラルに展開される：

```rust
"__LINE__" => {
    result.push_str(&line_no.to_string());
}
```

例: `__LINE__` → `3`

## テストケース

```bash
# __LINE__ on first line
assert 1 'int main() { return __LINE__; }'

# __LINE__ on second line
assert 2 'int x;
int main() { return __LINE__; }'
```

## 注意点

### インクルードファイル内の行番号

`#include` されたファイル内で `__LINE__` を使った場合、そのファイル内での行番号が返される（メインファイルでの行番号ではない）。`preprocess_recursive()` がファイルごとに呼ばれ、各ファイルで独立した行番号カウンターを持つため、自然にこの動作になる。

### `__FILE__` のパス形式

`__FILE__` はコンパイラに渡されたファイルパスをそのまま返す。相対パスで渡された場合は相対パス、絶対パスで渡された場合は絶対パスになる。
