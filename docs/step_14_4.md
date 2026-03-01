# Step 14.4: inlineおよびstatic inline関数

## 概要

`inline`、`static inline`、`__inline`、`__inline__`関数修飾子のサポートを追加します。これらは消費されて無視されます — 関数は常に通常通りコンパイルされ、コンパイラレベルでは実際にインライン化されることはありません。

## なぜ必要か

PostgreSQLやシステムヘッダでは、ヘッダ内の小さなユーティリティ関数に`static inline`が広く使用されています:

```c
static inline int Max(int a, int b) { return a > b ? a : b; }
static inline void *palloc(size_t size) { ... }
```

GCCも組み込みヘッダで`__inline`や`__inline__`の変形を使用します。

## 実装

### トークン

`Inline`トークン種別を追加しました。

### レキサー

3つの綴りを認識します:
- `inline` — C99標準
- `__inline` — GCC拡張
- `__inline__` — GCC拡張（アンダースコア2つの形式）

### パーサー

`inline`は型修飾子として扱われ、以下の箇所で消費・無視されます:

1. **`parse_type()`** — ベース型の前で`__attribute__`と並んでスキップ
2. **`is_type_keyword()`** — 型宣言の一部として認識
3. **`stmt()`** — 変数/関数宣言の開始として認識
4. **トップレベルの`parse()`** — トップレベルの`static`が消費された後、`inline`は`parse_type()`で処理され、関数は通常通りパースされる

### トップレベルの`static`

以前は、トップレベルの`static`はローカル静的変数に対してのみ処理されていました。今回、トップレベルの`static`キーワードは単純に消費されるようになり、`static inline int foo()`や`static int x`が正しく動作します — 非staticの対応するものと同じように扱われます。

## 動作

- `inline`はコンパイラへのヒントであり、要件ではない
- 本コンパイラは実際に関数をインライン化しない — すべての関数は個別のシンボルとしてコンパイルされる
- `static inline`関数はグローバルシンボルとして出力される（真のstatic/ローカルリンケージではない）
- これは互換性に必要な最小限の動作と一致する

## テストケース

```c
static inline int add(int a, int b) { return a + b; }
int main() { return add(2, 4); }  // → 6

inline int dbl(int x) { return x * 2; }
int main() { return dbl(5); }  // → 10
```
