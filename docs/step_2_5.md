# Step 2.5: while文

## 概要

`while (条件) 文` をサポートする。
ループはアセンブリレベルでは**条件チェック → 本体実行 → 先頭に戻る**のパターンで実現する。

## AST

```rust
Stmt::While {
    cond: Expr,        // ループ条件
    body: Box<Stmt>,   // ループ本体（1文）
}
```

## パーサーの文法

```
stmt = "while" "(" expr ")" stmt
     | ...
```

## コード生成：ループのアセンブリパターン

### while のフロー

```c
while (条件) 本体;
```

↓

```asm
.Lbegin:                  # ← ループ先頭
  # 条件式を評価 → %rax
  cmp $0, %rax
  je .Lend                # 偽(0)ならループ脱出
  # --- 本体 ---
  jmp .Lbegin             # ループ先頭に戻る
.Lend:                    # ← ループ終了
```

### 実行フロー図

```
                ┌───────────────┐
                ▼               │
            条件評価            │
            %rax = ?            │
                │               │
         ┌──── cmp $0, %rax ───┐│
         │  偽(=0)     真(≠0)  ││
         │              │      ││
         ▼              ▼      ││
      .Lend          本体実行   ││
     (脱出)             │      ││
                        └──────┘│
                   jmp .Lbegin ─┘
```

### if との違い

| | if | while |
|---|---|---|
| ジャンプ先 | endラベル | beginラベル（先頭に戻る） |
| 実行回数 | 最大1回 | 0回以上 |
| 構造 | 分岐 | ループ |

if は「一度だけ判断して分岐」、while は「毎回判断して繰り返し」。
アセンブリレベルでは、whileは末尾に `jmp .Lbegin` があるだけの違い。

### 具体例

入力: `int main() { int i = 0; while (i < 10) i = i + 1; return i; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # int i = 0
  mov $0, %rax
  mov %rax, -8(%rbp)

.L2:                          # while begin
  # 条件: i < 10
  mov $10, %rax               # rhs: 10
  push %rax
  mov -8(%rbp), %rax          # lhs: i
  pop %rdi
  cmp %rdi, %rax              # i < 10 ?
  setl %al
  movzb %al, %rax

  cmp $0, %rax                # 結果が0（偽）なら脱出
  je .L3

  # 本体: i = i + 1
  mov $1, %rax
  push %rax
  mov -8(%rbp), %rax
  pop %rdi
  add %rdi, %rax
  mov %rax, -8(%rbp)

  jmp .L2                     # ループ先頭に戻る

.L3:                          # while end
  # return i
  mov -8(%rbp), %rax
  jmp .Lreturn.main
```

ループは10回実行される：
- i=0: 0<10=true → i=1
- i=1: 1<10=true → i=2
- ...
- i=9: 9<10=true → i=10
- i=10: 10<10=false → ループ脱出 → return 10
