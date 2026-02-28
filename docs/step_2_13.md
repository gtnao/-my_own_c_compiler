# Step 2.13: コンマ演算子と三項演算子

## 概要

コンマ演算子 (`,`) と三項演算子 (`?:`) をサポートする。

## コンマ演算子

### 構文

```c
expr1, expr2, expr3
```

左辺を評価（結果は捨てる）→ 右辺を評価（結果が全体の値）。
左結合で、複数のコンマは順に結合される。

```c
(1, 2, 3)   // 1を評価して捨て、2を評価して捨て、3が結果 → 3
(a = 5, a)  // a=5を実行、aを返す → 5
```

### 優先順位

コンマ演算子はC言語で**最も優先順位が低い**演算子。
代入よりも低い：

```
expr = assign ("," assign)*     ← コンマは最低
assign = ternary ("=" ...)
ternary = logical_or ("?" ...)
...
```

```c
a = 1, b = 2   // (a = 1), (b = 2)  ←  = はコンマより先に結合
```

### パーサー

```rust
// expr = assign ("," assign)*
fn expr(&mut self) -> Expr {
    let mut node = self.assign();
    while self.current().kind == TokenKind::Comma {
        self.advance();
        let rhs = self.assign();
        node = Expr::Comma(Box::new(node), Box::new(rhs));
    }
    node
}
```

### コード生成

```rust
Expr::Comma(lhs, rhs) => {
    self.gen_expr(lhs);   // 左辺を評価（結果は捨てる）
    self.gen_expr(rhs);   // 右辺を評価（%rax に結果が残る）
}
```

左辺の結果（`%rax`）は、右辺の評価で上書きされるため自然に捨てられる。

## 三項演算子（条件演算子）

### 構文

```c
cond ? then_expr : else_expr
```

`cond` が非0なら `then_expr` を評価、0なら `else_expr` を評価。
if文の式バージョン。

```c
1 ? 10 : 20   // → 10
0 ? 10 : 20   // → 20
a > 3 ? 10 : 20   // a が 3 より大きければ 10、そうでなければ 20
```

### 優先順位

三項演算子は代入と論理ORの間：

```
assign = ternary ("=" assign)?
ternary = logical_or ("?" expr ":" ternary)?
logical_or = ...
```

三項演算子は右結合：

```c
a ? b : c ? d : e   →   a ? b : (c ? d : e)
```

else部分は `ternary()` を再帰呼び出しすることで右結合を実現。
then部分は `expr()` を呼び出す（コンマ演算子も含む）。

### パーサー

```rust
// ternary = logical_or ("?" expr ":" ternary)?
fn ternary(&mut self) -> Expr {
    let node = self.logical_or();

    if self.current().kind == TokenKind::Question {
        self.advance();
        let then_expr = self.expr();        // then部分はexpr（コンマ含む）
        self.expect(TokenKind::Colon);
        let else_expr = self.ternary();     // else部分は再帰（右結合）
        return Expr::Ternary {
            cond: Box::new(node),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        };
    }

    node
}
```

### AST

```rust
Expr::Ternary {
    cond: Box<Expr>,
    then_expr: Box<Expr>,
    else_expr: Box<Expr>,
}
```

### コード生成

if文と同じラベルベースの分岐パターン：

```rust
Expr::Ternary { cond, then_expr, else_expr } => {
    let else_label = self.new_label();
    let end_label = self.new_label();

    self.gen_expr(cond);                         // 条件を評価
    self.emit("  cmp $0, %rax");
    self.emit(&format!("  je {}", else_label));   // 0なら else へ
    self.gen_expr(then_expr);                    // then を評価
    self.emit(&format!("  jmp {}", end_label));   // end へ
    self.emit(&format!("{}:", else_label));
    self.gen_expr(else_expr);                    // else を評価
    self.emit(&format!("{}:", end_label));
}
```

### if文との違い

| 特徴 | if文 | 三項演算子 |
|------|------|-----------|
| 型 | 文 (Statement) | 式 (Expression) |
| 使える場所 | 文が書けるところ | 式が書けるところ |
| 例 | `if (c) a=1; else a=2;` | `a = c ? 1 : 2;` |
| return値 | なし | あり |

三項演算子は`return`文の中で直接使える：
```c
return a > 0 ? a : -a;  // 絶対値
```

### 具体例

入力: `int main() { return 1 ? 10 : 20; }`

```asm
  # cond: 1
  mov $1, %rax
  cmp $0, %rax
  je .L0               # 1 != 0 なのでジャンプしない

  # then: 10
  mov $10, %rax
  jmp .L1               # end へ

.L0:                     # (到達しない)
  # else: 20
  mov $20, %rax

.L1:                     # end
  # %rax = 10
  jmp .Lreturn.main
```

## コンマ演算子の注意点

コンマ演算子と、関数呼び出しの引数区切り・変数宣言の区切りは異なるもの。

```c
f(1, 2, 3)     // 関数呼び出し — コンマは引数区切り
int a=1, b=2;  // 変数宣言 — コンマは宣言区切り（未実装）
(1, 2, 3)      // コンマ演算子 — 左から評価して最後の値を返す
```

現時点では関数呼び出しも複数変数宣言も未実装なので区別の問題は発生しない。
