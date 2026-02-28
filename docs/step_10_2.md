# Step 10.2: #include ディレクティブ

## 概要

`#include "file"` と `#include <file>` のプリプロセッサディレクティブを実装する。インクルードされたファイルの内容が、ディレクティブの位置に展開される。

```c
// add.h
int add(int a, int b) { return a + b; }

// main.c
#include "add.h"
int main() { return add(3, 4); }
```

## 実装方法

### プリプロセッサモジュール（`preprocess.rs`）

コンパイルパイプラインの最初のステップとして、ソースのテキストレベルでの前処理を行う。レクサーに渡す前にインクルードを解決する。

```
ソースファイル → preprocess() → 展開済みソース → Lexer → Parser → Codegen
```

### アルゴリズム

1. ソースを行単位で走査
2. `#include "file"` または `#include <file>` を検出
3. ファイルパスを解決（現在のファイルのディレクトリを基準）
4. ファイル内容を読み込み
5. 読み込んだ内容を再帰的にプリプロセス（ネストしたインクルード対応）
6. ディレクティブ行を展開後の内容で置換

```rust
pub fn preprocess(source: &str, file_path: &str) -> String {
    let mut included = HashSet::new();
    preprocess_recursive(source, file_path, &mut included)
}

fn preprocess_recursive(source: &str, file_path: &str, included: &mut HashSet<PathBuf>) -> String {
    let dir = Path::new(file_path).parent().unwrap_or(Path::new("."));

    for line in source.lines() {
        if line.trim().starts_with("#include") {
            // Parse file path from "file" or <file>
            // Resolve relative to current file's directory
            // Read and recursively preprocess
        } else {
            // Pass through unchanged
        }
    }
}
```

### インクルードガード

同じファイルの重複インクルードを防ぐため、`HashSet<PathBuf>` でインクルード済みファイルを追跡。`canonicalize()` でパスを正規化し、シンボリックリンクや相対パスの違いによる重複を防ぐ。

### `#include "file"` vs `#include <file>`

- `#include "file"`: カレントファイルのディレクトリから検索
- `#include <file>`: システムインクルードパスから検索（現在は同じ動作）

システムヘッダの完全なサポートはこのコンパイラのスコープ外だが、構文はパースできる。

### パスの解決

```rust
let resolved = dir.join(&include_path);
if let Ok(contents) = std::fs::read_to_string(&resolved) {
    // Process and include
}
// If file not found, silently skip (for system headers)
```

## main.rs の変更

プリプロセスをパイプラインの最初に追加：

```rust
let input = fs::read_to_string(filename)?;
let preprocessed = preprocess(&input, filename);  // NEW
let source = preprocessed.trim();
let reporter = ErrorReporter::new(filename, source);
let mut lexer = Lexer::new(source, &reporter);
// ...
```

## テストケース

```bash
# Create test header and source
echo 'int add(int a, int b) { return a + b; }' > add.h
echo '#include "add.h"
int main() { return add(3, 4); }' > main.c
# Compile and verify: should return 7
```
