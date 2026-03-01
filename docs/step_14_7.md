# Step 14.7: `_Noreturn`キーワード

## 概要

`_Noreturn`および`__noreturn__`関数指定子のサポートを追加します。これらは消費されて無視されます — コンパイラはnoreturn属性に基づいた特別な最適化や検証を行いません。

## なぜ必要か

PostgreSQLは、呼び出し元に戻ることのない`ereport(ERROR, ...)`や`ExceptionalCondition()`のような関数にアノテーションを付けるために`_Noreturn`（`pg_noreturn`マクロ経由）を使用しています。システムヘッダも`__attribute__`形式で`__noreturn__`を使用します。

```c
_Noreturn void ExceptionalCondition(const char *conditionName, ...);
__noreturn__ void abort(void);
```

## 実装

### トークン

`TokenKind`に`Noreturn`バリアントを追加しました。

### レキサー

2つの綴りを認識します:
- `_Noreturn` — C11標準キーワード
- `__noreturn__` — GCC拡張（アンダースコア2つの形式）

### パーサー

`_Noreturn`は`inline`と並んで関数指定子として処理されます — `parse_type()`内の型の前で消費・無視されます:

```rust
while matches!(self.current().kind, TokenKind::Inline | TokenKind::Noreturn) {
    self.advance();
}
```

`is_type_keyword()`および`stmt()`の型開始パターンに追加し、`_Noreturn`で始まる宣言が正しく認識されるようにしました。

## 動作

- `_Noreturn`は関数が戻らないことをコンパイラに示すヒント
- 本コンパイラはこのヒントを無視する — すべての関数は通常の戻りパスでコンパイルされる
- `_Noreturn`は最適化と警告にのみ影響するため、互換性としてはこれで十分

## テストケース

```c
_Noreturn void exit_fn() { return; } int main() { return 5; }   // → 5
__noreturn__ void die() { return; } int main() { return 3; }    // → 3
```
