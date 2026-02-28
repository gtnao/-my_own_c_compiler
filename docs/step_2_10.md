# Step 2.10: インクリメント/デクリメント演算子

## 概要

`++` と `--` の前置（prefix）・後置（postfix）をサポートする。

| 式 | 動作 | 戻り値 |
|----|------|--------|
| `++a` | a を +1 してから返す | **新しい**値 |
| `a++` | 古い値を返してから a を +1 | **古い**値 |
| `--a` | a を -1 してから返す | **新しい**値 |
| `a--` | 古い値を返してから a を -1 | **古い**値 |

前置と後置の違いは「式としての値がいつの時点の値か」にある。

## 文法上の位置

C言語の演算子優先順位において、後置 `++`/`--` は前置よりも優先度が**高い**。
パーサーの関数呼び出し階層で表現すると：

```
expr → assign → equality → relational → add → mul → unary → postfix → primary
                                                       ↑         ↑
                                                 前置 ++/--   後置 ++/--
```

### 前置 (prefix): `unary` レベル

```rust
// unary = ("+" | "-") unary | "++" unary | "--" unary | postfix
fn unary(&mut self) -> Expr {
    match self.current().kind {
        TokenKind::PlusPlus => {
            self.advance();
            let operand = self.unary();
            Expr::PreInc(Box::new(operand))
        }
        TokenKind::MinusMinus => {
            self.advance();
            let operand = self.unary();
            Expr::PreDec(Box::new(operand))
        }
        // ... +, - は既存
        _ => self.postfix(),   // ← primary() から変更
    }
}
```

### 後置 (postfix): 新しい `postfix` レベル

```rust
// postfix = primary ("++" | "--")*
fn postfix(&mut self) -> Expr {
    let mut node = self.primary();

    loop {
        match self.current().kind {
            TokenKind::PlusPlus => {
                self.advance();
                node = Expr::PostInc(Box::new(node));
            }
            TokenKind::MinusMinus => {
                self.advance();
                node = Expr::PostDec(Box::new(node));
            }
            _ => break,
        }
    }

    node
}
```

後置はループで複数回マッチできる（`a++++` は意味的にはエラーだが、文法上はパースできる）。

## AST

```rust
Expr::PreInc(Box<Expr>)    // ++a
Expr::PreDec(Box<Expr>)    // --a
Expr::PostInc(Box<Expr>)   // a++
Expr::PostDec(Box<Expr>)   // a--
```

糖衣構文として `Assign + BinOp` に展開する方法もあるが、
後置の場合は「古い値を返す」という特殊な動作があるため、
専用のASTノードを使うほうがコード生成がシンプルになる。

## コード生成

### 前置インクリメント (`++a`)

```rust
Expr::PreInc(operand) => {
    let offset = self.locals[name];
    self.emit(&format!("  mov -{}(%rbp), %rax", offset));  // a を読む
    self.emit("  add $1, %rax");                            // +1
    self.emit(&format!("  mov %rax, -{}(%rbp)", offset));   // a に書き戻す
    // %rax = 新しい値（インクリメント後）
}
```

`%rax` には**新しい値**が残るため、`return ++a` は新しい値を返す。

### 後置インクリメント (`a++`)

```rust
Expr::PostInc(operand) => {
    let offset = self.locals[name];
    self.emit(&format!("  mov -{}(%rbp), %rax", offset));  // a を読む（古い値）
    self.emit("  mov %rax, %rdi");                          // 古い値を %rdi に退避
    self.emit("  add $1, %rdi");                            // %rdi を +1
    self.emit(&format!("  mov %rdi, -{}(%rbp)", offset));   // 新しい値を a に書く
    // %rax = 古い値（インクリメント前）
}
```

ポイント：`%rax` に古い値を残し、`%rdi` を使って新しい値を計算・格納する。
これにより `return a++` は古い値を返しつつ、変数には新しい値が入る。

### レジスタの使い分け

```
%rax: 式の結果として返す値（return文やさらなる計算で使われる）
%rdi: 後置演算で新しい値を一時的に保持するために使用
```

### 具体例

入力: `int main() { int a = 5; return a++; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # int a = 5;
  mov $5, %rax
  mov %rax, -8(%rbp)       # a = 5

  # return a++;
  mov -8(%rbp), %rax       # %rax = 5 (古い値)
  mov %rax, %rdi            # %rdi = 5
  add $1, %rdi              # %rdi = 6 (新しい値)
  mov %rdi, -8(%rbp)        # a = 6 (変数を更新)
  # %rax = 5 のまま → return 5
  jmp .Lreturn.main

  mov $0, %rax
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret                       # 終了コード 5
```

入力: `int main() { int a = 5; return ++a; }`

```asm
  # return ++a;
  mov -8(%rbp), %rax       # %rax = 5
  add $1, %rax              # %rax = 6 (新しい値)
  mov %rax, -8(%rbp)        # a = 6
  # %rax = 6 → return 6
  jmp .Lreturn.main
```

## for文との組み合わせ

`++` の導入により、for文のインクリメント部が簡潔に書ける：

```c
// Before: i = i + 1
for (i = 0; i < 10; i = i + 1) ...

// After: i++
for (i = 0; i < 10; i++) ...
```

for文のインクリメント部は式として評価されるが、その結果は捨てられるため、
前置・後置のどちらを使っても動作は同じ。

## トークンの曖昧性

`++` と `+=` は先頭が同じ `+` なので、lexer で先に `++` をチェックし、
マッチしなければ `+=` をチェックする。順序が重要：

```rust
// 先に ++ をチェック
if ch == '+' && self.peek_next() == Some('+') { ... }
// 次に += をチェック
if ch == '+' && self.peek_next() == Some('=') { ... }
```
