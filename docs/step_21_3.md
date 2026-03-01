# Step 21.3: PICモードでのstatic変数・関数の可視性修正

## 概要

このステップでは、`-fPIC`（Position Independent Code）モードでコンパイルしたアセンブリを共有ライブラリ（`.so`）としてリンクする際に発生するリロケーションエラーを修正します。

## 問題

### テキストリロケーションエラー

PICモードでコンパイルし、共有ライブラリとしてリンクする際に、45/331のPostgreSQLファイルで以下のようなエラーが発生しました：

```
/usr/bin/ld: pg_pic.o: warning: relocation against `some_variable' in read-only section `.text'
/usr/bin/ld: pg_pic.o: relocation R_X86_64_PC32 against symbol `some_variable' can not be used when making a shared object; recompile with -fPIC
```

### 原因1: `static`変数・関数に`.globl`を使用

C言語の`static`キーワードはファイルスコープ（内部リンケージ）を意味します。共有ライブラリでは、`.globl`として宣言されたシンボルはシンボルインターポジション（symbol interposition）の対象となり、直接的なRIP相対アクセスが禁止されます。

```c
// PostgreSQLのコード例
static int some_counter = 0;     // ファイル内でのみ参照可能
static void helper_func(void) {} // ファイル内でのみ呼び出し可能
```

修正前：
```asm
  .globl some_counter    # 誤り: 外部から見えてしまう
  .comm some_counter, 4, 4

  .globl helper_func     # 誤り: 外部から見えてしまう
helper_func:
```

修正後：
```asm
  .local some_counter    # 正しい: ファイルスコープ
  .comm some_counter, 4, 4

  .local helper_func     # 正しい: ファイルスコープ
helper_func:
```

### 原因2: 非staticグローバル変数への直接アクセス

PICモードでは、外部から可視なシンボル（`.globl`）にアクセスする際、GOT（Global Offset Table）経由のアクセスが必要です。これは、共有ライブラリのシンボルが実行時にインターポジションされる可能性があるためです。

```c
// non-staticグローバル変数
int global_counter = 0;
```

修正前：
```asm
  movl global_counter(%rip), %eax    # 直接アクセス → リロケーションエラー
```

修正後：
```asm
  mov global_counter@GOTPCREL(%rip), %rax  # GOT経由でアドレス取得
  movl (%rax), %eax                         # 間接ロード
```

## シンボルインターポジションとは

共有ライブラリのリンカは、`.globl`シンボルに対して**シンボルインターポジション**を許可します。これは、同じ名前のシンボルが複数の共有ライブラリに存在する場合、動的リンカが実行時にどのシンボルを使用するか決定する仕組みです。

例えば：
1. `libA.so` が `int counter` を定義
2. `libB.so` も `int counter` を定義
3. プログラムが両方をリンク
4. 動的リンカが`LD_PRELOAD`等に基づいてどちらの`counter`を使うか決定

このため、`.globl`シンボルへのアクセスはコンパイル時に直接アドレスを決定できず、GOTを経由する必要があります。

一方、`.local`シンボルは外部から見えないため、インターポジションの対象にならず、直接的なRIP相対アクセスが可能です。

## 実装

### 1. `static_names`の追跡

パーサーで`static`キーワード付きのグローバル変数・関数を`static_names: HashSet<String>`に記録します。

```rust
// parser.rs
let is_static = if self.current().kind == TokenKind::Static {
    self.advance();
    true
} else {
    false
};

// ... パース後
if is_static {
    self.static_names.insert(name.clone());
}
```

### 2. `.local` vs `.globl`の使い分け

コード生成時に、`static_names`に含まれるシンボルには`.local`を使用：

```rust
// codegen.rs - グローバル変数
if name.starts_with("__static.") || self.static_names.contains(name) {
    self.emit(&format!("  .local {}", name));
} else {
    self.emit(&format!("  .globl {}", name));
}

// codegen.rs - 関数
if self.static_names.contains(&func.name) {
    self.emit(&format!("  .local {}", func.name));
} else {
    self.emit(&format!("  .globl {}", func.name));
}
```

### 3. GOTアクセスの拡張

PICモードでは、`extern`だけでなく全ての`.globl`シンボルへのアクセスをGOT経由に変更：

```rust
fn needs_got_access(&self, name: &str) -> bool {
    if !self.pic_mode {
        return false;
    }
    // File-local symbols can use direct RIP-relative addressing
    if name.starts_with("__static.") || self.static_names.contains(name) {
        return false;
    }
    true
}
```

この`needs_got_access`メソッドを以下の3箇所で使用：
- `gen_addr()`: グローバル変数のアドレス計算
- `emit_load_var()`: グローバル変数からの読み出し
- `emit_store_var()`: グローバル変数への書き込み

## GOT（Global Offset Table）の仕組み

```
プログラムコード:
  mov symbol@GOTPCREL(%rip), %rax   # GOTエントリのアドレスをロード
  mov (%rax), %eax                   # GOTエントリを通じて実際の値をロード

メモリレイアウト:
  .text (読み取り専用):
    [命令列] → GOTエントリを指す相対オフセット

  .got (書き込み可能):
    [GOTエントリ] → 動的リンカが実行時にシンボルの実アドレスを書き込む

  .data/.bss:
    [実際のデータ] ← GOTエントリが指す先
```

- `@GOTPCREL(%rip)`: RIP（現在の命令ポインタ）からGOTエントリまでの相対オフセット
- 動的リンカが起動時にGOTエントリにシンボルの実アドレスを書き込む
- 間接参照が1段増えるが、テキストセクションを変更する必要がなくなる

## 検証

- **統合テスト**: 578 パス、0 失敗
- **PostgreSQLバックエンドファイル（パース＋アセンブル）**: 331/331 成功
- **PostgreSQLバックエンドファイル（PICリンク）**: 331/331 成功（修正前: 286/331）

## 変更されたファイル

- `src/ast.rs` — `Program`に`static_names`フィールド追加
- `src/parser.rs` — `static`キーワードの追跡、`static_names`への登録
- `src/codegen.rs` — `.local`/`.globl`の使い分け、GOTアクセスの拡張（`needs_got_access`メソッド追加）
