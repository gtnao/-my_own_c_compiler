# Step 2.6: for文

## 概要

`for (init; cond; inc) body` をサポートする。
for文はwhile文の糖衣構文と考えることができ、以下のように等価変換できる：

```c
for (init; cond; inc) body;
// ↓ 等価
init;
while (cond) {
    body;
    inc;
}
```

## AST

```rust
Stmt::For {
    init: Option<Box<Stmt>>,   // 初期化（文: 変数宣言 or 式文）
    cond: Option<Expr>,        // 条件式（省略可 → 無限ループ）
    inc: Option<Expr>,         // 更新式（省略可）
    body: Box<Stmt>,           // ループ本体
}
```

`init` が `Box<Stmt>` なのは、`for (int i = 0; ...)` のように変数宣言を含められるため。
`for (i = 0; ...)` のように式文の場合もある。

## パーサーの文法

```
stmt = "for" "(" (var_decl | expr ";" | ";") expr? ";" expr? ")" stmt
     | ...
```

3つの部分（init, cond, inc）はそれぞれ省略可能：
- `for (;;)` → 無限ループ
- `for (;i<10;)` → init/incなし（whileと同じ）
- `for (i=0;i<10;i=i+1)` → 全て指定

### init の3パターン

```rust
let init = if self.current().kind == TokenKind::Semicolon {
    self.advance();       // ";" だけ → initなし
    None
} else if self.current().kind == TokenKind::Int {
    Some(Box::new(self.var_decl()))  // "int i = 0;" → VarDecl
} else {
    let expr = self.expr();
    self.expect(TokenKind::Semicolon);
    Some(Box::new(Stmt::ExprStmt(expr)))  // "i = 0;" → ExprStmt
};
```

## コード生成

### for のアセンブリパターン

```c
for (init; cond; inc) body;
```

↓

```asm
  # --- init ---
.Lbegin:
  # --- cond ---
  cmp $0, %rax
  je .Lend              # 偽なら脱出
  # --- body ---
  # --- inc ---
  jmp .Lbegin           # 先頭に戻る
.Lend:
```

### while との比較

```
while:                          for:
                                  init
.Lbegin:                        .Lbegin:
  cond                            cond
  cmp $0, %rax                    cmp $0, %rax
  je .Lend                        je .Lend
  body                            body
  jmp .Lbegin                     inc          ← 追加
.Lend:                            jmp .Lbegin
                                .Lend:
```

for はwhile に対して `init` と `inc` が追加されただけ。
init はループの前に1回だけ実行、inc は本体の後に毎回実行される。

### 具体例

入力: `int main() { int s = 0; int i; for (i = 0; i < 10; i = i + 1) s = s + i; return s; }`

```asm
  # int s = 0
  mov $0, %rax
  mov %rax, -8(%rbp)

  # for init: i = 0
  mov $0, %rax
  mov %rax, -16(%rbp)

.L4:                          # for begin
  # cond: i < 10
  mov $10, %rax
  push %rax
  mov -16(%rbp), %rax
  pop %rdi
  cmp %rdi, %rax
  setl %al
  movzb %al, %rax
  cmp $0, %rax
  je .L5                      # 偽ならループ脱出

  # body: s = s + i
  mov -16(%rbp), %rax         # i
  push %rax
  mov -8(%rbp), %rax          # s
  pop %rdi
  add %rdi, %rax              # s + i
  mov %rax, -8(%rbp)          # s = s + i

  # inc: i = i + 1
  mov $1, %rax
  push %rax
  mov -16(%rbp), %rax
  pop %rdi
  add %rdi, %rax
  mov %rax, -16(%rbp)

  jmp .L4                     # ループ先頭へ
.L5:                          # for end
```

このループは `s = 0+1+2+...+9 = 45` を計算する。
