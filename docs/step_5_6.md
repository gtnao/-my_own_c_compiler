# Step 5.6: sizeof と配列

## 概要

`sizeof` 演算子を拡張し、式に対しても使えるようにする。
これにより、配列変数のサイズ取得が可能になる。

```c
int a[3];
sizeof(a);      // 12 (= 4 * 3)
sizeof(a[0]);   // 4  (= sizeof(int))

int b[2][3];
sizeof(b);      // 24 (= 4 * 3 * 2)
sizeof(b[0]);   // 12 (= 4 * 3)
```

## sizeof の2つの形式

### 1. sizeof(type) — 型名に対する sizeof

```c
sizeof(int)    // 4
sizeof(char)   // 1
sizeof(long)   // 8
```

既存の実装。パーサーが型名を読み取り、`SizeofType(Type)` ノードを生成。
コード生成で `ty.size()` を定数として出力。

### 2. sizeof expr — 式に対する sizeof

```c
int a[3];
sizeof(a)      // 12 — a の型は Array(Int, 3)
sizeof(a[0])   // 4  — a[0] の型は Int

int x = 5;
sizeof(x)      // 4  — x の型は Int
sizeof(&x)     // 8  — &x の型は Ptr(Int)
```

新規実装。式は**評価されない**（副作用なし）。
式の型のみを推論し、そのサイズを定数として出力する。

## パーサーの判定ロジック

`sizeof` の後に `(type)` か式かを判定する必要がある。

```rust
TokenKind::Sizeof => {
    self.advance();
    // sizeof(type) の判定: "(" の後が型キーワードなら型名
    if self.current().kind == TokenKind::LParen
        && self.pos + 1 < self.tokens.len()
        && Self::is_type_keyword(&self.tokens[self.pos + 1].kind)
    {
        self.advance(); // consume "("
        let ty = self.parse_type();
        self.expect(TokenKind::RParen);
        return Expr::SizeofType(ty);
    }
    // それ以外は式
    let operand = self.unary();
    return Expr::SizeofExpr(Box::new(operand));
}
```

`sizeof(a)` の場合：
1. `(` の後が `a`（Ident）→ 型キーワードではない
2. `unary()` を呼び出す → `primary()` → `(expr)` として `a` をパース
3. `SizeofExpr(Var("a"))` ノードが生成される

`sizeof(int)` の場合：
1. `(` の後が `int` → 型キーワード
2. `(` を消費、`parse_type()` で `Int` をパース、`)` を消費
3. `SizeofType(Int)` ノードが生成される

## コード生成

```rust
Expr::SizeofExpr(expr) => {
    let ty = self.expr_type(expr);
    self.emit(&format!("  mov ${}, %rax", ty.size()));
}
```

式を**評価しない**点が重要。`expr_type()` は式の型を静的に推論するだけで、
コードを生成しない。結果として即値の `mov` 命令1つになる。

### 例

```c
int a[3];
sizeof(a);
```

```asm
  mov $12, %rax   # sizeof(int[3]) = 4 * 3 = 12
```

式が副作用を持っていても評価されない：
```c
sizeof(a[i++])  // i++ は実行されない
```

## 型推論チェーン

`sizeof(a[0])` where `int a[2][3]`:

1. `a[0]` は `*(a + 0)` に脱糖
2. `expr_type(Deref(Add(Var("a"), Num(0))))`
3. `expr_type(Add(Var("a"), Num(0)))`:
   - `lhs_ty = Array(Array(Int, 3), 2)` → `is_pointer()` = true
   - 結果: `Ptr(Array(Int, 3))`
4. `Deref(Ptr(Array(Int, 3)))` → `Array(Int, 3)`
5. `Array(Int, 3).size()` = 12

## テスト

ユニットテスト 22 件 + 統合テスト 201 件（8 件追加）= 223 件
