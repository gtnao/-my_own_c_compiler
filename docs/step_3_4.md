# Step 3.4: 前方宣言とvoid関数

## 概要

2つの関連する機能を追加：

1. **前方宣言（プロトタイプ）**: 関数本体を持たない宣言 `int add(int a, int b);`
2. **void 関数**: 戻り値のない関数 `void noop() { return; }`

```c
int add(int a, int b);                     // 前方宣言
int main() { return add(3, 4); }           // 定義前に呼び出せる
int add(int a, int b) { return a + b; }    // 後で定義

void noop() { return; }                    // void 関数
```

## 1. 前方宣言（プロトタイプ）

### C言語での前方宣言の役割

C言語では、関数を使う前にその存在をコンパイラに知らせる必要がある。
前方宣言はそのためのメカニズム：

```c
// 前方宣言: 関数の型情報のみ
int add(int a, int b);

// main で add を呼べる（定義が後にある）
int main() {
    return add(3, 4);
}

// 実際の定義
int add(int a, int b) {
    return a + b;
}
```

### 実装アプローチ

現在のコンパイラでは型チェック（semantic analysis）を行わないため、
前方宣言は**パースして読み飛ばすだけ**で十分。
コード生成は実際の関数定義にのみ適用する。

### パーサーの変更

関数シグネチャの後のトークンで、定義と前方宣言を区別：

```rust
fn function_or_prototype(&mut self) -> Option<Function> {
    // type ident "(" params? ")"
    // ...パラメータリストをパース...
    self.expect(TokenKind::RParen);

    // ";" → forward declaration (skip)
    if self.current().kind == TokenKind::Semicolon {
        self.advance();
        return None;  // コード生成不要
    }

    // "{" → function definition
    self.expect(TokenKind::LBrace);
    // ...本体をパース...
    Some(Function { name, params, body, locals })
}
```

`parse()` は `Option<Function>` を返す `function_or_prototype()` を呼び、
`None`（前方宣言）の場合は結果リストに追加しない：

```rust
pub fn parse(&mut self) -> Vec<Function> {
    let mut functions = Vec::new();
    while self.current().kind != TokenKind::Eof {
        if let Some(func) = self.function_or_prototype() {
            functions.push(func);
        }
    }
    functions
}
```

## 2. void 関数

### void キーワード

`void` は「型なし」を表す。関数の戻り値型として使われ、
関数が値を返さないことを示す。

### トークンの追加

```rust
// token.rs
pub enum TokenKind {
    // ...
    Void,   // NEW
    // ...
}

// lexer.rs
"void" => TokenKind::Void,
```

### 戻り値型のパース

関数の先頭で `int` または `void` を受け付ける：

```rust
if self.current().kind == TokenKind::Int {
    self.advance();
} else if self.current().kind == TokenKind::Void {
    self.advance();
}
```

現段階では戻り値型の情報は AST に保存しない（型チェックが不要なため）。

### `return;`（値なしリターン）

void 関数では `return;`（セミコロンのみ）が使える。

AST の変更:
```rust
// Before
Stmt::Return(Expr)

// After
Stmt::Return(Option<Expr>)   // None = "return;"
```

パーサーの変更:
```rust
TokenKind::Return => {
    self.advance();
    if self.current().kind == TokenKind::Semicolon {
        self.advance();
        Stmt::Return(None)     // return;
    } else {
        let expr = self.expr();
        self.expect(TokenKind::Semicolon);
        Stmt::Return(Some(expr))  // return expr;
    }
}
```

コード生成の変更:
```rust
Stmt::Return(expr) => {
    if let Some(e) = expr {
        self.gen_expr(e);       // 式があれば評価
    }
    // 式がなくても %rax は未定義のまま（void なので問題なし）
    self.emit(&format!("  jmp .Lreturn.{}", func_name));
}
```

## テストケース

```
// 前方宣言
assert 3 'int ret3(); int ret3() { return 3; } int main() { return ret3(); }'
assert 7 'int add(int a, int b); int main() { return add(3, 4); } int add(int a, int b) { return a + b; }'

// void 関数
assert 0 'void noop() { return; } int main() { noop(); return 0; }'
assert 5 'void noop() {} int main() { noop(); return 5; }'
```

## 設計メモ

### なぜ戻り値型を AST に保存しないか

現段階のコンパイラでは semantic analysis（型チェック）フェーズがない。
型情報は Phase 4 で `types.rs` と `sema.rs` を導入する際に追加する。
今は `int` も `void` も同じコードを生成する（呼び出し側が戻り値を使うかどうかだけの違い）。

### void 関数の暗黙の `return`

void 関数で明示的な `return;` がなくても、関数末尾の
`mov $0, %rax` → `.Lreturn.{name}:` → `ret` で自然に戻る。
`%rax = 0` は void 関数では無意味だが、害もない。
