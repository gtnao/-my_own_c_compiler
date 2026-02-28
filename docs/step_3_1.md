# Step 3.1: 関数呼び出し（引数なし）・複数関数定義

## 概要

このステップでは、C言語の最も重要な機能の一つである**関数**のサポートを開始する。
具体的には以下を実装する：

1. **複数関数の定義**: `int ret3() { return 3; } int main() { return ret3(); }`
2. **関数呼び出し（引数なし）**: `ret3()` のような式

## 設計変更の全体像

### これまでの制約

Phase 2 まで、コンパイラは `main` 関数1つだけを処理していた：
- パーサーは `parse()` で1つの `Function` を返していた
- コード生成は1関数分のアセンブリのみ生成
- リターンラベルは `.Lreturn.main` にハードコード

### 必要な変更

| コンポーネント | 変更内容 |
|---------------|---------|
| AST | `FuncCall` 式ノード追加、`Function` に `locals` フィールド追加 |
| パーサー | 複数関数パース、関数呼び出し解析 |
| コード生成 | 複数関数生成、`call` 命令、スタックアライメント |
| main.rs | 新API対応 |

## 1. AST の変更

### FuncCall ノード

```rust
pub enum Expr {
    // ... 既存のノード ...
    FuncCall {
        name: String,       // 関数名
        args: Vec<Expr>,    // 引数（今回は常に空）
    },
}
```

関数呼び出しは**式**である。`ret3()` は式として評価され、戻り値（`%rax`）が結果となる。

### Function に locals を追加

```rust
pub struct Function {
    pub name: String,
    pub body: Vec<Stmt>,
    pub locals: Vec<String>,  // NEW: この関数のローカル変数
}
```

これまで `parser.get_locals()` で外部からローカル変数を取得していたが、
複数関数を扱うため、各 `Function` が自分のローカル変数を保持するようにした。
これにより、パーサーとコード生成の結合が clean になる。

## 2. パーサーの変更

### 複数関数のパース

```rust
// program = function*
pub fn parse(&mut self) -> Vec<Function> {
    let mut functions = Vec::new();
    while self.current().kind != TokenKind::Eof {
        functions.push(self.function());
    }
    functions
}
```

EOF まで `function()` を繰り返し呼ぶだけ。

### 関数呼び出しの解析

`primary()` で識別子の次に `(` が来たら関数呼び出しと判定：

```rust
TokenKind::Ident(name) => {
    self.advance();
    // Function call: ident "(" args ")"
    if self.current().kind == TokenKind::LParen {
        self.advance();
        let mut args = Vec::new();
        if self.current().kind != TokenKind::RParen {
            args.push(self.assign());
            while self.current().kind == TokenKind::Comma {
                self.advance();
                args.push(self.assign());
            }
        }
        self.expect(TokenKind::RParen);
        return Expr::FuncCall { name, args };
    }
    Expr::Var(name)
}
```

**ポイント**:
- 引数の区切りはコンマだが、コンマ演算子と混同しないよう、各引数は `assign()` レベルで解析する（`expr()` ではなく）
- Step 3.1 では引数は常に空だが、Step 3.2 で引数ありに対応するため、先に構造を用意している
- ローカル変数リストは `Function` 構造体に格納される

## 3. コード生成の変更

### 複数関数の生成

```rust
pub fn generate(&mut self, functions: &[Function]) -> String {
    for func in functions {
        self.gen_function(func);
    }
    self.output.clone()
}
```

各関数に対して独立にコード生成を行う。

### 関数ごとの状態リセット

```rust
fn gen_function(&mut self, func: &Function) {
    self.current_func_name = func.name.clone();
    self.stack_depth = 0;
    self.locals.clear();
    self.goto_labels.clear();
    // ...ローカル変数の割り当て...
    // ...プロローグ生成...
    // ...本体生成...
    // ...エピローグ生成...
}
```

重要な変更点：
- `current_func_name`: リターンラベル `.Lreturn.{name}` を動的に生成
- `stack_depth`: push/pop を追跡してスタックアライメントを管理

### 動的リターンラベル

```rust
Stmt::Return(expr) => {
    self.gen_expr(expr);
    let func_name = self.current_func_name.clone();
    self.emit(&format!("  jmp .Lreturn.{}", func_name));
}
```

これまで `.Lreturn.main` にハードコードされていたが、
各関数ごとに `.Lreturn.ret3`, `.Lreturn.main` のように固有ラベルを持つ。

### スタックアライメントと `call` 命令

#### System V AMD64 ABI のアライメント要件

`call` 命令を実行する時点で、**スタックポインタ `%rsp` が16バイト境界に揃っている**必要がある。

#### なぜアライメントが崩れるのか

式のコード生成はスタックマシン方式で、二項演算の際に `push %rax` を行う。
例えば `ret3() + ret5()` の評価：

```
1. gen_expr(rhs = ret5())   ← call ret5 ← stack aligned ✓
2. push %rax                ← stack は 8 バイトずれる！
3. gen_expr(lhs = ret3())   ← call ret3 ← stack misaligned ✗
```

#### 解決策: stack_depth トラッキング

push/pop のカウントを `stack_depth` フィールドで追跡する：

```rust
fn push(&mut self) {
    self.emit("  push %rax");
    self.stack_depth += 1;
}

fn pop(&mut self, reg: &str) {
    self.emit(&format!("  pop {}", reg));
    self.stack_depth -= 1;
}
```

`call` の前に `stack_depth` が奇数なら、スタックが 8 バイトずれている。
その場合 `sub $8, %rsp` で調整し、`call` 後に `add $8, %rsp` で復元：

```rust
Expr::FuncCall { name, args: _ } => {
    let needs_align = self.stack_depth % 2 != 0;
    if needs_align {
        self.emit("  sub $8, %rsp");   // 8バイト追加して16境界に揃える
    }
    self.emit(&format!("  call {}", name));
    if needs_align {
        self.emit("  add $8, %rsp");   // 調整を元に戻す
    }
}
```

#### なぜ stack_depth % 2 == 1 が misaligned なのか

関数のプロローグで：
1. `call` がリターンアドレスを push（-8）
2. `push %rbp`（-8）→ ここで16バイト境界に復帰
3. `sub $N, %rsp` で N は16の倍数

この時点でスタックは16バイト整列。`push %rax` を1回行うと 8 バイトずれる。
つまり奇数回の push = misaligned。

## 4. 具体例

入力: `int ret3() { return 3; } int main() { return ret3(); }`

```asm
  .globl ret3
ret3:
  push %rbp
  mov %rsp, %rbp
  # return 3
  mov $3, %rax
  jmp .Lreturn.ret3
  mov $0, %rax
.Lreturn.ret3:
  mov %rbp, %rsp
  pop %rbp
  ret

  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  # return ret3()
  call ret3               # ret3 を呼び出し、戻り値は %rax
  jmp .Lreturn.main
  mov $0, %rax
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret
```

### 複数関数の加算: `ret3() + ret5()`

```asm
  # Binary add: ret3() + ret5()
  # Evaluate rhs: ret5()
  call ret5                # %rax = 5
  push %rax                # stack_depth = 1 (misaligned)
  # Evaluate lhs: ret3()
  sub $8, %rsp             # realign stack
  call ret3                # %rax = 3
  add $8, %rsp             # undo alignment
  pop %rdi                 # %rdi = 5, stack_depth = 0
  add %rdi, %rax           # %rax = 3 + 5 = 8
```

## 5. 設計判断

### なぜ locals を Function に移したか

- **データの局所性**: 各関数のローカル変数は、その関数の一部として自然にまとまる
- **複数関数対応**: `parser.get_locals()` では最後にパースした関数の変数しか取れない
- **結合度の低減**: main.rs が parser と codegen の間でデータを受け渡す必要がなくなった

### なぜ引数パースを先に実装したか

Step 3.1 では引数なしだが、`primary()` の関数呼び出しパースで引数リストの構造を実装した。
これは Step 3.2（引数付き関数）でパーサー側の変更が不要になるため。
YAGNI の原則に反するように見えるが、コンマ区切りの引数パースは関数呼び出しの本質的な部分であり、
不完全な実装を後で修正するよりもここで正しく実装する方が合理的。
