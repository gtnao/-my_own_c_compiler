# Step 2.4: if文

## 概要

`if (条件) 文` および `if (条件) 文 else 文` をサポートする。
条件分岐はアセンブリレベルでは**比較命令 + 条件ジャンプ + ラベル**で実現する。

## 新しいトークン

| トークン | 例 | 用途 |
|----------|-----|------|
| `If` | `if` | if キーワード |
| `Else` | `else` | else キーワード |

## AST の変更

```rust
Stmt::If {
    cond: Expr,                    // 条件式
    then_stmt: Box<Stmt>,          // then節（1つの文）
    else_stmt: Option<Box<Stmt>>,  // else節（省略可能）
}
```

`then_stmt` と `else_stmt` は `Box<Stmt>` で、単一の文を保持する。
ブロック文（`{...}`）は Step 2.7 で追加するが、現時点では1文のみ。

## パーサーの文法

```
stmt = "if" "(" expr ")" stmt ("else" stmt)?
     | ...
```

### ダングリングelse問題

```c
if (a) if (b) x; else y;
```

この `else` は内側の `if (b)` に結びつく（最近接規則）。
現在のパーサーは**再帰的に `stmt()` を呼ぶ**ため、自然にこの規則を満たす：

```
if (a)
    if (b) x; else y;   ← elseは内側のifに属する
```

パーサーが `if (b)` の後の `stmt()` を処理する際、
`else` が続くかチェックする → 見つかる → 内側の `if` の `else_stmt` になる。
外側の `if` に戻った時、次のトークンは `else` ではないので `else_stmt = None`。

## コード生成：条件分岐のパターン

### ラベルの仕組み

コンパイラは一意なラベルを `.L0`, `.L1`, `.L2`, ... と順番に生成する。

```rust
fn new_label(&mut self) -> String {
    let label = format!(".L{}", self.label_count);
    self.label_count += 1;
    label
}
```

### if-else のアセンブリパターン

```c
if (条件) then文; else else文;
```

↓

```asm
  # 条件式を評価 → 結果は%rax
  cmp $0, %rax          # %rax == 0 ?
  je .L0                # 0なら（偽なら）elseへジャンプ
  # --- then文 ---
  jmp .L1               # then実行後、endへジャンプ
.L0:                    # elseラベル
  # --- else文 ---
.L1:                    # endラベル
```

### 実行フロー図

```
条件が真（非0）の場合:
  条件評価 → cmp → je(ジャンプしない) → then文 → jmp .Lend → .Lend

条件が偽（0）の場合:
  条件評価 → cmp → je(ジャンプする) → .Lelse → else文 → .Lend
```

### `cmp $0, %rax` の意味

C言語では「0は偽、0以外は真」。
`cmp $0, %rax` は `%rax` と `0` を比較し、
`je`（Jump if Equal）は `%rax == 0`（偽）の場合にジャンプする。

つまり:
- `%rax` が `0`（偽）→ `je` が発動 → else節へ
- `%rax` が非0（真）→ `je` が発動しない → そのままthen節を実行

### else なしの場合

```c
if (条件) then文;
```

```asm
  cmp $0, %rax
  je .L0                # 偽ならendへ
  # --- then文 ---
  jmp .L1
.L0:                    # elseラベル（何もない）
.L1:                    # endラベル
```

else がなくても同じ構造。`.L0` と `.L1` の間に何もないだけ。
最適化コンパイラなら不要なラベルやジャンプを除去するが、
学習用コンパイラではシンプルさを優先して常に同じパターンを出力する。

### 具体例

入力: `int main() { if (0) return 1; else return 2; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  # if (0)
  mov $0, %rax          # 条件: 0
  cmp $0, %rax          # 0 == 0 → ZF=1
  je .L0                # ZF=1 → ジャンプ！
  # then: return 1（スキップされる）
  mov $1, %rax
  jmp .Lreturn.main
  jmp .L1
.L0:                    # else:
  mov $2, %rax          # return 2
  jmp .Lreturn.main
.L1:
  mov $0, %rax
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret
```
