# Step 15.6〜15.8: 抽象宣言子、typeof、_Static_assert

## Step 15.6: 関数プロトタイプにおける抽象宣言子

関数プロトタイプでは、パラメータ名を省略できます（抽象宣言子）。

```c
// Prototype with no parameter names
int apply(int (*)(int, int), int, int);

// Definition with parameter names
int apply(int (*f)(int, int), int a, int b) { return f(a, b); }
```

### 実装

2つの変更が必要でした。

1. **匿名関数ポインタパラメータ**: パラメータ内で `(*` を解析する際、識別子が続かない場合は一意のダミー名を生成します。
```rust
let param_name = match &self.current().kind {
    TokenKind::Ident(s) => { ... }
    _ => {
        // Anonymous function pointer parameter
        self.unique_counter += 1;
        format!("__anon_fptr.{}", self.unique_counter)
    }
};
```

2. **匿名型付きパラメータ**: 型キーワードの後に識別子がなく、`,` または `)` が続く場合に対応します。
```rust
TokenKind::Comma | TokenKind::RParen => {
    // Abstract declarator: no parameter name
    self.unique_counter += 1;
    format!("__anon_param.{}", self.unique_counter)
}
```

## Step 15.7: typeof / __typeof__

既に完全に実装済みです。パーサーは `typeof(expr)` と `typeof(type)` の両方を処理します。

```c
int x = 42;
typeof(x) y = x;        // y is int
__typeof__(x) z = x;    // same thing (GCC spelling)
```

レキサーは `typeof`、`__typeof`、`__typeof__` のすべてを `TokenKind::Typeof` にマッピングしています。

## Step 15.8: _Static_assert

既に完全に実装済みです。`_Static_assert(expr, "message")` はトップレベルおよび関数本体の中で解析されます。

```c
_Static_assert(sizeof(int) == 4, "int must be 4 bytes");
```

式がコンパイル時にゼロと評価された場合、コンパイラは指定されたメッセージとともにエラーを出力します。
