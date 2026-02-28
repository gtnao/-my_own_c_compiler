# Step 2.15: continue文、goto文、ラベル

## 概要

3つの制御構文を追加する：
1. **continue文**: ループの残りをスキップして次のイテレーションへ
2. **goto文**: 指定したラベルに無条件ジャンプ
3. **ラベル**: goto のジャンプ先を定義

## 1. continue文

### breakとの違い

```c
for (int i = 0; i < 10; i++) {
    if (cond) break;      // ループ全体を抜ける → end ラベル
    if (cond) continue;   // 残りをスキップ → inc ラベル（i++ の前）
}
```

| 文 | ジャンプ先 | ループへの影響 |
|----|-----------|---------------|
| break | ループの**後**（終了ラベル） | ループ終了 |
| continue | ループの**先頭/インクリメント** | 次のイテレーション |

### continue のジャンプ先はループ種類で異なる

| ループ | continue のジャンプ先 |
|--------|---------------------|
| while | 条件判定（ループ先頭）に戻る |
| for | インクリメント部分（`i++`）の直前にジャンプ |
| do-while | 条件判定の直前にジャンプ |

**for文で最も重要**: continue はインクリメントをスキップしない。
これが while との本質的な違い。

```c
// for: continue しても i++ は実行される
for (int i = 0; i < 10; i++) {
    if (i % 2 == 0) continue;  // → i++ にジャンプ
    s += i;
}

// while: continue すると i++ 相当も手動で書かないとスキップされる
int i = 0;
while (i < 10) {
    if (i % 2 == 0) { i++; continue; }  // 手動で i++ が必要
    s += i;
    i++;
}
```

### continue ラベルスタック

break と同様にスタックで管理：

```rust
pub struct Codegen {
    break_labels: Vec<String>,     // break 用
    continue_labels: Vec<String>,  // continue 用
}
```

### while での配置

```rust
Stmt::While { cond, body } => {
    let begin_label = self.new_label();
    let end_label = self.new_label();

    self.break_labels.push(end_label.clone());
    self.continue_labels.push(begin_label.clone());  // ← ループ先頭

    self.emit(&format!("{}:", begin_label));     // continue はここにジャンプ
    // ... 条件 → body ...
    self.emit(&format!("{}:", end_label));       // break はここにジャンプ

    self.continue_labels.pop();
    self.break_labels.pop();
}
```

### for での配置

```rust
Stmt::For { init, cond, inc, body } => {
    let begin_label = self.new_label();
    let continue_label = self.new_label();   // ← inc の直前
    let end_label = self.new_label();

    // ... init → begin: → cond → body ...

    self.emit(&format!("{}:", continue_label));  // continue はここにジャンプ
    if let Some(inc_expr) = inc {
        self.gen_expr(inc_expr);                 // インクリメント実行
    }
    self.emit(&format!("  jmp {}", begin_label));
    // ...
}
```

### 具体例

入力: `for (i = 0; i < 10; i++) { if (i % 2 == 0) continue; s += i; }`

```asm
  # init: i = 0
  ...

.L0:                          # begin
  # cond: i < 10
  ...
  je .L2                       # end (false → ループ終了)

  # body
  # if (i % 2 == 0) continue;
  ...
  cmp $0, %rax
  je .L3                       # else ラベル
  jmp .L1                      # continue → inc へ ★
  jmp .L4                      # end of if
.L3:
.L4:

  # s += i;
  ...

.L1:                          # continue ラベル ★
  # inc: i++
  ...
  jmp .L0                     # begin に戻る

.L2:                          # end (break ラベル)
```

## 2. goto文とラベル

### 概要

`goto` は指定したラベルに無条件でジャンプする。
ラベルは `名前:` の形式で文の前に付ける。

```c
goto end;           // end ラベルへジャンプ
// ... (実行されない)
end:                // ラベル定義
  return 2;
```

### goto の用途

goto は一般的に避けられるが、正当な用途がある：
1. **多重ループからの脱出**: C言語にはラベル付きbreakがないため
2. **エラー処理のクリーンアップ**: リソース解放のパターン
3. **状態機械の実装**

```c
// 多重ループ脱出
for (...) {
    for (...) {
        if (found) goto done;
    }
}
done:

// エラー処理
if (alloc1() == NULL) goto error;
if (alloc2() == NULL) goto cleanup1;
return 0;
cleanup1: free(p1);
error: return -1;
```

### AST

```rust
Stmt::Goto(String)        // goto label_name;
Stmt::Label {
    name: String,          // ラベル名
    stmt: Box<Stmt>,       // ラベルの後に続く文
}
```

ラベルは「文を修飾する」形式。`label: stmt` はそれ自体が一つの文。

### パーサー

#### goto

```rust
TokenKind::Goto => {
    self.advance();
    let name = /* 識別子を読む */;
    self.advance();
    self.expect(TokenKind::Semicolon);
    Stmt::Goto(name)
}
```

#### ラベル

ラベルは `ident ":"` の形。`_` のデフォルト分岐で先読みして判別：

```rust
_ => {
    // ident の次が ":" ならラベル
    if let TokenKind::Ident(name) = &self.current().kind {
        if self.tokens[self.pos + 1].kind == TokenKind::Colon {
            let name = name.clone();
            self.advance();  // ident
            self.advance();  // :
            let stmt = self.stmt();  // ラベルの後の文
            return Stmt::Label { name, stmt: Box::new(stmt) };
        }
    }

    // ラベルでなければ通常の式文
    let expr = self.expr();
    self.expect(TokenKind::Semicolon);
    Stmt::ExprStmt(expr)
}
```

### コード生成

goto/ラベルのコード生成では、ユーザー定義のラベル名をアセンブリのラベルに変換する。
同じ名前のgotoとラベルが同じアセンブリラベルを指すように、`HashMap` で管理する：

```rust
goto_labels: HashMap<String, String>  // ユーザー名 → アセンブリラベル

fn get_or_create_goto_label(&mut self, name: &str) -> String {
    if let Some(label) = self.goto_labels.get(name) {
        label.clone()
    } else {
        let label = self.new_label();
        self.goto_labels.insert(name.to_string(), label.clone());
        label
    }
}
```

初回参照時にラベルを生成し、2回目以降は同じラベルを返す。
これにより、goto が先でラベルが後（前方参照）でも正しく動作する。

```rust
Stmt::Goto(name) => {
    let label = self.get_or_create_goto_label(name);
    self.emit(&format!("  jmp {}", label));
}
Stmt::Label { name, stmt } => {
    let label = self.get_or_create_goto_label(name);
    self.emit(&format!("{}:", label));
    self.gen_stmt(stmt);
}
```

### 具体例

入力: `int main() { goto end; return 1; end: return 2; }`

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp

  # goto end;
  jmp .L0                     # end ラベルへジャンプ

  # return 1;  (実行されない)
  mov $1, %rax
  jmp .Lreturn.main

  # end: return 2;
.L0:                          # end ラベル
  mov $2, %rax
  jmp .Lreturn.main

  mov $0, %rax
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret                         # 終了コード 2
```

## Phase 2 完了

このステップで Phase 2（文と制御フロー）の全15ステップが完了。
サポートする制御構文：

| 構文 | ステップ |
|------|---------|
| return文 | 2.1 |
| 式文 | 2.1 |
| ローカル変数 | 2.2, 2.3 |
| if/else | 2.4 |
| while | 2.5 |
| for | 2.6 |
| ブロック文 | 2.7 |
| 変数初期化 | 2.8 |
| 複合代入 (+=, -=, etc.) | 2.9 |
| インクリメント/デクリメント | 2.10 |
| 論理演算子 (&&, \|\|, !) | 2.11 |
| ビット演算子 (&, \|, ^, ~, <<, >>) | 2.12 |
| コンマ演算子、三項演算子 | 2.13 |
| do-while, switch/case/default, break | 2.14 |
| continue, goto, ラベル | 2.15 |

テスト数: 103（統合テスト） + 14（ユニットテスト）= 117
