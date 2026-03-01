# Step 16.1: 可変引数マクロ（__VA_ARGS__）

## 概要

`...` と `__VA_ARGS__` を使った可変引数（variadic）マクロのサポートを追加します。これは、PostgreSQLの `elog()`、`ereport()` をはじめとする多くのロギング/エラー報告マクロに不可欠です。

## 仕組み

### マクロ定義

```c
#define LOG(fmt, ...) printf(fmt, __VA_ARGS__)
```

パラメータリスト内の `...` は、マクロが可変個の引数を受け取ることを示します。マクロ本体では、`__VA_ARGS__` が名前付きパラメータを超えて渡されたすべての追加引数に展開されます。

### 実装

**MacroDef** にマクロが可変引数かどうかを追跡するフィールドを追加しました。

```rust
enum MacroDef {
    Object(String),
    Function(Vec<String>, String, bool),  // params, body, is_variadic
}
```

**解析**: `#define` を解析する際、最後のパラメータが `...` であれば、それをパラメータリストから取り除き、`is_variadic` を `true` に設定します。

**展開**: 可変引数マクロの呼び出しを展開する際の動作は以下の通りです。
1. 名前付きパラメータは通常通り対応する引数にマッチします
2. 名前付きパラメータを超える余分な引数は `, ` で結合され、本体中の `__VA_ARGS__` に置換されます

```rust
if is_variadic {
    let va_args = if args.len() > params.len() {
        args[params.len()..].join(", ")
    } else {
        String::new()
    };
    subst_params.push("__VA_ARGS__".to_string());
    subst_args.push(va_args);
}
```

## 使用例

```c
// Basic variadic macro
#define LOG(fmt, ...) printf(fmt, __VA_ARGS__)
LOG("x=%d y=%d\n", 10, 20);
// Expands to: printf("x=%d y=%d\n", 10, 20);

// Macro that passes variadic args to another function
#define CALL(fn, ...) fn(__VA_ARGS__)
CALL(add, 20, 22);
// Expands to: add(20, 22);
```

## テストケース

```c
#define FIRST(a, ...) a
return FIRST(3, 4, 5);  // → 3

#define CALL(fn, ...) fn(__VA_ARGS__)
return CALL(add, 20, 22);  // → 42
```
