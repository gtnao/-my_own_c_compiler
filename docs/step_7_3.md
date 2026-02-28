# Step 7.3: アロー演算子 `->`

## 概要

構造体ポインタからメンバにアクセスするアロー演算子 `->` を実装する。

```c
struct { int x; int y; } s;
struct { int x; int y; } *p = &s;
p->x = 42;    // equivalent to (*p).x = 42
return p->y;  // equivalent to (*p).y
```

## `->` の意味

`p->member` は `(*p).member` の糖衣構文（syntactic sugar）。ポインタを経由した構造体メンバアクセスを簡潔に書ける。

### 変換の流れ

```
p->x
↓ パーサーが変換
(*p).x
↓ AST
Member(Deref(Var("p")), "x")
```

## トークンの追加 (`token.rs`)

```rust
Arrow,  // ->
```

## レクサーの変更 (`lexer.rs`)

`-` の後に `>` が来る場合を、`--` や `-=` よりも前に判定する：

```rust
if ch == '-' && self.peek_next() == Some('>') {
    self.pos += 2;
    tokens.push(Token { kind: TokenKind::Arrow, pos });
    continue;
}
```

優先順位は以下の順序で判定する必要がある：
1. `->` (Arrow)
2. `--` (MinusMinus)
3. `-=` (MinusEq)
4. `-` (Minus)

`->` を先に判定しないと、`-` トークンとして消費されてしまい、`>` が独立したトークンになる。

## パーサーの変更 (`parser.rs`)

`postfix()` に `Arrow` トークンの処理を追加：

```rust
TokenKind::Arrow => {
    // p->member is (*p).member
    self.advance();
    let member_name = /* parse identifier */;
    self.advance();
    node = Expr::Member(
        Box::new(Expr::Deref(Box::new(node))),
        member_name
    );
}
```

パーサーの段階で `Deref + Member` に変換するため、AST・コード生成の変更は不要。既存の `Deref` と `Member` のコード生成がそのまま使われる。

## コード生成

新しいコード生成は不要。パーサーが `p->x` を `(*p).x` に変換するため、既存の以下のコードパスが使われる：

1. `gen_addr(Member(Deref(Var("p")), "x"))`:
   - `gen_addr(Deref(Var("p")))` → `gen_expr(Var("p"))` → ポインタ値を `%rax` に
   - メンバオフセットを `add` で加算

2. `gen_expr(Member(...))`:
   - `gen_addr` でアドレスを計算
   - `emit_load_indirect` で型に応じた値をロード

### 具体例

`p->x`（`p` は `struct { int x; int y; } *` 型、`p` は `rbp-24` に格納）：

```asm
# gen_addr(Deref(Var("p"))) → gen_expr(Var("p"))
  mov -24(%rbp), %rax    # load pointer p
# Member offset for x is 0, no add needed
# emit_load_indirect for int
  movslq (%rax), %rax    # load int value from *p
```

`p->y`：

```asm
  mov -24(%rbp), %rax    # load pointer p
  add $4, %rax           # offset of y
  movslq (%rax), %rax    # load int value
```

## テストケース

```bash
# basic arrow access
assert 3 'int main() { struct { int x; int y; } s; s.x = 1; s.y = 2;
  struct { int x; int y; } *p = &s; return p->x + p->y; }'

# single member
assert 10 'int main() { struct { int a; } s; s.a = 10;
  struct { int a; } *p = &s; return p->a; }'

# write through arrow
assert 42 'int main() { struct { int x; int y; } s;
  struct { int x; int y; } *p = &s; p->x = 42; return s.x; }'

# multiple members
assert 7 'int main() { struct { int a; int b; } s; s.a = 3; s.b = 4;
  struct { int a; int b; } *p = &s; return p->a + p->b; }'
```
