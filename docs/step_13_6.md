# Step 13.6: 複数ファイルのコンパイルとCLIオプション

## 概要

コンパイルパイプラインを制御するためのGCC互換コマンドラインオプションを追加します:

- `-E` — プリプロセスのみ（プリプロセス済みソースを出力）
- `-S` — アセンブリにコンパイル（`.s`ファイル）
- `-c` — オブジェクトファイルにコンパイル（`.o`ファイル）
- `-o <file>` — 出力ファイル名を指定
- デフォルト（フラグなし）— コンパイルしてリンクし実行ファイルを生成、または単一ファイルの場合はアセンブリを標準出力に出力

## CLIの使い方

```bash
# Preprocess only
mycc -E input.c              # output to stdout
mycc -E input.c -o output.i  # output to file

# Compile to assembly
mycc -S input.c              # creates input.s
mycc -S input.c -o output.s  # creates output.s

# Compile to object file
mycc -c input.c              # creates input.o
mycc -c input.c -o output.o  # creates output.o

# Compile and link
mycc input.c -o program      # creates executable 'program'
mycc file1.c file2.c -o prog # multi-file compilation

# Legacy mode (single file, stdout)
mycc input.c                  # assembly to stdout (backwards compatible)
```

## 実装

### 出力モード

```rust
enum OutputMode {
    Preprocess, // -E
    Assembly,   // -S
    Object,     // -c
    Executable, // default
}
```

### 引数の解析

引数を左から右に処理するシンプルなフラグベースの解析:

```rust
match args[i].as_str() {
    "-E" => mode = OutputMode::Preprocess,
    "-S" => mode = OutputMode::Assembly,
    "-c" => mode = OutputMode::Object,
    "-o" => { output_file = Some(args[i+1].clone()); i += 1; }
    arg if arg.starts_with('-') => {} // ignore unknown flags
    _ => input_files.push(args[i].clone()),
}
```

不明なフラグはGCC互換性のために無視されます。

### コンパイルパイプライン

`compile_to_assembly()`関数がパイプライン全体をカプセル化します:

```
read file → preprocess → lex → parse → codegen → assembly string
```

`-c`および実行ファイルモードでは、アセンブリは一時ファイルに書き込まれ、`gcc -c`でアセンブルされます。リンク時には、すべてのオブジェクトファイルが`gcc`でリンクされます。

### 複数ファイルのコンパイル

複数の入力ファイルが指定された場合:
1. 各ファイルが独立してアセンブリにコンパイルされる
2. 各アセンブリファイルがオブジェクトファイルにアセンブルされる（gcc -c経由）
3. すべてのオブジェクトファイルがリンクされる（gcc経由）
4. 一時ファイルがクリーンアップされる

## 後方互換性

単一ファイルで`-o`フラグなし（`test.sh`で使用されるレガシーモード）で起動した場合、アセンブリは標準出力に出力されます。これにより既存のテストインフラストラクチャとの互換性が維持されます。

## 出力ファイルの命名規則

| モード | デフォルト出力 |
|---|---|
| `-S input.c` | `input.s` |
| `-c input.c` | `input.o` |
| `input.c -o prog` | `prog` |
| `input.c`（フラグなし） | 標準出力 |
