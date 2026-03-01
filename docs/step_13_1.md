# Step 13.1: 定数畳み込み

## 概要

定数畳み込み（Constant Folding）は、定数式を実行時にコードを生成して計算するのではなく、コンパイル時に評価するコンパイル時最適化です。

最適化前:
```asm
  mov $3, %rax    # load 3
  push %rax
  mov $2, %rax    # load 2
  pop %rdi
  add %rdi, %rax  # compute 2+3 at runtime
```

最適化後:
```asm
  mov $5, %rax    # result computed at compile time
```

## 実装

`Expr::BinOp`ノードを直接構築する代わりに、ヘルパーメソッド`make_binop`を使用します。両方のオペランドが`Expr::Num`の場合、結果はコンパイル時に計算され`Expr::Num`として返されます:

```rust
fn make_binop(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
    if let (Expr::Num(l), Expr::Num(r)) = (&lhs, &rhs) {
        let result = match op {
            BinOp::Add => l.wrapping_add(*r),
            BinOp::Sub => l.wrapping_sub(*r),
            BinOp::Mul => l.wrapping_mul(*r),
            // ... all operators ...
            _ => return Expr::BinOp { ... };
        };
        return Expr::Num(result);
    }
    Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }
}
```

## 対応する演算

すべての二項演算が畳み込み対象です:
- 算術演算: `+`, `-`, `*`, `/`, `%`
- 比較演算: `==`, `!=`, `<`, `<=`, `>`, `>=`
- ビット演算: `&`, `|`, `^`, `<<`, `>>`

ゼロ除算およびゼロでの剰余演算は畳み込みを行いません（実行時にクラッシュするコードを生成しますが、これはGCCの定数ゼロ除算に対する動作と一致しています）。

## 連鎖的な畳み込み

畳み込みは構文解析中（再帰下降パーサー内）に行われるため、自然に連鎖します:

```c
return 1 + 2 + 3;
```

1. `1 + 2`をパース → `make_binop(Add, Num(1), Num(2))` → `Num(3)`
2. `Num(3) + 3`をパース → `make_binop(Add, Num(3), Num(3))` → `Num(6)`

結果: `mov $6, %rax`

## ラッピング算術

すべての演算でRustの`wrapping_*`メソッドを使用し、符号付き整数オーバーフローに対するCの動作（実装定義だが、2の補数マシンでは通常ラッピング）と一致させています。

## 畳み込みの対象外

- 単項演算（例: `-1`はレキサーによって負のリテラルとして`Num(-1)`にパースされるか、`UnaryOp(Neg, Num(1))`として畳み込みされない）
- 変数を含む式
- 短絡評価演算子（`&&`, `||`）
- 三項演算子（`? :`）
