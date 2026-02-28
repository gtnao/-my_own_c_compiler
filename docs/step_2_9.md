# Step 2.9: 複合代入演算子

## 概要

`+=`, `-=`, `*=`, `/=`, `%=` の複合代入演算子をサポートする。

## 複合代入とは

`a += b` は `a = a + b` の省略形。C言語では「糖衣構文（syntactic sugar）」と呼ばれる。
ただし、厳密には副作用のある式（例: `*p++ += 1`）の場合、左辺は1回だけ評価されるという違いがある。
現時点では変数しか左辺に来ないため、単純な展開で問題ない。

| 演算子 | 展開後 |
|--------|--------|
| `a += b` | `a = a + b` |
| `a -= b` | `a = a - b` |
| `a *= b` | `a = a * b` |
| `a /= b` | `a = a / b` |
| `a %= b` | `a = a % b` |

## 変更箇所

### 1. トークン追加 (`token.rs`)

```rust
PlusEq,      // +=
MinusEq,     // -=
StarEq,      // *=
SlashEq,     // /=
PercentEq,   // %=
```

### 2. 字句解析 (`lexer.rs`)

`+`, `-`, `*`, `/`, `%` の直後に `=` があるかを `peek_next()` でチェックする。
2文字トークンのチェックは単一文字トークンのマッチより**前に**行う必要がある。

```rust
if ch == '+' && self.peek_next() == Some('=') {
    self.pos += 2;
    tokens.push(Token { kind: TokenKind::PlusEq, pos });
    continue;
}
// -=, *=, /=, %= も同様
```

**重要**: `==` と `=` の区別と同じ仕組み。先に2文字を試し、
マッチしなければ1文字トークンとして処理する。

### 3. パーサー (`parser.rs`)

`assign()` メソッドで、通常の `=` の後に複合代入のチェックを追加：

```rust
fn assign(&mut self) -> Expr {
    let node = self.equality();

    // 通常の代入
    if self.current().kind == TokenKind::Eq {
        self.advance();
        let rhs = self.assign();
        return Expr::Assign { lhs: Box::new(node), rhs: Box::new(rhs) };
    }

    // 複合代入: a op= b → a = a op b に展開
    let op = match self.current().kind {
        TokenKind::PlusEq => Some(BinOp::Add),
        TokenKind::MinusEq => Some(BinOp::Sub),
        TokenKind::StarEq => Some(BinOp::Mul),
        TokenKind::SlashEq => Some(BinOp::Div),
        TokenKind::PercentEq => Some(BinOp::Mod),
        _ => None,
    };

    if let Some(op) = op {
        self.advance();
        let rhs = self.assign();
        return Expr::Assign {
            lhs: Box::new(node.clone()),     // a （左辺）
            rhs: Box::new(Expr::BinOp {
                op,
                lhs: Box::new(node),          // a （右辺の左側）
                rhs: Box::new(rhs),           // b （右辺の右側）
            }),
        };
    }

    node
}
```

#### AST の変換イメージ

`a += 5` は以下のASTに変換される：

```
Assign
├── lhs: Var("a")
└── rhs: BinOp(Add)
         ├── lhs: Var("a")
         └── rhs: Num(5)
```

これは `a = a + 5` と全く同じAST。

### 4. コード生成

ASTレベルで `Assign` + `BinOp` に展開済みなので、
コード生成に変更は不要。既存の `Assign` と `BinOp` の処理がそのまま使われる。

## 生成されるアセンブリ

入力: `int main() { int a = 10; a += 5; return a; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # int a = 10;
  mov $10, %rax
  mov %rax, -8(%rbp)

  # a += 5;  →  a = a + 5;
  # (1) rhs の評価: a + 5
  mov $5, %rax            # rhs of + : 5
  push %rax               # スタックに保存
  mov -8(%rbp), %rax      # lhs of + : a の値 (10)
  pop %rdi                # rdi = 5
  add %rdi, %rax          # rax = 10 + 5 = 15

  # (2) 代入: a = result
  mov %rax, -8(%rbp)      # a に 15 を格納

  # return a;
  mov -8(%rbp), %rax
  jmp .Lreturn.main

  mov $0, %rax
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret
```

## `node.clone()` について

パーサーで `node.clone()` を使っている理由：

```rust
Expr::Assign {
    lhs: Box::new(node.clone()),  // 1回目の node 使用（代入先）
    rhs: Box::new(Expr::BinOp {
        lhs: Box::new(node),       // 2回目の node 使用（読み取り）
        ...
    }),
}
```

`a += b` を `a = a + b` に展開する際、`a` が2箇所で使われるため、
Rustの所有権システム上 `clone()` が必要になる。
`Expr` は `#[derive(Clone)]` を付けてあるのでこれが可能。

## 糖衣構文展開のメリット

コード生成側を変更せずに新機能を追加できるのが糖衣構文展開のメリット：

- パーサーで変換するだけ → コード生成は既存のまま
- ASTの種類が増えない → 保守性が高い
- 将来の最適化もAssignとBinOpに対して行えば自動的に適用される
