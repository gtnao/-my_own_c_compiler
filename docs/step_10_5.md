# Step 10.5: 条件付きコンパイル (#ifdef / #ifndef / #if / #else / #elif / #endif)

## 概要

プリプロセッサに条件付きコンパイル機能を追加する。ソースコードの一部をマクロの定義状態や定数式の評価結果に基づいて包含/除外する。

```c
#define DEBUG

#ifdef DEBUG
int main() { return 1; }  // DEBUG が定義されていれば、こちらがコンパイルされる
#else
int main() { return 0; }  // そうでなければこちら
#endif
```

## ディレクティブ一覧

| ディレクティブ | 意味 |
|---|---|
| `#ifdef NAME` | NAME が `#define` されていれば真 |
| `#ifndef NAME` | NAME が `#define` されていなければ真 |
| `#if expr` | expr が非ゼロなら真 |
| `#elif expr` | 前の分岐が偽で expr が非ゼロなら真 |
| `#else` | 前の分岐がすべて偽なら真 |
| `#endif` | 条件付きブロックの終了 |

## 実装方法

### 条件スタック (`cond_stack`)

条件付きコンパイルはネストできるため、スタックで管理する：

```rust
let mut cond_stack: Vec<bool> = Vec::new();
```

各要素は「現在のネストレベルがアクティブ（コードを出力する）かどうか」を表す。

### ネストの仕組み

条件付きブロックがネストした場合、内側のブロックは外側がアクティブでない限りアクティブにならない。これを「親のアクティブ状態」と呼ぶ：

```c
#ifdef OUTER     // outer_active = macros.contains("OUTER")
  #ifdef INNER   // inner_active = outer_active && macros.contains("INNER")
    // ここに来るのは OUTER も INNER も定義されている場合のみ
  #endif
#endif
```

親のアクティブ状態は `cond_stack` の一つ前の要素から取得する：

```rust
let parent_active = if cond_stack.len() > 1 {
    cond_stack[cond_stack.len() - 2]
} else {
    true  // トップレベルは常にアクティブ
};
```

### `#ifdef` / `#ifndef` の処理

```rust
if trimmed.starts_with("#ifdef") {
    let name = trimmed["#ifdef".len()..].trim();
    let active = parent_active && macros.contains_key(name);
    cond_stack.push(active);
    continue;
}
```

- `#ifdef NAME`: マクロテーブルに NAME が存在するかチェック
- `#ifndef NAME`: 存在しないかチェック（`!macros.contains_key(name)`）
- どちらも親がアクティブでなければ、結果に関わらず false になる

### `#if` の処理

```rust
if trimmed.starts_with("#if ") {
    let cond_str = trimmed["#if".len()..].trim();
    let val = evaluate_simple_cond(cond_str, macros);
    let active = parent_active && val;
    cond_stack.push(active);
    continue;
}
```

条件式を `evaluate_simple_cond()` で評価する。

### `#elif` の処理

`#elif` は `#else` + `#if` を1つにまとめたもの。ロジックが複雑：

1. **前の分岐が真だった場合**: この分岐は必ず偽（一度真の分岐があったら、以降はスキップ）
2. **前の分岐が偽だった場合**: 条件式を評価し、親がアクティブかつ条件が真なら有効化

```rust
if trimmed.starts_with("#elif") {
    let cond_str = trimmed["#elif".len()..].trim();
    let len = cond_stack.len();
    if len > 0 {
        let current = cond_stack[len - 1];
        if current {
            // Previous branch was taken → skip this one
            cond_stack[len - 1] = false;
        } else {
            let parent_active = if len > 1 { cond_stack[len - 2] } else { true };
            let val = evaluate_simple_cond(cond_str, macros);
            cond_stack[len - 1] = parent_active && val;
        }
    }
    continue;
}
```

### `#else` の処理

```rust
if trimmed == "#else" {
    let len = cond_stack.len();
    if len > 0 {
        let current = cond_stack[len - 1];
        let parent_active = if len > 1 { cond_stack[len - 2] } else { true };
        cond_stack[len - 1] = parent_active && !current;
    }
    continue;
}
```

`#else` は前の分岐の真偽を反転させる。ただし親がアクティブでなければ常に偽。

**重要**: `parent_active && !current` とするのは、`#else` が単純な反転ではないから。例えば：

```c
#ifdef UNDEF     // current = false
  ...
#else            // parent_active(true) && !false → true
  ...            // ここが有効
#endif
```

### `#endif` の処理

```rust
if trimmed == "#endif" {
    cond_stack.pop();
    continue;
}
```

### スキップされた行の処理

条件スタックの最上位が `false` の場合、通常の行（`#include`, `#define`, 一般行）はすべてスキップされる。ただし、条件ディレクティブ自体（`#ifdef`, `#endif` 等）はスキップ中でも処理する必要がある。これはネストの追跡のため：

```rust
// Skip lines in inactive conditional regions
if cond_stack.last().copied().unwrap_or(true) == false {
    continue;
}
```

この処理は条件ディレクティブのチェック**後**に配置される。

### Rust の借用チェッカーへの対応

`#elif` と `#else` では `cond_stack` の最後の要素を変更しつつ、その一つ前の要素も読む必要がある。`last_mut()` を使うと可変借用が残り、インデックスアクセスが不可能になる：

```rust
// ❌ コンパイルエラー: last_mut() の可変借用が残っている
if let Some(top) = cond_stack.last_mut() {
    let parent = cond_stack[len - 2];  // immutable borrow while mutable borrow exists
    *top = parent && !*top;
}
```

解決策として、インデックスベースでアクセスする：

```rust
// ✅ OK: インデックスアクセスなら借用の衝突なし
let len = cond_stack.len();
let current = cond_stack[len - 1];
let parent = if len > 1 { cond_stack[len - 2] } else { true };
cond_stack[len - 1] = parent && !current;
```

## 条件式の評価 (`evaluate_simple_cond`)

`#if` と `#elif` で使われる条件式を評価する関数。完全なC式評価器ではなく、実用上十分な簡易版：

### サポートする構文

1. **整数リテラル**: `#if 1`, `#if 0`
2. **`defined(NAME)`**: `#if defined(FOO)`, `#if defined FOO`
3. **比較演算子**: `#if X == 1`, `#if X != 0`, `#if X > 5`, etc.
4. **マクロ展開**: 条件式中のマクロは展開された上で評価

```rust
fn evaluate_simple_cond(cond: &str, macros: &HashMap<String, MacroDef>) -> bool {
    let expanded = expand_macros(cond, macros);
    let trimmed = expanded.trim();

    // Handle defined(NAME)
    if trimmed.starts_with("defined") { ... }

    // Handle comparisons: ==, !=, >=, <=, >, <
    for (op, f) in &[("==", ...), ("!=", ...), ...] {
        if let Some(pos) = trimmed.find(op) { ... }
    }

    // Simple integer: non-zero is true
    trimmed.parse::<i64>().unwrap_or(0) != 0
}
```

### マクロ展開の重要性

`#if X == 2` のような条件では、まず `X` をマクロ展開してから評価する。展開後に `2 == 2` のような文字列になり、それを比較演算子で分割して評価する。

未定義のマクロ名は展開されずに残るが、`parse::<i64>()` で `0` にフォールバックするため、結果的に未定義マクロは `0` として扱われる（C標準の動作に近い）。

### 比較演算子の順序

`>=` と `<=` は `>` と `<` より先にチェックする必要がある。`find(">")` が `>=` の `>` にもマッチしてしまうため：

```rust
// ✅ 正しい順序
(">=", ...), ("<=", ...), (">", ...), ("<", ...)

// ❌ 間違い: "X >= 5" で ">" が先にマッチし、"X " と "= 5" に分割される
(">", ...), ("<", ...), (">=", ...), ("<=", ...)
```

同様に `==` と `!=` も先にチェックする。

## インクルードガードとの組み合わせ

`#ifndef` は標準的なインクルードガードで使われる：

```c
// header.h
#ifndef HEADER_H
#define HEADER_H

int add(int a, int b) { return a + b; }

#endif
```

このパターンにより、同じヘッダが複数回インクルードされても、2回目以降は `HEADER_H` が定義済みのため内容がスキップされる。

## テストケース

```bash
# #ifdef: defined macro → active branch
assert 1 '#define FOO
#ifdef FOO
int main() { return 1; }
#else
int main() { return 2; }
#endif'

# #ifdef: undefined macro → else branch
assert 2 '#ifdef FOO
int main() { return 1; }
#else
int main() { return 2; }
#endif'

# #ifndef: defined macro → else branch
assert 10 '#define X 10
#ifndef X
int main() { return 0; }
#else
int main() { return X; }
#endif'

# #ifndef: undefined macro → active branch
assert 5 '#ifndef UNDEF
int main() { return 5; }
#else
int main() { return 0; }
#endif'

# #if 1 → active, #if 0 → else
assert 1 '#if 1
int main() { return 1; }
#else
int main() { return 0; }
#endif'

assert 0 '#if 0
int main() { return 1; }
#else
int main() { return 0; }
#endif'

# #elif with comparison
assert 20 '#define X 2
#if X == 1
int main() { return 10; }
#elif X == 2
int main() { return 20; }
#else
int main() { return 30; }
#endif'
```
