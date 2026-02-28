# Step 2.14: do-while文、switch/case/default、break文

## 概要

3つの制御構文を追加する：
1. **do-while文**: 本体を先に1回実行してから条件を判定するループ
2. **switch文**: 値に基づく多分岐
3. **break文**: ループやswitch文を途中で抜ける

## 1. do-while文

### while との違い

```c
// while: 条件 → 本体（0回以上）
while (cond) body;

// do-while: 本体 → 条件（1回以上）
do body while (cond);
```

do-while は本体を**必ず1回は実行する**点が while と異なる。

### AST

```rust
Stmt::DoWhile {
    body: Box<Stmt>,
    cond: Expr,
}
```

### パーサー

```rust
TokenKind::Do => {
    self.advance();
    let body = self.stmt();                    // 本体をパース
    self.expect(TokenKind::While);             // "while" を消費
    self.expect(TokenKind::LParen);
    let cond = self.expr();                    // 条件式をパース
    self.expect(TokenKind::RParen);
    self.expect(TokenKind::Semicolon);         // do-while は ";" が必要
    Stmt::DoWhile { body, cond }
}
```

### コード生成

```rust
Stmt::DoWhile { body, cond } => {
    let begin_label = self.new_label();
    let end_label = self.new_label();

    self.break_labels.push(end_label.clone());
    self.emit(&format!("{}:", begin_label));   // ループ開始
    self.gen_stmt(body);                        // 本体（条件前に実行）
    self.gen_expr(cond);                        // 条件を評価
    self.emit("  cmp $0, %rax");
    self.emit(&format!("  jne {}", begin_label)); // 非0ならループ先頭へ
    self.emit(&format!("{}:", end_label));       // ループ終了
    self.break_labels.pop();
}
```

while との違い：条件チェックが **jne**（非0なら先頭へ戻る）で、ループ開始前の条件チェックがない。

### 具体例

入力: `int main() { int i = 0; do { i++; } while (i < 5); return i; }`

```asm
.L0:                          # begin
  # i++
  mov -8(%rbp), %rax
  mov %rax, %rdi
  add $1, %rdi
  mov %rdi, -8(%rbp)

  # cond: i < 5
  mov $5, %rax
  push %rax
  mov -8(%rbp), %rax
  pop %rdi
  cmp %rdi, %rax
  setl %al
  movzb %al, %rax

  cmp $0, %rax
  jne .L0                     # i < 5 が真ならループ
.L1:                          # end
```

## 2. switch文

### 概要

式の値に応じて複数の分岐先にジャンプする。
if-else if チェーンの効率的な代替。

```c
switch (expr) {
    case 1: stmts; break;
    case 2: stmts; break;
    default: stmts; break;
}
```

### フォールスルー

C言語のswitch文は **break がないと次のcaseに落ちる**（フォールスルー）。
これは意図的な動作だが、バグの原因になりやすい。

```c
switch (a) {
    case 1: r = 10;        // break なし → case 2 に落ちる
    case 2: r = 20; break; // ここで抜ける
}
// a==1 の場合: r = 10 → r = 20 → break → r は 20
// a==2 の場合: r = 20 → break → r は 20
```

### AST

```rust
Stmt::Switch {
    cond: Expr,
    cases: Vec<(i64, Vec<Stmt>)>,   // (定数値, 文のリスト)
    default: Option<Vec<Stmt>>,     // default節（省略可能）
}
```

### パーサー

switch文のパースは複雑：

```rust
TokenKind::Switch => {
    self.advance();
    self.expect(TokenKind::LParen);
    let cond = self.expr();
    self.expect(TokenKind::RParen);
    self.expect(TokenKind::LBrace);

    let mut cases = Vec::new();
    let mut default = None;

    while self.current().kind != TokenKind::RBrace {
        if self.current().kind == TokenKind::Case {
            self.advance();
            let val = /* 整数定数を読む */;
            self.advance();
            self.expect(TokenKind::Colon);

            // case/default/} が来るまで文を集める
            let mut stmts = Vec::new();
            while self.current().kind != TokenKind::Case
                && self.current().kind != TokenKind::Default
                && self.current().kind != TokenKind::RBrace
            {
                stmts.push(self.stmt());
            }
            cases.push((val, stmts));
        } else if self.current().kind == TokenKind::Default {
            // 同様にdefault節をパース
        }
    }
    self.expect(TokenKind::RBrace);
    Stmt::Switch { cond, cases, default }
}
```

### コード生成

switch のコード生成は2パスで行う：

**パス1**: 条件式を評価し、各caseの値と比較 → 一致するcaseラベルにジャンプ

**パス2**: 各caseの本体を順番に配置

```rust
Stmt::Switch { cond, cases, default } => {
    let end_label = self.new_label();
    self.break_labels.push(end_label.clone());

    // 条件式を評価
    self.gen_expr(cond);

    // パス1: 比較とジャンプテーブル
    let mut case_labels = Vec::new();
    for (val, _) in cases {
        let label = self.new_label();
        self.emit(&format!("  cmp ${}, %rax", val));
        self.emit(&format!("  je {}", label));
        case_labels.push(label);
    }

    // どのcaseにも一致しない → default or end
    if default.is_some() {
        let default_label = self.new_label();
        self.emit(&format!("  jmp {}", default_label));
    } else {
        self.emit(&format!("  jmp {}", end_label));
    }

    // パス2: 各caseの本体
    for (i, (_, stmts)) in cases.iter().enumerate() {
        self.emit(&format!("{}:", case_labels[i]));
        for s in stmts {
            self.gen_stmt(s);
        }
    }

    // default の本体
    // ...

    self.emit(&format!("{}:", end_label));
    self.break_labels.pop();
}
```

### アセンブリの構造

入力: `switch (a) { case 1: return 10; case 2: return 20; default: return 30; }`

```asm
  # 条件式 a を評価
  mov -8(%rbp), %rax

  # 比較テーブル
  cmp $1, %rax
  je .Lcase1            # a == 1 → case 1 へ
  cmp $2, %rax
  je .Lcase2            # a == 2 → case 2 へ
  jmp .Ldefault         # どれにも一致しない → default へ

  # case 本体（フォールスルー順に配置）
.Lcase1:
  mov $10, %rax
  jmp .Lreturn.main     # return 10

.Lcase2:
  mov $20, %rax
  jmp .Lreturn.main     # return 20

.Ldefault:
  mov $30, %rax
  jmp .Lreturn.main     # return 30

.Lend:                   # switch 終了（break のジャンプ先）
```

## 3. break文

### 概要

最も内側のループ（while/for/do-while）またはswitch文を途中で抜ける。

```c
while (1) {
    if (cond) break;   // while を抜ける
}
```

### AST

```rust
Stmt::Break
```

### break ラベルスタック

break のジャンプ先は、最も内側のループ/switchの終了ラベル。
ネストしたループに対応するため、**スタック**で管理する：

```rust
pub struct Codegen {
    // ...
    break_labels: Vec<String>,  // break 用のラベルスタック
}
```

ループ/switch に入る時にラベルを push、出る時に pop：

```rust
// while の例
let end_label = self.new_label();
self.break_labels.push(end_label.clone());  // push
// ... ループ本体 ...
self.emit(&format!("{}:", end_label));
self.break_labels.pop();                     // pop
```

break文のコード生成はスタックトップのラベルにジャンプするだけ：

```rust
Stmt::Break => {
    if let Some(label) = self.break_labels.last() {
        self.emit(&format!("  jmp {}", label));
    }
}
```

### ネスト例

```c
while (1) {                    // break_labels: [.L1]
    while (1) {                // break_labels: [.L1, .L3]
        break;                 // → jmp .L3 (内側を抜ける)
    }                          // break_labels: [.L1]
    break;                     // → jmp .L1 (外側を抜ける)
}                              // break_labels: []
```
