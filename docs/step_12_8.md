# ステップ 12.8: 1行での複数変数宣言

## 概要

同じ基本型の複数の変数を1つの文で宣言できるようにする:

```c
int a = 1, b = 2, c = 3;
int x, y;
```

## 実装

最初の変数宣言子（初期化子の有無を問わず）をパースした後、`,` があるかチェックする。見つかった場合は `;` まで追加の宣言子をパースし続ける:

```rust
if self.current().kind == TokenKind::Comma {
    let mut stmts = vec![first_var_decl];
    while self.current().kind == TokenKind::Comma {
        self.advance();
        // Parse pointer stars for this declarator
        // Parse name
        // Parse optional initializer
        stmts.push(next_var_decl);
    }
    self.expect(TokenKind::Semicolon);
    return Stmt::Block(stmts);
}
```

各追加宣言子は独自のポインタスターを持てる。例えば:
```c
int a, *b, **c;  // a: int, b: int*, c: int**
```

基本型（この場合 `int`）は共有されるが、各宣言子は独立してポインタの間接参照を追加する。

## 脱糖

`int a = 1, b = 2;` は Block に脱糖される:
```
Block([
    VarDecl { name: "a", ty: int, init: Some(1) },
    VarDecl { name: "b", ty: int, init: Some(2) },
])
```

## テストケース

```c
int a = 1, b = 2; return a + b;                // => 3
int a = 1, b = 2, c = 3; return a + b + c;     // => 6
int a, b; a = 1; b = 2; return a + b;           // => 3
```
