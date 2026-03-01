# Step 16.2: defined() 対応の完全な #if 式評価器

## 概要

プリプロセッサの `#if` / `#elif` ディレクティブ用に、単純な `evaluate_simple_cond` 関数を完全な再帰下降式の式評価器に置き換えます。`defined()`、`&&`、`||`、`!`、比較演算子、算術演算、ビット演算、三項演算子、括弧式をサポートします。

## 問題

以前の実装では、以下のみを処理できていました。
- 式の先頭にある単純な `defined(NAME)`
- 文字列検索で見つけた単一の比較演算子

そのため、以下のような複合式では失敗していました。
```c
#if defined(FOO) && !defined(BAR)
#if X + 5 == 10
#if (A > 0) || (B > 0)
```

## 解決策: 再帰下降評価器

Cの演算子優先順位に従った完全な式パーサーを持つ `CondEval` 構造体を実装しました。

```
expr        = ternary
ternary     = logical_or ("?" expr ":" ternary)?
logical_or  = logical_and ("||" logical_and)*
logical_and = bitwise_or ("&&" bitwise_or)*
bitwise_or  = bitwise_xor ("|" bitwise_xor)*
bitwise_xor = bitwise_and ("^" bitwise_and)*
bitwise_and = equality ("&" equality)*
equality    = relational (("==" | "!=") relational)*
relational  = shift (("<" | ">" | "<=" | ">=") shift)*
shift       = add (("<<" | ">>") add)*
add         = mul (("+" | "-") mul)*
mul         = unary (("*" | "/" | "%") unary)*
unary       = "!" unary | "~" unary | "-" unary | "+" unary | primary
primary     = number | "(" expr ")" | "defined" ident | char_literal | ident
```

### 主な機能

- **`defined` 演算子**: `defined(NAME)` と `defined NAME` の両方の形式に対応
- **マクロ展開**: マクロである未知の識別子は、その値が再帰的に展開される
- **数値解析**: 10進数、16進数（`0x...`）に対応し、サフィックス（`U`、`L`、`UL`、`ULL`）はスキップ
- **文字リテラル**: `'A'` は65に評価される
- **未知の識別子**: 0に評価される（C標準の動作）
- **短絡評価**: `&&` と `||` は両辺を評価する（保守的な実装）

## テストケース

```c
#define FOO 1
#if defined(FOO) && !defined(BAR)
// Active — FOO is defined, BAR is not
#endif

#define X 5
#if X + 5 == 10
// Active — 5 + 5 == 10
#endif

#if 1 || 0
// Active — logical OR
#endif
```
