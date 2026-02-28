# Step 10.8: `#error`, `#warning`, `#line`, `#pragma`

## 概要

プリプロセッサの残りのディレクティブを実装する：

| ディレクティブ | 動作 |
|---|---|
| `#error msg` | エラーメッセージを出力してコンパイルを中止 |
| `#warning msg` | 警告メッセージを出力（コンパイルは継続） |
| `#line N` | 行番号の変更（無視） |
| `#pragma ...` | コンパイラ固有の指示（無視） |

## `#error`

`#error` はプリプロセッサレベルでコンパイルを中止し、エラーメッセージを標準エラー出力に表示する。主に条件付きコンパイルと組み合わせて、サポートされない構成を検出するために使われる：

```c
#ifndef __linux__
#error "This compiler only supports Linux"
#endif
```

### 実装

```rust
} else if trimmed.starts_with("#error") {
    let msg = trimmed["#error".len()..].trim();
    eprintln!("{}:{}: error: {}", file_path, line_no + 1, msg);
    std::process::exit(1);
}
```

`std::process::exit(1)` でプロセスを即時終了する。エラーメッセージには GCC 互換のフォーマット `ファイル名:行番号: error: メッセージ` を使用する。

### 条件付きコンパイルとの組み合わせ

`#error` はスキップされた条件ブロック内では実行されない。条件ディレクティブのチェック後、スキップ判定の後に配置されているため、非アクティブ領域の `#error` は無視される。

## `#warning`

`#warning` は GNU 拡張のディレクティブで、警告を出力するがコンパイルは継続する：

```c
#warning "This feature is deprecated"
```

### 実装

```rust
} else if trimmed.starts_with("#warning") {
    let msg = trimmed["#warning".len()..].trim();
    eprintln!("{}:{}: warning: {}", file_path, line_no + 1, msg);
}
```

`#error` と異なり、`exit()` を呼ばない。

## `#line`

`#line` は行番号を変更するディレクティブで、通常はコード生成ツールが使用する：

```c
#line 100
// 以降の行番号が 100 から始まる
#line 200 "other.c"
// ファイル名も変更
```

### 実装

本コンパイラでは `#line` は無視する。`__LINE__` の行番号追跡は `preprocess_recursive()` 内の `enumerate()` による実際の行位置を使用する。

```rust
} else if trimmed.starts_with("#line") {
    // Ignored
}
```

## `#pragma`

`#pragma` はコンパイラ固有の指示を与えるディレクティブ。C標準では認識されない `#pragma` は無視してよいと定められている：

```c
#pragma once         // インクルードガード（GCC/Clang）
#pragma pack(push,1) // アライメント変更
#pragma GCC optimize("O2")
```

### 実装

本コンパイラでは全ての `#pragma` を無視する：

```rust
} else if trimmed.starts_with("#pragma") {
    // Ignored
}
```

`#pragma once` はインクルードガード機能の代替だが、本コンパイラでは `HashSet<PathBuf>` による重複インクルード防止が既に実装されているため、無視しても問題ない。

## テストケース

```bash
# #pragma is silently ignored
assert 42 '#pragma once
int main() { return 42; }'

# #warning outputs warning but continues compilation
assert 10 '#warning testing
int main() { return 10; }'
```

`#error` のテストはコンパイル失敗を期待するテストになるため、今回は含めない（テストフレームワークが正常終了のみをチェックするため）。
