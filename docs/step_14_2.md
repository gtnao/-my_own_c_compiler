# Step 14.2: float型とdouble型

## 概要

`float`（32ビットIEEE 754）および`double`（64ビットIEEE 754）浮動小数点型のサポートを追加します。以下を含みます:

- 型宣言（`float f`, `double d`）
- 浮動小数点リテラル（`3.14`, `1.5f`）
- 算術演算（`+`, `-`, `*`, `/`）
- 比較演算子（`==`, `!=`, `<`, `<=`, `>`, `>=`）
- 型変換（int⇔float, int⇔double, float⇔double）
- キャスト式（`(int)3.14`, `(double)5`）
- `sizeof(float)` = 4, `sizeof(double)` = 8

## x86-64の浮動小数点アーキテクチャ

### XMMレジスタ

x86-64は浮動小数点演算用に16個の128ビットSSEレジスタ（`%xmm0`〜`%xmm15`）を提供します。汎用レジスタ（`%rax`など）とは異なり、XMMレジスタは専用のSSE命令を使用します:

| レジスタ | 用途 |
|---|---|
| `%xmm0` | float/double演算の主要アキュムレータ |
| `%xmm1` | 二項演算の副オペランド |
| `%xmm0`〜`%xmm7` | 関数引数の受け渡し（System V ABI） |
| `%xmm0` | 関数の戻り値 |

### SSE命令のサフィックス

- `ss` — スカラー単精度（float, 32ビット）
- `sd` — スカラー倍精度（double, 64ビット）

### 主要な命令

| 操作 | float | double |
|---|---|---|
| メモリからのロード | `movss (%rax), %xmm0` | `movsd (%rax), %xmm0` |
| メモリへのストア | `movss %xmm0, (%rdi)` | `movsd %xmm0, (%rdi)` |
| 加算 | `addss %xmm1, %xmm0` | `addsd %xmm1, %xmm0` |
| 減算 | `subss %xmm1, %xmm0` | `subsd %xmm1, %xmm0` |
| 乗算 | `mulss %xmm1, %xmm0` | `mulsd %xmm1, %xmm0` |
| 除算 | `divss %xmm1, %xmm0` | `divsd %xmm1, %xmm0` |
| 比較 | `ucomiss %xmm1, %xmm0` | `ucomisd %xmm1, %xmm0` |

### 型変換命令

| 変換 | 命令 |
|---|---|
| int → float | `cvtsi2ss %rax, %xmm0` |
| int → double | `cvtsi2sd %rax, %xmm0` |
| float → int（切り捨て） | `cvttss2si %xmm0, %rax` |
| double → int（切り捨て） | `cvttsd2si %xmm0, %rax` |
| float → double | `cvtss2sd %xmm0, %xmm0` |
| double → float | `cvtsd2ss %xmm0, %xmm0` |

注: `cvttss2si`と`cvttsd2si`は**切り捨て**（ゼロ方向への丸め）を使用し、Cのキャストセマンティクスと一致します。非切り捨て版（`cvtss2si`, `cvtsd2si`）は現在の丸めモード（デフォルト: 最近接偶数への丸め）を使用しますが、これはCの`(int)`キャストの動作ではありません。

## 実装

### デュアルレジスタ規約

コンパイラは2つのアキュムレータ規約を維持します:
- **整数式** → 結果は`%rax`に格納
- **float/double式** → 結果は`%xmm0`に格納

`expr_type()`メソッドが式がどちらのレジスタ規約を使用するかを決定します。境界（代入、キャスト、関数呼び出し）では、変換命令が2つの間を橋渡しします。

### 浮動小数点リテラルのロード

浮動小数点リテラルはAST内で`f64`として格納されます（Cのデフォルト: 裸のリテラルは`double`）。ロードには整数レジスタを中間経由として使用します:

```asm
  movabs $4614253070214989087, %rax   # f64 bit pattern of 3.14
  movq %rax, %xmm0                   # move to XMM register
```

### スタックベースの浮動小数点演算

float/double値は整数と同じスタックマシン方式を使用しますが、push/popが異なります:

```rust
fn push_float(&mut self) {
    self.emit("  sub $8, %rsp");
    self.emit("  movsd %xmm0, (%rsp)");
    self.stack_depth += 1;
}

fn pop_float(&mut self, reg: &str) {
    self.emit(&format!("  movsd (%rsp), {}", reg));
    self.emit("  add $8, %rsp");
    self.stack_depth -= 1;
}
```

### 二項演算の型昇格

一方のオペランドが`double`で、もう一方が`float`または`int`の場合、演算は倍精度で実行されます:

```
int + double → cvtsi2sd → addsd (result: double)
float + double → cvtss2sd → addsd (result: double)
float + float → addss (result: float)
```

### `ucomiss`/`ucomisd`による浮動小数点の比較

`ucomiss`および`ucomisd`命令は、整数の`cmp`とは異なるCPUフラグを設定します:

| 条件 | フラグ状態 | セット命令 |
|---|---|---|
| xmm0 > xmm1 | CF=0, ZF=0 | `seta` |
| xmm0 >= xmm1 | CF=0 | `setae` |
| xmm0 < xmm1 | CF=1 | `setb` |
| xmm0 <= xmm1 | CF=1 or ZF=1 | `setbe` |
| xmm0 == xmm1 | ZF=1, PF=0 | `sete` + `setnp` |
| 順序なし（NaN） | PF=1 | — |

等値比較では、NaN同士の比較で`PF=1`がセットされるため、`ZF`と`PF`の両方をチェックする必要があります。

### レキサーの変更

数値レキサーを拡張して浮動小数点リテラルを検出するようにしました:
- 小数点: `3.14`, `.5`, `3.`
- 指数: `1e10`, `1.5e-3`
- floatサフィックス: `3.14f`, `1.0F`
- 整数サフィックス（`L`, `l`, `U`, `u`）も消費して無視するようにしました。

## テストケース

```c
double a = 3.14; return (int)a;           // → 3 (truncation)
double a = 2.5; double b = 1.5; return (int)(a + b);  // → 4
float a = 1.5; float b = 1.5; return (int)(a + b);    // → 3
double a = 10.7; return (int)a;           // → 10 (truncation, not rounding)
sizeof(float) == 4; sizeof(double) == 8;
double a = 1.5; double b = 1.5; return a == b;  // → 1
double a = 1.0; double b = 2.0; return a < b;   // → 1
```
