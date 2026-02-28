# Step 10.3: #define（オブジェクト形式マクロ）

## 概要

`#define NAME value` によるオブジェクト形式マクロ（関数形式ではない）を実装する。ソースコード中の識別子 `NAME` が `value` に置換される。

```c
#define MAX 100
#define PI_APPROX 3

int main() {
    int a = MAX;     // → int a = 100;
    return PI_APPROX; // → return 3;
}
```

## 実装方法

### プリプロセッサの拡張

`preprocess.rs` に `#define` と `#undef` の処理を追加。マクロは `HashMap<String, String>` で管理。

```rust
fn preprocess_recursive(
    source: &str,
    file_path: &str,
    included: &mut HashSet<PathBuf>,
    macros: &mut HashMap<String, String>,
) -> String {
    for line in source.lines() {
        if trimmed.starts_with("#define") {
            // Parse: #define NAME value
            let mut parts = rest.splitn(2, whitespace);
            let name = parts.next().unwrap();
            let value = parts.next().unwrap_or("").trim();
            macros.insert(name, value);
        } else if trimmed.starts_with("#undef") {
            macros.remove(name);
        } else {
            // Expand macros in this line
            let expanded = expand_macros(line, &macros);
            result.push_str(&expanded);
        }
    }
}
```

### マクロ展開

`expand_macros()` 関数は行内の識別子をスキャンし、マクロ名に一致するものを置換する：

1. 文字列リテラル `"..."` 内はスキップ（マクロ展開しない）
2. 文字リテラル `'...'` 内もスキップ
3. 識別子を検出したらマクロテーブルを参照
4. 一致したら値に置換
5. 置換後の値も再帰的に展開（連鎖マクロ対応）

```rust
fn expand_macros(line: &str, macros: &HashMap<String, String>) -> String {
    // Scan character by character
    // Skip string/char literals
    // Extract identifiers [a-zA-Z_][a-zA-Z0-9_]*
    // Check against macro table
    // Replace with value (recursively expanded)
}
```

### 文字列内のマクロ展開防止

```c
#define X 5
printf("X is %d", X);
```

→ `"X is %d"` の中の `X` は展開しない。`printf` の引数の `X` だけ `5` に展開。

### `#undef`

```c
#define X 5
int a = X;  // → int a = 5;
#undef X
int b = X;  // → error: undefined variable X
```

## テストケース

```bash
assert 42 '#define N 42
int main() { return N; }'

assert 10 '#define X 3
#define Y 7
int main() { return X + Y; }'

assert 5 '#define VAL 5
int main() { int a = VAL; return a; }'
```
