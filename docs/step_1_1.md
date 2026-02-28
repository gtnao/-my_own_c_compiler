# Step 1.1: 単一の整数リテラルをコンパイル

## 概要

Cコンパイラ構築の最初のステップ。入力として単一の整数を受け取り、その値を返す `main` 関数のx86-64アセンブリを生成する。

## 入出力

**入力** (ファイル):
```
42
```

**出力** (stdout):
```asm
  .globl main
main:
  mov $42, %rax
  ret
```

## 実装

### src/main.rs

このステップでは全てのロジックを `main.rs` に記述する。モジュール分割はまだ行わない。

```rust
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file>", args[0]);
        process::exit(1);
    }

    let input = fs::read_to_string(&args[1]).unwrap_or_else(|err| {
        eprintln!("Failed to read file '{}': {}", args[1], err);
        process::exit(1);
    });

    let input = input.trim();

    let val: i64 = input.parse().unwrap_or_else(|_| {
        eprintln!("Expected a number, but got '{}'", input);
        process::exit(1);
    });

    println!("  .globl main");
    println!("main:");
    println!("  mov ${}, %rax", val);
    println!("  ret");
}
```

### 処理の流れ

```
ファイル読み込み → 文字列をi64にパース → アセンブリを標準出力に出力
```

1. **コマンドライン引数**: ファイルパスを1つ受け取る
2. **ファイル読み込み**: `fs::read_to_string` でファイル内容を文字列として読む
3. **パース**: `trim()` で前後の空白・改行を除去し、`parse::<i64>()` で整数に変換
4. **コード生成**: 4行のアセンブリを `println!` で標準出力に出力

## 生成されるアセンブリの解説

```asm
  .globl main       # (1) mainシンボルをリンカに公開
main:                # (2) main関数のエントリポイント
  mov $42, %rax      # (3) 即値42をraxレジスタに格納
  ret                # (4) 呼び出し元に戻る
```

### (1) `.globl main`

アセンブラディレクティブ。`main` シンボルをグローバルにして、リンカが見つけられるようにする。Cの `main` 関数はプログラムのエントリポイントであり、Cランタイム (`_start` → `__libc_start_main`) から呼ばれる。

### (2) `main:`

ラベル。この位置のアドレスに `main` という名前が付く。

### (3) `mov $42, %rax`

AT&T構文の `mov` 命令。

| 記法 | 意味 |
|------|------|
| `$42` | 即値 (immediate value)。`$` は定数であることを示す |
| `%rax` | 64ビットの汎用レジスタ。`%` はレジスタであることを示す |

System V AMD64 ABI (x86-64 Linuxの関数呼び出し規約) では、**`%rax` は関数の戻り値を格納するレジスタ** と定められている。

### (4) `ret`

`ret` 命令は以下の2つの操作を行う:

1. スタックトップからリターンアドレスを `pop` する (`pop %rip` に相当)
2. そのアドレスにジャンプする

このとき `%rax` の値がそのまま関数の戻り値として呼び出し元に渡される。

## コンパイルから実行までの流れ

```bash
# 1. Rustコンパイラをビルド
$ cargo build

# 2. Cソースファイルを作成
$ echo '42' > /tmp/test.c

# 3. 自作コンパイラでアセンブリを生成
$ ./target/debug/my_own_c_compiler /tmp/test.c > /tmp/test.s

# 4. GCCでアセンブリをアセンブル＆リンク
$ gcc -o /tmp/test /tmp/test.s

# 5. 実行
$ /tmp/test

# 6. 終了コードを確認 (= main の戻り値)
$ echo $?
42
```

### なぜGCCを使うのか

自作コンパイラが行うのは「Cソース → アセンブリ」の変換のみ。アセンブリから実行可能バイナリへの変換は既存のツールチェーンに任せる:

```
自作コンパイラ          GCC (as + ld)
.c ──────────→ .s ──────────────→ 実行可能バイナリ
```

- **as** (GNU Assembler): `.s` → `.o` (オブジェクトファイル)
- **ld** (リンカ): `.o` + Cランタイム → 実行可能バイナリ

`gcc` コマンドは内部で `as` と `ld` を順番に呼んでくれる。

## テスト

### tests/test.sh

シェルスクリプトによる統合テスト。パターンは以下の通り:

```bash
assert() {
  expected="$1"  # 期待する終了コード
  input="$2"     # コンパイルする入力

  echo "$input" > "$TMPDIR/tmp.c"                      # 入力をファイルに書き出し
  $COMPILER "$TMPDIR/tmp.c" > "$TMPDIR/tmp.s"           # 自作コンパイラでアセンブリ生成
  gcc -o "$TMPDIR/tmp" "$TMPDIR/tmp.s"                  # GCCでアセンブル＆リンク
  "$TMPDIR/tmp"                                         # 実行
  actual="$?"                                           # 終了コードを取得

  # 期待値と比較
}
```

### テストケース

| 入力 | 期待する終了コード | 目的 |
|------|-------------------|------|
| `0` | 0 | 最小値 |
| `42` | 42 | 一般的な値 |
| `255` | 255 | 終了コードの最大値 (8ビット) |
| `1` | 1 | 最小の正の値 |
| `100` | 100 | 3桁の値 |

> **注意**: Linuxの終了コードは0-255の範囲。256以上の値は下位8ビットのみが使われる (例: 256 → 0)。

## このステップで学べること

1. **コンパイラの最小構成**: 入力→変換→出力の基本パイプライン
2. **x86-64アセンブリの基本**: `mov`, `ret`, レジスタ、AT&T構文
3. **System V AMD64 ABI**: 戻り値は `%rax`
4. **テスト駆動開発**: 最初からテストを書いて正しさを確認する習慣

## ファイル構成

```
src/
  main.rs          -- 全ロジック (ファイル読み込み, パース, コード生成)
tests/
  test.sh          -- 統合テスト
docs/
  step_1_1.md      -- このファイル
```

## 次のステップ

→ **Step 1.2: 加減算** — `+` と `-` 演算子を追加し、`5+20-4` のような式をコンパイルできるようにする。
