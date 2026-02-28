# Step 9.4: 複合リテラル

## 概要

C99の複合リテラル（compound literal）を実装する。`(type){initializers}` の形式で、式の途中で一時的な配列や構造体を作成できる。

```c
// Array compound literal
int *p = (int[]){1, 2, 3};
// p[0]=1, p[1]=2, p[2]=3

// Struct compound literal with member access
struct { int a; int b; } s = (struct { int a; int b; }){3, 7};
```

## 仕組み

### パーサーでの検出

複合リテラルは `(type){...}` というパターンで、キャスト式 `(type)expr` と先頭が同じ。区別は `)` の直後に `{` が来るかどうかで判断する：

```rust
// In unary():
if self.current().kind == TokenKind::LParen
    && is_type_keyword(next_token)
{
    self.advance(); // consume "("
    let ty = self.parse_type();
    // Parse optional array dimensions: (int[3]) or (int[])
    // ...
    self.expect(TokenKind::RParen);

    if self.current().kind == TokenKind::LBrace {
        // Compound literal: (type){initializers}
        return self.parse_compound_literal(ty, has_empty_bracket);
    }

    // Regular cast: (type)expr
    let operand = self.unary();
    return Expr::Cast { ty, expr: Box::new(operand) };
}
```

### 型パース内での配列次元

`(int[3])` のように型名の後に配列次元が来る場合に対応するため、`parse_type()` の後に追加の配列次元パースを行う：

```rust
let mut ty = self.parse_type();
while self.current().kind == TokenKind::LBracket {
    self.advance();
    if self.current().kind == TokenKind::RBracket {
        // Empty brackets: (int[]){...}
        has_empty_bracket = true;
        ty = Type::array_of(ty, 0); // placeholder
    } else {
        let n = parse_num();
        self.expect(RBracket);
        ty = Type::array_of(ty, n);
    }
}
```

### desugar（展開）

複合リテラルは、パース時に匿名ローカル変数 + 初期化代入 + カンマ演算子の連鎖に展開される：

```c
(int[]){1, 2, 3}
```

→ 以下に展開：

```c
(__compound_1[0] = 1, __compound_1[1] = 2, __compound_1[2] = 3, __compound_1)
```

カンマ演算子 `,` は左辺を評価して副作用を実行し、右辺の値を返す。この連鎖により：

1. 各要素への代入が順番に実行される
2. 最後に匿名変数自体が式の値として返される

### 新しいASTノードは不要

この desugar アプローチにより、新しいASTノードやコード生成の変更は不要。既存の `Comma`、`Assign`、`Var` ノードの組み合わせで表現できる。

### gen_addr の Comma 対応

構造体複合リテラルのメンバアクセス `((struct {...}){a, b}).member` では、`Member` 式の基底がカンマ式になる。`gen_addr` でカンマ式を処理できるように拡張：

```rust
Expr::Comma(lhs, rhs) => {
    // Evaluate lhs for side effects, then get address of rhs
    self.gen_expr(lhs);
    self.gen_addr(rhs);
}
```

### expr_type の Comma 対応

カンマ式の型は右辺の型（最後の式の型）：

```rust
Expr::Comma(_, rhs) => self.expr_type(rhs),
```

## テストケース

```bash
# Array compound literal
assert 3 'int main() { int *p = (int[]){1, 2, 3}; return p[2]; }'
assert 1 'int main() { int *p = (int[]){1, 2, 3}; return p[0]; }'
assert 6 'int main() { int *p = (int[3]){1, 2, 3}; return p[0] + p[1] + p[2]; }'

# Struct compound literal with member access
assert 10 'int main() { return ((struct { int a; int b; }){3, 7}).a + ((struct { int a; int b; }){3, 7}).b; }'
```
