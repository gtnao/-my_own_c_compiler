# Step 2.1: return文と式文

## 概要

Phase 1 では入力が裸の式（`5+3`）だったのを、C言語の関数構文 `int main() { return 42; }` に変更する。
これにより「文（Statement）」という概念が登場し、コンパイラの構造が「式だけ処理する」から「文の列を処理する」に進化する。

## 何が変わったか

### 入力形式の変化

```
Phase 1:  5+3
Phase 2:  int main() { return 5+3; }
```

### 新しいトークン

| トークン | 例 | 用途 |
|----------|-----|------|
| `Ident(String)` | `main`, `foo` | 識別子（関数名・変数名） |
| `Return` | `return` | return キーワード |
| `Int` | `int` | 型キーワード |
| `LBrace` / `RBrace` | `{` / `}` | ブロックの開始/終了 |
| `Semicolon` | `;` | 文の終端 |

### レキサーの変更：識別子とキーワードの区別

```
入力: "int main return foo"
        ↓
Int  Ident("main")  Return  Ident("foo")
```

レキサーはまず `read_ident()` で連続する英数字・アンダースコアを読み取り、
結果の文字列がキーワード（`int`, `return`）に一致すれば対応するトークンに、
そうでなければ `Ident(String)` にする。

```rust
let word = self.read_ident();
let kind = match word.as_str() {
    "return" => TokenKind::Return,
    "int"    => TokenKind::Int,
    _        => TokenKind::Ident(word),
};
```

この「まず識別子として読んでからキーワード判定する」方法は、多くのコンパイラで使われる標準的な手法。
別の方法として「キーワードテーブルを事前に構築する」やり方もあるが、
キーワード数が少ないうちは match で十分。

## AST の変化：式から文へ

### Phase 1 の AST

```
Expr（式だけ）
├── Num(42)
├── BinOp { op, lhs, rhs }
└── UnaryOp { op, operand }
```

### Phase 2 の AST

```
Function { name, body: Vec<Stmt> }
    │
    ├── Stmt::Return(Expr)      ← return文
    └── Stmt::ExprStmt(Expr)    ← 式文（副作用のために式を実行）
```

**式（Expression）** と **文（Statement）** の違い：

- **式**：値を生成する。`1+2` は `3` という値を生成する
- **文**：動作を実行する。`return 42;` は「42を返す」という動作

C言語では式にセミコロンを付けると式文になる：
```c
42;        // 式文: 42を計算して結果を捨てる
return 42; // return文: 42を計算してmainの戻り値にする
```

## パーサーの文法

```
program  = function
function = "int" ident "(" ")" "{" stmt* "}"
stmt     = "return" expr ";"
         | expr ";"
```

パースの流れ（`int main() { 1; return 2; }` の場合）：

```
1. "int"    → expect(Int)           ← 関数の型
2. "main"   → name = "main"         ← 関数名
3. "(" ")"  → expect(LParen, RParen)← 引数リスト（今は空）
4. "{"      → expect(LBrace)        ← 本体開始
5. "1" ";"  → ExprStmt(Num(1))      ← 式文
6. "return" "2" ";" → Return(Num(2))← return文
7. "}"      → expect(RBrace)        ← 本体終了
```

## コード生成の変化

### Phase 1 のコード生成

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  mov $42, %rax        ← 式の結果がそのまま戻り値
  mov %rbp, %rsp
  pop %rbp
  ret
```

### Phase 2 のコード生成

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  mov $1, %rax         ← ExprStmt(1): 結果は捨てられる
  mov $2, %rax         ← Return(2): 結果を%raxに入れて...
  jmp .Lreturn.main    ← ...returnラベルにジャンプ
  mov $0, %rax         ← デフォルト戻り値（到達しない場合）
.Lreturn.main:         ← 全てのreturn文がここに飛ぶ
  mov %rbp, %rsp
  pop %rbp
  ret
```

### return ラベルの仕組み

```
int main() {
    if (...) return 1;   →  jmp .Lreturn.main
    return 2;            →  jmp .Lreturn.main
}
                            .Lreturn.main:
                              mov %rbp, %rsp
                              pop %rbp
                              ret
```

関数内に複数の `return` 文があっても、全て同じ `.Lreturn.main` ラベルにジャンプする。
これにより、エピローグ（スタックの後始末 + `ret`）を1箇所にまとめられる。

もしラベルを使わず各 `return` の後にエピローグを直接書くと：
```asm
  # return 1 のコード
  mov $1, %rax
  mov %rbp, %rsp    ← 重複！
  pop %rbp           ← 重複！
  ret                ← 重複！

  # return 2 のコード
  mov $2, %rax
  mov %rbp, %rsp    ← 重複！
  pop %rbp           ← 重複！
  ret                ← 重複！
```

ラベル方式ならエピローグは1箇所だけで済む。

### `mov $0, %rax`（デフォルト戻り値）

```c
int main() {
    1 + 2;    // 式文のみ、returnなし
}
```

このように `return` 文がないまま関数の末尾に到達した場合、
`mov $0, %rax` が実行されて 0 が返る。
C言語の仕様では `main` 関数が `return` なしで終わった場合、0 を返すと定められている。
