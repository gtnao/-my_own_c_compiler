# Step 2.8: 変数宣言と初期化の統合

## 概要

`int a = 5;` のように、変数宣言と同時に初期値を設定できるようにする。

## 現状

この機能は Step 2.2〜2.3 で既に実装済み。
`VarDecl { name, init: Option<Expr> }` が AST に存在し、
パーサーの `var_decl()` メソッドが `int ident ("=" expr)? ";"` をパースする。

このステップでは、実装の仕組みを改めてドキュメント化する。

## なぜ初期化子 (initializer) が必要か

初期化子がない場合、変数宣言と代入は分離される：

```c
int a;       // 宣言のみ（未初期化）
a = 5;       // 代入文（ExprStmt）
```

初期化子があると、1文で宣言と初期化を同時に行える：

```c
int a = 5;   // 宣言 + 初期化
```

C言語ではこれらは意味が異なる。宣言時の `=` は代入演算子ではなく、
初期化子 (initializer) と呼ばれる。ただし、現時点では生成されるコードは
ほぼ同じ（変数のスタック位置に値を格納する）。

## AST

```rust
Stmt::VarDecl {
    name: String,           // 変数名
    init: Option<Expr>,     // 初期化式（None = 未初期化）
}
```

`init` が `Some(expr)` の場合、式を評価してスタック上の変数に格納する。
`init` が `None` の場合、何もしない（変数はスタック上に確保されるが、値は不定）。

## パーサー

```rust
fn var_decl(&mut self) -> Stmt {
    self.expect(TokenKind::Int);       // "int" を消費
    let name = /* 識別子を読む */;
    self.advance();

    // ローカル変数リストに追加（重複時はスキップ）
    if !self.locals.contains(&name) {
        self.locals.push(name.clone());
    }

    // "=" があれば初期化式をパース
    let init = if self.current().kind == TokenKind::Eq {
        self.advance();                // "=" を消費
        Some(self.expr())              // 初期化式をパース
    } else {
        None
    };
    self.expect(TokenKind::Semicolon); // ";" を消費
    Stmt::VarDecl { name, init }
}
```

### パーサーの呼び出し箇所

`stmt()` メソッドのマッチ分岐で、`TokenKind::Int` を見つけたら `var_decl()` を呼ぶ：

```rust
TokenKind::Int => {
    self.var_decl()
}
```

`for` 文の `init` 部分でも `var_decl()` が使われる：

```rust
// for (int i = 0; ...)
let init = if self.current().kind == TokenKind::Int {
    Some(Box::new(self.var_decl()))    // var_decl が ";" まで消費
} else { ... };
```

## コード生成

```rust
Stmt::VarDecl { name, init } => {
    if let Some(expr) = init {
        self.gen_expr(expr);                          // 初期化式を評価 → %rax
        let offset = self.locals[name];               // 変数のスタックオフセット
        self.emit(&format!("  mov %rax, -{}(%rbp)", offset));  // スタックに格納
    }
    // init が None なら何もしない
}
```

### メモリレイアウト

変数は `%rbp` からの負方向オフセットでアクセスされる：

```
高アドレス
┌──────────────┐
│  古い %rbp   │ ← %rbp が指す位置
├──────────────┤
│  変数 a      │ ← -8(%rbp)   最初に宣言された変数
├──────────────┤
│  変数 b      │ ← -16(%rbp)  2番目に宣言された変数
├──────────────┤
│  変数 c      │ ← -24(%rbp)  3番目に宣言された変数
├──────────────┤
│   ...        │
└──────────────┘ ← %rsp
低アドレス
```

### 具体例

入力: `int main() { int a = 3; int b = 5; return a + b; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp          # 変数2個 × 8バイト = 16バイト確保

  # int a = 3;
  mov $3, %rax           # 初期化式 3 を評価
  mov %rax, -8(%rbp)     # a に格納

  # int b = 5;
  mov $5, %rax           # 初期化式 5 を評価
  mov %rax, -16(%rbp)    # b に格納

  # return a + b;
  mov $0, %rax           # (rhs の評価準備)
  ...                    # a + b を評価して %rax に結果
  jmp .Lreturn.main

  mov $0, %rax           # デフォルト return 0
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret
```

## 未初期化変数の危険性

```c
int a;          // 未初期化 → スタック上のゴミ値が入っている
return a;       // 不定値を返す（C言語仕様上は未定義動作）
```

現在の実装では未初期化変数の使用を検出しない。
将来的に意味解析 (`sema.rs`) を導入した際に、
未初期化変数の使用を警告するようにできる。

## `=` トークンの二重の意味

現在、`=` (`TokenKind::Eq`) は2つの文脈で使われる：

1. **代入演算子**: `a = 5;` → パーサーの `assign()` で処理
2. **初期化子**: `int a = 5;` → パーサーの `var_decl()` で処理

パーサーはトークン列の中の位置（`int` キーワードの後かどうか）で
どちらの意味かを判別している。これは C 言語の仕様通り。
