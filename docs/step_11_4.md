# ステップ 11.4: コールバックパターン

## 概要

このステップでは、関数ポインタのサポートを拡張し、関数パラメータとして使えるようにすることで、コールバックパターンを実現する。コールバックは C プログラミングの基本要素であり、`qsort`、シグナルハンドラ、イベント駆動システム、あらゆる形式の高階プログラミングを支えている。

## コールバックパターン

コールバックとは、別の関数に引数として渡される関数であり、渡された先の関数から呼び出される:

```c
int apply(int (*f)(int), int x) {
    return f(x);  // call the passed function
}

int double(int x) { return x * 2; }

int main() {
    return apply(double, 5);  // => 10
}
```

重要なポイント: `f` は `Ptr(Void)` 型（8バイトポインタ）の単なるローカル変数にすぎない。`f(x)` として呼び出されると、間接呼び出し `call *%r10` が生成される。

## パーサーの変更

### 関数ポインタパラメータ

`function_or_prototype()` のパラメータパースを拡張し、パラメータ位置での関数ポインタ構文を認識するようにした:

```
parameter = type ident                     // normal parameter
          | type "(" "*" ident ")" "(" param_types ")"  // function pointer parameter
          | type ident "[" "]"             // array parameter
```

`parse_type()` が基本型を返した後、パーサーは `(` `*` をチェックする:

```rust
if current == '(' && next == '*' {
    // Parse function pointer parameter
    // Consume: ( * name ) ( param_types )
    // Type becomes Ptr(Void)
}
```

パラメータ型リスト `(int, int)` はパースされるが、型情報は保存されない。関数ポインタは単純に `Ptr(Void)` として型付けされる。

### 呼び出し規約

関数内で `f` がパラメータである場合に `f(x)` が呼ばれると:
1. パーサーは `f` が宣言済み変数であることを認識 → `FuncPtrCall` を生成
2. コード生成が `f` をスタックスロットからロード
3. ポインタを `%r10` に保存
4. 引数をレジスタにセットアップ
5. `call *%r10` を実行

## 使用例: コールバックによる map_sum

```c
int map_sum(int *a, int n, int (*f)(int)) {
    int s = 0;
    int i;
    for (i = 0; i < n; i++)
        s += f(a[i]);  // apply callback to each element
    return s;
}

int dbl(int x) { return x * 2; }

int main() {
    int a[3] = {1, 2, 3};
    return map_sum(a, 3, dbl);  // => 2+4+6 = 12
}
```

### `f(a[i])` の生成アセンブリ:

```asm
  # Load f (function pointer from parameter)
  mov -24(%rbp), %rax     # f is 3rd parameter
  mov %rax, %r10          # save to %r10

  # Evaluate a[i] (argument for callback)
  # ... array indexing code ...
  # Result in %rax

  push %rax
  pop %rdi                # arg goes to %rdi
  mov $0, %al
  call *%r10              # indirect call through f
```

## 呼び出しフロー

```
main()
  │
  ├─ Evaluates &dbl → lea dbl(%rip), %rax
  ├─ Passes as 3rd argument in %rdx
  │
  └─ call map_sum
       │
       ├─ map_sum receives f in %rdx → stores to stack
       │
       └─ Loop body:
            ├─ Loads f from stack → %r10
            ├─ Evaluates a[i] → %rdi
            └─ call *%r10  ─────────→  dbl(a[i])
                                        │
                                        └─ returns x*2
```

## テストケース

```c
// Basic callback
int apply(int (*f)(int), int x) { return f(x); }
int dbl(int x) { return x * 2; }
int main() { return apply(dbl, 5); }  // => 10

// Callback with different function
int apply(int (*f)(int), int x) { return f(x); }
int sq(int x) { return x * x; }
int main() { return apply(sq, 5); }   // => 25

// Array processing with callback (map + reduce)
int map_sum(int *a, int n, int (*f)(int)) {
    int s = 0; int i;
    for (i = 0; i < n; i++) s += f(a[i]);
    return s;
}
int dbl(int x) { return x * 2; }
int main() {
    int a[3] = {1, 2, 3};
    return map_sum(a, 3, dbl);  // => 12
}
```

## フェーズ 11 完了

このステップで、フェーズ 11（標準ライブラリ互換性）が完了した:

| ステップ | 機能 | 説明 |
|----------|------|------|
| 11.1 | printf | libc リンクによる外部関数呼び出し |
| 11.2 | 可変長引数 | `va_list`、`va_start`、`va_arg` とレジスタ保存領域 |
| 11.3 | 関数ポインタ | 宣言、関数からポインタへの暗黙変換、`call *%r10` |
| 11.4 | コールバック | パラメータとしての関数ポインタ、高階パターンの実現 |
