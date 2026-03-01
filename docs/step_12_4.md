# ステップ 12.4: const 修飾子

## 概要

`const` 修飾子は、変数の値が初期化後に変更されるべきでないことを示す。本コンパイラでは、`const` はパースされ認識されるが強制はされない。コード生成には影響しない。

## 実装

`const` は型修飾子として扱われ、型パース時に単に消費されて無視される:

```rust
// At the start of parse_type()
while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile) {
    self.advance();
}
```

ポインタのアスタリスクの後にも出現しうる:
```c
int *const p;   // const pointer to int
const int *p;   // pointer to const int
```

両方の位置が処理される:
```rust
// After each * in pointer parsing
while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile) {
    self.advance();
}
```

## 強制しない理由

実際の C コンパイラでは `const` は以下の目的で使用される:
1. コンパイル時のエラーチェック（const 変数への代入検出）
2. 最適化（const グローバルを読み取り専用セクションに配置）

本コンパイラで const の正当性を強制しない理由:
- 型システムで const 性を追跡する必要がある
- 主な目標は正しいコード生成であり、エラーチェックではない
- コード生成を変更せずに後から強制を追加できる

## テストケース

```c
const int a = 42; return a;          // => 42
const int *p; int a = 3; p = &a; return *p;  // => 3
```
