# Step 10.4: #define（関数形式マクロ）

## 概要

関数形式マクロ `#define NAME(params) body` を実装する。パラメータを受け取り、展開時に引数で置換するマクロ。

```c
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define SQ(x) ((x) * (x))

int main() {
    return MAX(3, 7);  // → ((3) > (7) ? (3) : (7)) → 7
    return SQ(3);       // → ((3) * (3)) → 9
}
```

## オブジェクト形式マクロとの区別

マクロ名の直後に `(` が来る（スペースなし）場合は関数形式マクロとして解釈する：

```c
#define FOO(x) x+1   // 関数形式: FOO は (x) のパラメータを取る
#define BAR (x)       // オブジェクト形式: BAR は " (x)" に展開される
```

## 実装方法

### マクロ定義の型

```rust
enum MacroDef {
    Object(String),                    // #define NAME value
    Function(Vec<String>, String),     // #define NAME(params) body
}
```

### パース

```rust
if after_name.starts_with('(') {
    // Function-like macro
    let params = parse_param_list(); // "a, b" → ["a", "b"]
    let body = rest_after_paren;
    macros.insert(name, MacroDef::Function(params, body));
} else {
    // Object-like macro
    macros.insert(name, MacroDef::Object(value));
}
```

### 展開

関数形式マクロの展開は3つのステップ：

1. **引数パース**: マクロ呼び出し `MAX(3, 7)` から引数 `["3", "7"]` を抽出
2. **パラメータ置換**: マクロボディ内の `a` → `3`、`b` → `7` に置換
3. **再帰展開**: 置換結果を再度マクロ展開（ネストしたマクロ対応）

#### 引数パース（`parse_macro_args`）

```rust
fn parse_macro_args(input: &str) -> Vec<String> {
    // Comma-separated, respecting nested parentheses
    // MAX(ADD(1,2), 3) → ["ADD(1,2)", "3"]
}
```

ネストした括弧をカウントし、トップレベルのカンマでのみ分割する。

#### パラメータ置換（`substitute_params`）

```rust
fn substitute_params(body: &str, params: &[String], args: &[String]) -> String {
    // Scan for identifiers matching param names
    // Replace with corresponding argument values
}
```

識別子ベースの置換なので、`ax` のような長い識別子のパラメータ `a` 部分だけを誤って置換しない。

## テストケース

```bash
# MAX macro
assert 7 '#define MAX(a,b) ((a)>(b)?(a):(b))
int main() { return MAX(3, 7); }'

# ADD macro
assert 15 '#define ADD(x,y) ((x)+(y))
int main() { return ADD(7, 8); }'

# SQ (square) macro
assert 9 '#define SQ(x) ((x)*(x))
int main() { return SQ(3); }'
```
