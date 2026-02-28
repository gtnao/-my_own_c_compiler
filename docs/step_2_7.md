# Step 2.7: ブロック文（複合文）

## 概要

`{ stmt1; stmt2; ... }` のようなブロック文（複合文）をサポートする。
これにより、if/while/for の本体に複数の文を書けるようになる。

## なぜブロック文が必要か

Step 2.5 の while では本体が1文しか書けなかった：

```c
// OK: 1文だけ
while (i < 10) i = i + 1;

// NG: 2文以上はブロックが必要
while (i < 10) { s = s + i; i = i + 1; }  // ← Step 2.7で可能に
```

if/while/for の文法は `stmt` を1つだけ受け取るが、
ブロック `{...}` 自体が1つの `Stmt::Block` として扱われるため、
中に何文入れても文法上は「1つの文」として成立する。

## AST

```rust
Stmt::Block(Vec<Stmt>)   // 0個以上の文のリスト
```

## パーサー

```rust
TokenKind::LBrace => {
    self.advance();           // "{" を消費
    let mut stmts = Vec::new();
    while self.current().kind != TokenKind::RBrace {
        stmts.push(self.stmt());  // 中の文を順番にパース
    }
    self.expect(TokenKind::RBrace);  // "}" を消費
    Stmt::Block(stmts)
}
```

## コード生成

ブロック文のコード生成は単純で、中の文を順番に生成するだけ：

```rust
Stmt::Block(stmts) => {
    for s in stmts {
        self.gen_stmt(s);
    }
}
```

ブロック自体はラベルもジャンプも生成しない。

### 具体例

入力: `while (i <= 10) { s = s + i; i = i + 1; }`

```asm
.L2:                          # while begin
  # cond: i <= 10
  ...
  cmp $0, %rax
  je .L3

  # Block 開始（特別なコードなし）

  # stmt 1: s = s + i
  ...

  # stmt 2: i = i + 1
  ...

  # Block 終了（特別なコードなし）

  jmp .L2
.L3:                          # while end
```

## 関数本体との関係

実は関数本体 `int main() { ... }` の `{ ... }` もブロックと同じ構造。
ただし関数本体は `function()` メソッドが直接パースするので、
`Stmt::Block` ではなく `Function { body: Vec<Stmt> }` に格納される。

将来的にスコープ（変数の寿命）を導入する際、
ブロックごとに新しいスコープを作る必要があるが、現時点では全ての変数がフラットに管理されている。
