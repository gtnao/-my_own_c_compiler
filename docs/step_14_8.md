# ステップ 14.8: `restrict` 型修飾子

## 概要

`restrict`、`__restrict`、`__restrict__` 型修飾子のサポートを追加する。これらは読み取られた後、無視される。コンパイラはエイリアス解析の最適化を行わない。

## なぜ必要か

PostgreSQL はパフォーマンスが重要なポインタパラメータにおいて、ポインタがエイリアスしないことを示すヒントとして `restrict`（または GCC 経由の `__restrict__`）を使用している:

```c
void memcpy(void * restrict dest, const void * restrict src, size_t n);
int * __restrict__ pg_ptr;
```

システムヘッダ（特に `<string.h>`、`<stdlib.h>`）でも `restrict` は広く使われている。

## 実装

### トークン

`TokenKind` に `Restrict` バリアントを追加した。

### 字句解析

3つの綴りを認識する:
- `restrict` — C99 標準キーワード
- `__restrict` — GCC 拡張
- `__restrict__` — GCC 拡張（二重アンダースコア形式）

### 構文解析

`restrict` は `const` および `volatile` と並ぶ型修飾子として扱われる。すべての修飾子スキップループにおいて読み取り後に無視される:

1. **`parse_type()`** — 基本型の前でスキップされる（型の前の修飾子）
2. **ポインタ修飾子** — `*` の後にスキップされる（例: `int * restrict p`）
3. **パラメータ修飾子** — 関数パラメータの型解析でスキップされる
4. **`is_type_keyword()`** — 型宣言の一部として認識される
5. **`stmt()`** — 変数宣言の開始として認識される

パーサー内の4つの修飾子スキップ箇所すべてに `Restrict` が含まれるようになった:
```rust
while matches!(self.current().kind,
    TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict | TokenKind::Alignas) {
    // ...
}
```

## 動作

- `restrict` はポインタのエイリアシングに関する C99 の最適化ヒントである
- 本コンパイラではこのヒントを無視する — エイリアス解析は行わない
- 修飾子は構文解析時に単に読み取られるだけである
- `int * restrict p` は本コンパイラでは `int *p` と等価である

## テストケース

```c
int main() { int a = 5; int * restrict p = &a; return *p; }      // → 5
int main() { int a = 7; int * __restrict p = &a; return *p; }    // → 7
int main() { int a = 9; int * __restrict__ p = &a; return *p; }  // → 9
```
