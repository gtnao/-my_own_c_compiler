# Step 10.6: `#` 演算子（文字列化）と `##` 演算子（トークン連結）

## 概要

関数形式マクロのボディ内で使える2つの特殊演算子を実装する：

- **`#` (stringize)**: マクロ引数を文字列リテラルに変換
- **`##` (token paste)**: 隣接する2つのトークンを1つに連結

```c
#define STR(x) #x
#define CONCAT(a,b) a##b

STR(hello)        // → "hello"
CONCAT(foo, bar)  // → foobar
```

## `#` 演算子（文字列化 / Stringize）

### 動作

マクロボディ中の `#param` を、引数の文字列リテラル化した結果に置換する：

```c
#define STR(x) #x
STR(hello)     // → "hello"
STR(3 + 4)     // → "3 + 4"
```

### 実装

`substitute_params()` 関数内で、ボディをスキャンする際に `#` を検出する：

```rust
if bytes[i] == b'#' && i + 1 < bytes.len() && bytes[i + 1] != b'#' {
    i += 1;
    // Skip whitespace after #
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    // Read identifier
    let start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    let ident = &body[start..i];
    if let Some(pos) = params.iter().position(|p| p == ident) {
        let arg = if pos < args.len() { &args[pos] } else { "" };
        result.push('"');
        // Escape special characters
        for ch in arg.chars() {
            if ch == '"' || ch == '\\' {
                result.push('\\');
            }
            result.push(ch);
        }
        result.push('"');
    }
}
```

### 特殊文字のエスケープ

引数に `"` や `\` が含まれる場合、文字列リテラルとして正しくなるようエスケープする：

```c
#define STR(x) #x
STR(a"b)  // → "a\"b"  (ダブルクォートをエスケープ)
STR(a\b)  // → "a\\b"  (バックスラッシュをエスケープ)
```

### `#` と `##` の区別

`#` が `##` の先頭と混同されないように、`bytes[i + 1] != b'#'` をチェックする：

```rust
if bytes[i] == b'#' && i + 1 < bytes.len() && bytes[i + 1] != b'#' {
    // # operator (stringize)
}
// ...
if bytes[i] == b'#' && i + 1 < bytes.len() && bytes[i + 1] == b'#' {
    // ## operator (token paste)
}
```

## `##` 演算子（トークン連結 / Token Paste）

### 動作

`##` の左右のトークンを連結して1つのトークンにする：

```c
#define CONCAT(a,b) a##b
CONCAT(foo, bar)    // → foobar

#define VAR(n) var##n
VAR(1)              // → var1
```

### 実装

`substitute_params()` で `##` を検出した際の処理：

1. **左側の空白を除去**: `result` の末尾の空白を取り除く
2. **`##` をスキップ**: 2文字分進める
3. **右側の空白をスキップ**: 次のトークンまでの空白を飛ばす
4. **右側のトークンを処理**: パラメータ名なら引数値に置換、そうでなければそのまま出力

```rust
if bytes[i] == b'#' && i + 1 < bytes.len() && bytes[i + 1] == b'#' {
    // Remove trailing whitespace from result
    while result.ends_with(' ') {
        result.pop();
    }
    i += 2;
    // Skip whitespace after ##
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    // Read next token (substitute if parameter)
    if i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let ident = &body[start..i];
        if let Some(pos) = params.iter().position(|p| p == ident) {
            if pos < args.len() {
                result.push_str(&args[pos]);
            }
        } else {
            result.push_str(ident);
        }
    }
}
```

### 空白の除去

`##` の左右の空白は結果に含めない。C標準では `##` は周囲の空白を「飲み込む」とされている：

```c
#define PASTE(a, b) a ## b
PASTE(x, y)  // → xy (not "x y")
```

実装では：
- **左側**: `result` の末尾の空白を `pop()` で除去
- **右側**: `##` の後の空白をスキップしてから次のトークンを読む

## 処理順序

`substitute_params()` での処理は以下の優先順位：

1. `#param` (stringize) — `#` の後に `#` が続かない場合
2. `a ## b` (token paste) — `##` を検出
3. 通常のパラメータ置換 — 識別子がパラメータ名に一致したら引数値で置換
4. その他の文字 — そのまま出力

## テストケース

```bash
# Stringize operator
assert_output 'hello' '#define STR(x) #x
int printf();
int main() { printf(STR(hello)); return 0; }'

# Token paste operator
assert 12 '#define CONCAT(a,b) a##b
int main() { int xy = 12; return CONCAT(x,y); }'

assert 42 '#define VAR(n) var##n
int main() { int var1 = 42; return VAR(1); }'
```
