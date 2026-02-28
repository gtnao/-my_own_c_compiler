# Step 1.4: 乗除算、優先順位、括弧、単項演算子

## 概要

コンパイラの中核アーキテクチャを導入するステップ。`ast.rs` (AST定義)、`parser.rs` (再帰下降パーサー)、`codegen.rs` (コード生成器) を新規作成し、演算子の優先順位、括弧、単項演算子を正しく処理する。

## 入出力

**入力**: `5 + 6 * 7`

**出力**:
```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  mov $7, %rax
  push %rax
  mov $6, %rax
  pop %rdi
  imul %rdi, %rax
  push %rax
  mov $5, %rax
  pop %rdi
  add %rdi, %rax
  mov %rbp, %rsp
  pop %rbp
  ret
```

`5 + (6 * 7) = 47` が正しく計算される。

## アーキテクチャの変更

### コンパイラパイプライン

```
ソースコード → Lexer → トークン列 → Parser → AST → Codegen → アセンブリ
```

Step 1.3 までは `main.rs` がトークン列を直接走査してアセンブリを生成していたが、このステップからはASTを介した2段階の処理になる。

### 新規ファイル

| ファイル | 役割 |
|---------|------|
| `src/ast.rs` | AST (抽象構文木) のノード定義 |
| `src/parser.rs` | 再帰下降パーサー (トークン列 → AST) |
| `src/codegen.rs` | コード生成器 (AST → x86-64 アセンブリ) |

### main.rs の変更

```rust
// Before (Step 1.3): トークン列を直接走査
let tokens = lexer.tokenize();
// ... 手動でトークンを消費してprintln!

// After (Step 1.4): パイプライン
let tokens = lexer.tokenize();
let expr = Parser::new(tokens).parse();      // トークン列 → AST
let output = Codegen::new().generate(&expr); // AST → アセンブリ
print!("{}", output);
```

## ast.rs — 抽象構文木

```rust
pub enum BinOp { Add, Sub, Mul, Div }
pub enum UnaryOp { Neg }

pub enum Expr {
    Num(i64),
    BinOp { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    UnaryOp { op: UnaryOp, operand: Box<Expr> },
}
```

ASTは式 (expression) の木構造。例えば `5 + 6 * 7` は:

```
    Add
   /   \
  5    Mul
      /   \
     6     7
```

`Box<Expr>` を使うのは、Rustでは再帰的な型は固定サイズにする必要があるため。`Box` はヒープアロケーションで間接参照にする。

## parser.rs — 再帰下降パーサー

### 文法規則

演算子の優先順位を文法規則で表現する:

```
expr    = mul ("+" mul | "-" mul)*
mul     = unary ("*" unary | "/" unary)*
unary   = ("+" | "-") unary | primary
primary = num | "(" expr ")"
```

**優先順位** (低い → 高い):
1. `+`, `-` (加減算)
2. `*`, `/` (乗除算)
3. 単項 `+`, `-`
4. 数値リテラル、括弧

### なぜこの文法で優先順位が実現されるのか

`expr` が `mul` を呼び、`mul` が `unary` を呼ぶ。下の規則ほど「結合力が強い」。

`5 + 6 * 7` をパースする場合:
1. `expr` が `mul` を呼ぶ → `mul` は `5` を返す
2. `expr` は `+` を見つける
3. `expr` が再度 `mul` を呼ぶ → `mul` は `6 * 7` を木にして返す
4. `expr` は `Add(5, Mul(6, 7))` を構築

`*` は `mul` の中で処理されるので、`+` より先に結合する。

### 再帰下降パーサーとは

各文法規則に対応する関数を用意し、関数の呼び出しで文法のネストを表現する方式。手書きで書きやすく、エラー報告も柔軟にできる。

```rust
fn expr(&mut self) -> Expr {
    let mut node = self.mul();        // まず mul を評価
    loop {
        match self.current().kind {
            TokenKind::Plus => {
                self.advance();
                let rhs = self.mul(); // 右辺も mul を評価
                node = Expr::BinOp { op: BinOp::Add, lhs: node, rhs };
            }
            // ...
            _ => break,
        }
    }
    node
}
```

## codegen.rs — スタックマシン方式のコード生成

### スタックマシンとは

二項演算の両辺を **スタック** を使って計算する方式:

1. 右辺を評価 → 結果が `%rax` に入る
2. `%rax` をスタックに push
3. 左辺を評価 → 結果が `%rax` に入る
4. スタックから pop して `%rdi` に入れる (= 右辺の値)
5. `%rax` (左辺) と `%rdi` (右辺) で演算

```rust
fn gen_expr(&mut self, expr: &Expr) {
    match expr {
        Expr::BinOp { op, lhs, rhs } => {
            self.gen_expr(rhs);          // rhs → %rax
            self.emit("  push %rax");    // save rhs
            self.gen_expr(lhs);          // lhs → %rax
            self.emit("  pop %rdi");     // rdi = rhs
            // now: %rax = lhs, %rdi = rhs
            match op {
                BinOp::Add => self.emit("  add %rdi, %rax"),
                BinOp::Sub => self.emit("  sub %rdi, %rax"),
                BinOp::Mul => self.emit("  imul %rdi, %rax"),
                BinOp::Div => {
                    self.emit("  cqto");
                    self.emit("  idiv %rdi");
                }
            }
        }
        // ...
    }
}
```

### 新しいアセンブリ命令

| 命令 | 動作 | 使用場面 |
|------|------|---------|
| `push %rbp` | `%rbp` をスタックにpush | 関数プロローグ |
| `pop %rbp` | スタックトップを `%rbp` に pop | 関数エピローグ |
| `mov %rsp, %rbp` | スタックポインタをフレームポインタにコピー | 関数プロローグ |
| `imul %rdi, %rax` | `%rax = %rax * %rdi` (符号付き乗算) | 乗算 |
| `cqto` | `%rax` を `%rdx:%rax` に符号拡張 | 除算の前準備 |
| `idiv %rdi` | `%rdx:%rax / %rdi` → 商=`%rax`, 余り=`%rdx` | 除算 |
| `neg %rax` | `%rax = -%rax` (二の補数の否定) | 単項マイナス |

### 関数プロローグ/エピローグ

```asm
main:
  push %rbp          # 呼び出し元の rbp を保存
  mov %rsp, %rbp     # 現在の rsp を rbp にセット (フレームポインタ)
  ...                 # body
  mov %rbp, %rsp     # rsp を rbp に戻す (ローカル変数の領域を解放)
  pop %rbp           # 呼び出し元の rbp を復元
  ret                 # return
```

今回はローカル変数がないが、スタックマシン方式で `push`/`pop` を使うため、フレームポインタを設定しておく。

### 除算の仕組み

x86-64の `idiv` 命令は特殊:

1. **被除数**: `%rdx:%rax` の128ビット値 (上位64ビット:下位64ビット)
2. **除数**: オペランド (ここでは `%rdi`)
3. **結果**: 商 → `%rax`, 余り → `%rdx`

`cqto` (Convert Quad to Octo) は `%rax` の符号ビットを `%rdx` 全体にコピーする。これにより `%rax` の64ビット値が `%rdx:%rax` の128ビットに正しく符号拡張される。

## テスト

| 入力 | 期待値 | 検証内容 |
|------|--------|---------|
| `5+6*7` | 47 | `*` は `+` より優先 |
| `5*(9-6)` | 15 | 括弧による優先順位変更 |
| `(3+5)/2` | 4 | 括弧と除算 |
| `-10+20` | 10 | 単項マイナス |
| `- -10` | 10 | 二重否定 |
| `- - +10` | 10 | 単項+と-の組み合わせ |
| `2*3` | 6 | 単純な乗算 |
| `9/3` | 3 | 単純な除算 |
| `(2+3)*(5-1)` | 20 | 括弧のネスト |
| `10/2/2+1-1+1` | 3 | 左結合の除算 |

## このステップで学べること

1. **AST (抽象構文木)**: ソースコードの構造を木で表現する
2. **再帰下降パーサー**: 文法規則と関数を1対1で対応させるパーサー手法
3. **演算子の優先順位**: 文法規則の階層で優先順位を自然に表現する
4. **スタックマシン方式**: push/pop で二項演算のオペランドを管理するコード生成手法
5. **x86-64の除算**: `cqto` + `idiv` の仕組み
6. **関数プロローグ/エピローグ**: フレームポインタの設定と復元

## 次のステップ

→ **Step 1.5: 剰余演算子** — `%` 演算子を追加する。
