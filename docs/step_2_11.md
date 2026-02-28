# Step 2.11: 論理演算子

## 概要

`&&`（論理AND）、`||`（論理OR）、`!`（論理NOT）をサポートする。
`&&` と `||` は**短絡評価 (short-circuit evaluation)** を行う。

## 短絡評価とは

`&&` と `||` は左辺の結果だけで全体の結果が確定する場合、右辺を評価しない。

| 演算子 | 短絡条件 | 理由 |
|--------|----------|------|
| `a && b` | `a` が 0 なら `b` を評価しない | 0 && (何か) は必ず 0 |
| `a \|\| b` | `a` が非0なら `b` を評価しない | 非0 \|\| (何か) は必ず 1 |

これは C 言語の仕様で保証されている動作。副作用のある式（例: `p != NULL && *p > 0`）で
ヌルポインタデリファレンスを防ぐために使われる。

## 演算子の優先順位

```
assign  (最低)
  ↓
logical_or   ( || )
  ↓
logical_and  ( && )
  ↓
equality     ( == != )
  ↓
relational   ( < <= > >= )
  ↓
add          ( + - )
  ↓
mul          ( * / % )
  ↓
unary        ( + - ! ++ -- )  ← ! はここ
  ↓
postfix      ( ++ -- )
  ↓
primary  (最高)
```

`||` は `&&` より優先順位が**低い**。これにより：

```c
a || b && c   →   a || (b && c)
```

## トークン

```rust
Bang,       // !
AmpAmp,     // &&
PipePipe,   // ||
```

`!` は単独で使う場合は論理NOT、`!=` の先頭文字としても使われる。
lexerでは先に `!=` をチェックし、マッチしなければ `!` 単体として処理する。

## AST

```rust
// 短絡評価が必要なため、BinOp とは別のノード
Expr::LogicalAnd(Box<Expr>, Box<Expr>)
Expr::LogicalOr(Box<Expr>, Box<Expr>)

// ! は UnaryOp に追加
UnaryOp::LogicalNot
```

### なぜ BinOp に入れないのか

`BinOp` のコード生成は「両辺を評価してからオペレーションする」パターン：

```asm
# BinOp パターン
eval rhs → push → eval lhs → pop %rdi → op %rdi, %rax
```

しかし `&&`/`||` は左辺の結果次第で右辺を**評価しない**必要がある。
ジャンプ命令を使った特殊なコード生成が必要なため、別ノードにする。

## パーサー

### logical_or

```rust
// logical_or = logical_and ("||" logical_and)*
fn logical_or(&mut self) -> Expr {
    let mut node = self.logical_and();
    while self.current().kind == TokenKind::PipePipe {
        self.advance();
        let rhs = self.logical_and();
        node = Expr::LogicalOr(Box::new(node), Box::new(rhs));
    }
    node
}
```

### logical_and

```rust
// logical_and = equality ("&&" equality)*
fn logical_and(&mut self) -> Expr {
    let mut node = self.equality();
    while self.current().kind == TokenKind::AmpAmp {
        self.advance();
        let rhs = self.equality();
        node = Expr::LogicalAnd(Box::new(node), Box::new(rhs));
    }
    node
}
```

### `!` (論理NOT)

unary レベルに追加：

```rust
TokenKind::Bang => {
    self.advance();
    let operand = self.unary();
    Expr::UnaryOp { op: UnaryOp::LogicalNot, operand: Box::new(operand) }
}
```

## コード生成

### `&&` (論理AND) — 短絡評価

```rust
Expr::LogicalAnd(lhs, rhs) => {
    let false_label = self.new_label();
    let end_label = self.new_label();

    self.gen_expr(lhs);                          // 左辺を評価
    self.emit("  cmp $0, %rax");
    self.emit(&format!("  je {}", false_label));  // 左辺が0 → 短絡 → false

    self.gen_expr(rhs);                          // 右辺を評価
    self.emit("  cmp $0, %rax");
    self.emit(&format!("  je {}", false_label));  // 右辺が0 → false

    self.emit("  mov $1, %rax");                 // 両方とも非0 → 結果は 1
    self.emit(&format!("  jmp {}", end_label));

    self.emit(&format!("{}:", false_label));
    self.emit("  mov $0, %rax");                 // いずれかが0 → 結果は 0

    self.emit(&format!("{}:", end_label));
}
```

#### 実行フロー図 (`1 && 0` の場合)

```
eval lhs (= 1)
  cmp $0, %rax     → ZF=0 (1 != 0)
  je .Lfalse        → ジャンプしない

eval rhs (= 0)
  cmp $0, %rax     → ZF=1 (0 == 0)
  je .Lfalse        → ジャンプする ★

.Lfalse:
  mov $0, %rax      → 結果 = 0 ★
.Lend:
```

#### 実行フロー図 (`0 && (何か)` の場合 — 短絡)

```
eval lhs (= 0)
  cmp $0, %rax     → ZF=1 (0 == 0)
  je .Lfalse        → ジャンプする ★ 右辺は評価されない

.Lfalse:
  mov $0, %rax      → 結果 = 0 ★
.Lend:
```

### `||` (論理OR) — 短絡評価

```rust
Expr::LogicalOr(lhs, rhs) => {
    let true_label = self.new_label();
    let end_label = self.new_label();

    self.gen_expr(lhs);                          // 左辺を評価
    self.emit("  cmp $0, %rax");
    self.emit(&format!("  jne {}", true_label));  // 左辺が非0 → 短絡 → true

    self.gen_expr(rhs);                          // 右辺を評価
    self.emit("  cmp $0, %rax");
    self.emit(&format!("  jne {}", true_label));  // 右辺が非0 → true

    self.emit("  mov $0, %rax");                 // 両方とも0 → 結果は 0
    self.emit(&format!("  jmp {}", end_label));

    self.emit(&format!("{}:", true_label));
    self.emit("  mov $1, %rax");                 // いずれかが非0 → 結果は 1

    self.emit(&format!("{}:", end_label));
}
```

`&&` との対比：
- `&&` は `je`（0ならジャンプ）で `.Lfalse` へ
- `||` は `jne`（非0ならジャンプ）で `.Ltrue` へ

### `!` (論理NOT)

```rust
UnaryOp::LogicalNot => {
    self.emit("  cmp $0, %rax");    // 0 と比較
    self.emit("  sete %al");         // 等しければ %al = 1, そうでなければ %al = 0
    self.emit("  movzb %al, %rax");  // %al を 64ビットに拡張
}
```

`!` は比較演算子 `==` の「0との比較」版と同じ仕組み。
`!0` = 1, `!1` = 0, `!42` = 0。

## C言語の論理値

C言語では：
- **false**: 値が 0
- **true**: 値が 0 以外（1 でなくてもよい）

ただし、`&&`, `||`, `!` の**結果**は必ず 0 か 1 になる：
- `2 && 3` → 1（2 でも 3 でもなく 1）
- `2 || 0` → 1
- `!0` → 1
