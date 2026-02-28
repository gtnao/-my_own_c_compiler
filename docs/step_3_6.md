# Step 3.6: グローバル変数

## 概要

関数の外で宣言される**グローバル変数**を実装する。
グローバル変数は全ての関数からアクセスでき、プログラムの実行期間中存在する。

```c
int g;
void set(int v) { g = v; }
int main() { set(2); return g; }  // => 2
```

## グローバル変数 vs ローカル変数

| 特性 | ローカル変数 | グローバル変数 |
|------|------------|--------------|
| 宣言場所 | 関数内 | 関数の外（トップレベル） |
| 寿命 | 関数呼び出し中のみ | プログラム全体 |
| メモリ | スタック (`-N(%rbp)`) | データセクション (`.bss`) |
| アクセス | `%rbp` 相対 | RIP 相対 (`name(%rip)`) |
| 初期値 | 未定義 | 0（BSS セクション） |

## 実装

### 1. AST の変更

`Program` 構造体でグローバル変数と関数を管理：

```rust
pub struct Program {
    pub globals: Vec<String>,       // グローバル変数名のリスト
    pub functions: Vec<Function>,   // 関数定義のリスト
}
```

### 2. パーサーの変更

#### トップレベルの判定

`type ident` の後に `(` が来れば関数、そうでなければグローバル変数：

```rust
fn is_function(&self) -> bool {
    // type ident "(" → function/prototype
    if self.tokens[self.pos].kind == TokenKind::Int
        || self.tokens[self.pos].kind == TokenKind::Void
    {
        if let TokenKind::Ident(_) = &self.tokens[self.pos + 1].kind {
            return self.tokens[self.pos + 2].kind == TokenKind::LParen;
        }
    }
    false
}
```

#### グローバル変数のパース

```rust
fn global_var(&mut self) {
    self.expect(TokenKind::Int);
    let name = /* 識別子を読む */;
    self.expect(TokenKind::Semicolon);
    self.globals.push(name);
}
```

#### 変数参照の解決

`resolve_var()` は、ローカルスコープで見つからなければ、名前をそのまま返す。
コード生成側でグローバル変数かどうかを判定する。

### 3. コード生成の変更

#### `.comm` ディレクティブ

グローバル変数は `.comm` ディレクティブで BSS セクションに配置：

```rust
for name in &program.globals {
    self.emit(&format!("  .comm {}, 8, 8", name));
}
```

`.comm symbol, size, alignment` の意味：
- `symbol`: 変数名（リンカが参照するシンボル）
- `size`: バイト数（8 = 64ビット整数）
- `alignment`: アライメント（8バイト境界）

BSS セクションの特徴：
- **実行ファイルに空間を消費しない**: 実行時にゼロで初期化される
- 初期化されていないグローバル変数に最適

#### RIP 相対アドレッシング

x86-64 では、グローバル変数への **RIP 相対アドレッシング** を使用する：

```asm
mov g(%rip), %rax       # グローバル変数 g を読む
mov %rax, g(%rip)       # グローバル変数 g に書く
```

#### なぜ RIP 相対か

x86-64 の Position-Independent Code (PIC) では、データのアドレスは
実行時に決まる。RIP 相対アドレッシングは、現在の命令ポインタ（`%rip`）
からの相対オフセットでデータを参照する。

これにより：
1. 位置独立コードが自然に生成される
2. コードがメモリのどこにロードされても動作する
3. ASLR (Address Space Layout Randomization) と互換

#### ローカル vs グローバルの判定

コード生成時に、変数名が `globals` セットに含まれるかで判定：

```rust
fn emit_load_var(&mut self, name: &str) {
    if self.globals.contains(name) {
        self.emit(&format!("  mov {}(%rip), %rax", name));
    } else {
        let offset = self.locals[name];
        self.emit(&format!("  mov -{}(%rbp), %rax", offset));
    }
}

fn emit_store_var(&mut self, name: &str) {
    if self.globals.contains(name) {
        self.emit(&format!("  mov %rax, {}(%rip)", name));
    } else {
        let offset = self.locals[name];
        self.emit(&format!("  mov %rax, -{}(%rbp)", offset));
    }
}
```

## 具体例

入力: `int g; void set(int v) { g = v; } int main() { set(2); return g; }`

```asm
  .comm g, 8, 8                # BSS: g に 8 バイト確保

  .globl set
set:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp
  mov %rdi, -8(%rbp)           # パラメータ v をスタックに保存

  # g = v;
  mov -8(%rbp), %rax           # v を読む
  mov %rax, g(%rip)            # g に RIP 相対で書き込み

  mov $0, %rax
.Lreturn.set:
  mov %rbp, %rsp
  pop %rbp
  ret

  .globl main
main:
  push %rbp
  mov %rsp, %rbp

  # set(2);
  mov $2, %rax
  push %rax
  pop %rdi                     # 引数 → %rdi
  call set

  # return g;
  mov g(%rip), %rax            # g を RIP 相対で読み取り → 2
  jmp .Lreturn.main

  mov $0, %rax
.Lreturn.main:
  mov %rbp, %rsp
  pop %rbp
  ret
```

### ローカル変数によるシャドウイング

```c
int g;
int main() { int g = 3; return g; }  // => 3
```

ローカル変数 `g` がグローバル変数 `g` をシャドウする。
パーサーのスコープ解決により、`return g;` はローカルの `g` を参照する。

## Phase 3 完了

このステップで Phase 3（関数）の全6ステップが完了。

| 機能 | ステップ |
|------|---------|
| 関数呼び出し（引数なし）・複数関数定義 | 3.1 |
| 関数引数（最大6個、System V AMD64 ABI） | 3.2 |
| スタック経由の引数（7個以上） | 3.3 |
| 前方宣言と void 関数 | 3.4 |
| ブロックスコープとシャドウイング | 3.5 |
| グローバル変数 | 3.6 |

テスト数: 126（統合テスト）+ 16（ユニットテスト）= 142
