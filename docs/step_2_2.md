# Step 2.2: ローカル変数（単一文字）

## 概要

`int a; a = 3; return a;` のように、ローカル変数の宣言・代入・参照をサポートする。
変数はスタック上に配置され、`%rbp` からの相対アドレスでアクセスする。

## 新しいトークン

| トークン | 例 | 用途 |
|----------|-----|------|
| `Eq` | `=` | 代入演算子 |

`==`（等価比較）と `=`（代入）の区別が重要。レキサーは2文字トークンを先にチェックするため、
`==` を見たら `EqEq` に、`=` 単体なら `Eq` になる。

```
入力: "a == b"  →  Ident("a")  EqEq  Ident("b")
入力: "a = b"   →  Ident("a")  Eq    Ident("b")
```

## AST の変更

### 新しい式ノード

```rust
Expr::Var(String)                // 変数参照: a → Var("a")
Expr::Assign { lhs, rhs }       // 代入: a = 3 → Assign { lhs: Var("a"), rhs: Num(3) }
```

### 新しい文ノード

```rust
Stmt::VarDecl { name, init }    // 変数宣言: int a; or int a = 5;
```

`VarDecl` は宣言時に初期化式を持てる（`init: Option<Expr>`）。

## パーサーの文法変更

```
stmt    = "return" expr ";"
        | "int" ident ("=" expr)? ";"     ← NEW
        | expr ";"
expr    = assign                          ← Changed (was: equality)
assign  = equality ("=" assign)?          ← NEW
primary = num | ident | "(" expr ")"      ← ident追加
```

### 代入の右結合性

代入演算子 `=` は**右結合**。つまり `a = b = 3` は `a = (b = 3)` と解釈される。

```rust
// assign = equality ("=" assign)?
fn assign(&mut self) -> Expr {
    let node = self.equality();
    if self.current().kind == TokenKind::Eq {
        self.advance();
        let rhs = self.assign();  // 再帰呼び出し → 右結合
        return Expr::Assign { lhs: Box::new(node), rhs: Box::new(rhs) };
    }
    node
}
```

左結合（`+`, `-` 等）は `loop` で実装するが、右結合は再帰で実装する。
この違いは重要で、間違えると `a = b = 3` が `(a = b) = 3` になってしまう。

### ローカル変数の追跡

パーサーが `locals: Vec<String>` を持ち、`int a;` を見るたびに変数名を記録する。
この一覧はコード生成時にスタック上のオフセットを決めるために使われる。

## コード生成：スタック上の変数配置

### スタックフレームのレイアウト

```
アドレス高い側
┌──────────────┐
│  return addr │  ← call命令が積む
├──────────────┤
│  saved %rbp  │  ← push %rbp で保存
├──────────────┤ ← %rbp がここを指す
│  変数 a      │  -8(%rbp)
├──────────────┤
│  変数 b      │  -16(%rbp)
├──────────────┤ ← %rsp がここを指す
│    ...       │
アドレス低い側
```

### 変数のオフセット計算

```rust
// パーサーが記録した変数の順番に基づいてオフセットを割り当て
// locals = ["a", "b"] の場合:
//   a → -8(%rbp)    (index 0 → (0+1)*8 = 8)
//   b → -16(%rbp)   (index 1 → (1+1)*8 = 16)
for (i, name) in local_vars.iter().enumerate() {
    self.locals.insert(name.clone(), (i + 1) * 8);
}
```

### 16バイトアライメント

```rust
self.stack_size = local_vars.len() * 8;
if self.stack_size % 16 != 0 {
    self.stack_size = (self.stack_size + 15) & !15;
}
```

System V AMD64 ABI では、関数呼び出し時に `%rsp` が16バイト境界に揃っている必要がある。
`push %rbp` で8バイトずれるので、`sub` で調整するサイズは16の倍数にする。

例：変数1つ（8バイト）→ `sub $16, %rsp`（16に切り上げ）

### 生成されるアセンブリ

入力: `int main() { int a; a = 3; return a; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp          ← 変数1個(8B)→16Bに切り上げ

  # a = 3 (Assign)
  mov $3, %rax           ← rhs: Num(3) を評価
  mov %rax, -8(%rbp)     ← lhs: Var("a") に格納

  # return a
  mov -8(%rbp), %rax     ← Var("a") を読み出し
  jmp .Lreturn.main

  mov $0, %rax
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret
```

### 変数のアクセスパターン

**読み出し（Var）**:
```asm
mov -8(%rbp), %rax       # 変数の値を%raxに読み込む
```

**書き込み（Assign）**:
```asm
mov $3, %rax             # 右辺を評価（結果は%rax）
mov %rax, -8(%rbp)       # %raxの値を変数に書き込む
```

代入式は**値を返す**。`a = 3` の結果は `3` であり、`%rax` にその値が残る。
これにより `b = a = 3` のような連鎖代入が自然に動作する：
1. `a = 3` → `%rax = 3`、`a` に `3` を格納
2. `b = (結果)` → `%rax` の `3` を `b` に格納
