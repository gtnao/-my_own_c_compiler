# Step 20.4: PICコード生成と共有ライブラリのサポート

## 概要

このステップでは、Position Independent Code（PIC: 位置独立コード）の生成サポートを追加し、共有ライブラリ（`.so` ファイル）のコンパイルを可能にします。これはPostgreSQL拡張に必要です。

## 変更内容

### コマンドラインフラグ

- `-fPIC` / `-fpic`: PICコード生成を有効化
- `-shared`: 共有ライブラリとしてリンク（gccリンカに `-shared` を渡す）

### PICコード生成

PICモードでは、以下の変更が適用されます。

#### 1. GOT経由のexternグローバル変数アクセス

非PIC:
```asm
mov CurrentMemoryContext(%rip), %rax    # Direct RIP-relative access
```

PIC:
```asm
mov CurrentMemoryContext@GOTPCREL(%rip), %rax  # Load address from GOT
mov (%rax), %rax                                # Indirect load through GOT
```

GOT（Global Offset Table: グローバルオフセットテーブル）は、ロード時に動的リンカによって埋められます。共有オブジェクトでは、externシンボルのアドレスがコンパイル時に不明であるため、直接RIP相対アドレッシングを使用できません。

#### 2. PLT経由の関数呼び出し

非PIC:
```asm
call printf       # Direct call
```

PIC:
```asm
call printf@PLT   # Call through Procedure Linkage Table
```

PLT（Procedure Linkage Table: プロシージャリンケージテーブル）は、共有オブジェクトにおける関数呼び出しの遅延バインディングを提供します。

#### 3. 関数ポインタへの変換（関数名を値として使用）

非PIC:
```asm
lea func_name(%rip), %rax   # Direct address
```

PIC:
```asm
mov func_name@GOTPCREL(%rip), %rax   # Address through GOT
```

### 静的ローカル変数の可視性

静的ローカル変数（`__static.` プレフィックス付き）は `.globl` の代わりに `.local` ディレクティブを使用するようになりました。

```asm
# Before (incorrect for shared objects):
  .globl __static.Pg_magic_data.1
__static.Pg_magic_data.1:

# After (correct):
  .local __static.Pg_magic_data.1
__static.Pg_magic_data.1:
```

これにより、リンカがファイルローカルなシンボルに対してPLT/GOTエントリを作成しようとすることを防ぎます。

### コード生成での実装

`Codegen` 構造体に以下が追加されました。
- `pic_mode: bool` -- PICコード生成が有効かどうか
- `extern_names: HashSet<String>` -- externシンボル名のセット

変更された主なメソッド:
- `gen_addr()`: PICモードでexternグローバルに `@GOTPCREL` を使用
- `emit_load_var()`: PICモードでGOT経由でexternグローバルをロード
- `emit_store_var()`: PICモードでGOT経由でexternグローバルにストア
- `gen_expr()`（`FuncCall` 用）: PICモードで `@PLT` サフィックスを使用

## 検証

PostgreSQL拡張を共有ライブラリとしてコンパイルしました。
```bash
$ ./target/debug/my_own_c_compiler -fPIC -S -I/usr/include/postgresql/14/server \
    -o pg_ext.s pg_ext.c
$ gcc -shared -o pg_ext.so pg_ext.s
$ nm -D pg_ext.so | grep add_one
00000000000014f1 T add_one
```

生成された `.so` ファイルは `add_one`、`Pg_magic_func`、`pg_finfo_add_one` シンボルを正しくエクスポートしています。
