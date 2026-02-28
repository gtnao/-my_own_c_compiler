# Step 9.1: 配列初期化子

## 概要

配列宣言時に `{...}` で初期値を指定する機能を実装する。

```c
int a[3] = {1, 2, 3};
int b[] = {10, 20, 30, 40};  // size inferred from initializer
```

## 実装方針

配列初期化子をパーサーの段階で「変数宣言 + 要素ごとの代入」に分解（desugar）する。

### 変換例

```c
int a[3] = {1, 2, 3};
```

は以下と同等のAST列に変換される：

```c
int a[3];     // VarDecl (init: None)
*(a + 0) = 1; // ExprStmt(Assign)
*(a + 1) = 2; // ExprStmt(Assign)
*(a + 2) = 3; // ExprStmt(Assign)
```

パーサーの `var_decl()` から `Stmt::Block(stmts)` を返すことで、1つの宣言文の位置に複数の文を生成する。

### サイズ省略の対応

`int a[] = {1, 2, 3}` のように配列サイズを省略した場合：
1. `[]` を空ブラケットとしてパースし、仮のサイズ0を設定
2. 初期化リストの要素数からサイズを確定
3. `Type::array_of(base, count)` で正しい型を構築

## パーサーの変更

### 空ブラケットのパース

```rust
if self.current().kind == TokenKind::RBracket {
    // Empty brackets: int a[] = {...}
    has_empty = true;
    self.advance();
    dims.push(0); // placeholder
}
```

### 初期化リストのパース

`=` の後に `{` が来たら初期化リストモード：

```rust
if self.current().kind == TokenKind::LBrace {
    self.advance();
    let mut init_exprs = Vec::new();
    while self.current().kind != TokenKind::RBrace {
        init_exprs.push(self.assign());
        if Comma { self.advance(); }
    }
    expect(RBrace);
    expect(Semicolon);

    // Fix type for empty brackets
    let ty = if has_empty_bracket {
        Type::array_of(base, init_exprs.len())
    } else {
        ty
    };

    // Generate initialization statements
    let mut stmts = vec![VarDecl { name, ty, init: None }];
    for (i, expr) in init_exprs.into_iter().enumerate() {
        stmts.push(ExprStmt(Assign {
            lhs: Deref(BinOp(Add, Var(name), Num(i))),
            rhs: expr,
        }));
    }
    return Stmt::Block(stmts);
}
```

### ポインタ算術による要素アクセス

代入先は `*(a + i)` として生成される。これは配列の添字アクセス `a[i]` と同じ意味で、既存のポインタ算術コード生成（`imul` によるスケーリング）が再利用される。

## コード生成

変更不要。パーサーが初期化子を代入文列に変換するため、既存のコード生成がそのまま動作する。

## テストケース

```bash
# explicit size with initializer
assert 1 'int main() { int a[3] = {1, 2, 3}; return a[0]; }'
assert 2 'int main() { int a[3] = {1, 2, 3}; return a[1]; }'
assert 3 'int main() { int a[3] = {1, 2, 3}; return a[2]; }'
assert 6 'int main() { int a[3] = {1, 2, 3}; return a[0] + a[1] + a[2]; }'

# inferred size from initializer
assert 10 'int main() { int a[] = {1, 2, 3, 4}; return a[0] + a[1] + a[2] + a[3]; }'

# sizeof with inferred array
assert 4 'int main() { int a[] = {1, 2, 3, 4}; return sizeof(a) / sizeof(a[0]); }'
```
