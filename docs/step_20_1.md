# Step 20.1: PostgreSQLビルドシステムとの統合

## 概要

このステップでは、PostgreSQLヘッダファイルのコンパイルに必要な基盤サポートを追加します。実世界のシステムヘッダとPostgreSQLのビルドインフラの複雑さに対応するため、コンパイラの複数のサブシステムを強化しました。

## 変更内容

### 1. CLIフラグのサポート: `-I` と `-D`

**`-I`（インクルードパス）**
```bash
./my_own_c_compiler -I/usr/include/postgresql/14/server source.c
```

2つの形式をサポートします。
- `-I<dir>`（スペースなし）: `-I/usr/include`
- `-I <dir>`（スペースあり）: `-I /usr/include`

インクルードパスは、コンパイラ組み込みヘッダの後、システムヘッダ（`/usr/include`）の前に検索されます。これはGCCの動作と一致しています。

**`-D`（マクロ定義）**
```bash
./my_own_c_compiler -DVAL=42 -DFLAG source.c
```

サポートする形式:
- `-D<name>=<value>`: 特定の値でマクロを定義
- `-D<name>`: マクロを `1` として定義
- `-D <name>=<value>`: スペース区切り形式

### 2. プリプロセッサ: コメント除去

ディレクティブ処理の前にCスタイルのコメント（`/* ... */` と `// ...`）を除去する `strip_comments()` 関数を追加しました。これが重要な理由は以下の通りです。

- glibcの `features.h` や `sys/cdefs.h` などのシステムヘッダには複雑な複数行コメントが含まれている
- コメント除去なしでは、コメント内の `#` 文字がプリプロセッサディレクティブとして誤認識される可能性がある
- ブロックコメントは正しい行番号を維持するために改行を保持する

### 3. プリプロセッサ: ディレクティブの正規化

C標準では `#` とディレクティブ名の間に空白が許可されています。
```c
#  define FOO 1      /* valid C */
# ifdef BAR          /* valid C */
```

プリプロセッサは `trimmed.starts_with("#define")` に依存する代わりに、`#` と先頭の空白を削除してからパターンマッチングを行うことでディレクティブを正規化するようになりました。

### 4. プリプロセッサ: 正しい `#if`/`#elif`/`#else`/`#endif` チェーン追跡

**修正されたバグ**: 条件付きコンパイルのスタックは以前 `(active: bool)` のみを追跡していました。これにより `#ifdef ... #elif ... #else` チェーンで不正な動作が発生していました。

```c
#ifdef HAVE_LONG_INT_64      // true
typedef long int int64;
#elif defined(HAVE_LONG_LONG) // should be skipped
typedef long long int int64;
#else                          // should be skipped
#error must have 64-bit type  // BUG: was reached!
#endif
```

修正ではスタックを `Vec<(bool, bool)>` -- `(active, any_branch_taken)` に変更しました。
- `any_branch_taken` は現在の `#if`/`#elif`/`#else` チェーンでいずれかのブランチが既に採用されたかどうかを追跡
- `#elif`: `any_branch_taken` が真の場合、無条件で active=false に設定
- `#else`: `any_branch_taken` が真の場合は active=false に、そうでなければ active=true に設定

### 5. レキサー: 整数オーバーフローの安全性

`0xFFFFFFFFFFFFFFFF` のような大きな16進数リテラルが、レキサーで算術オーバーフローパニックを引き起こしていました。以下のように修正しました。
- 16進数、2進数、8進数の解析に `u64` と `wrapping_mul`/`wrapping_add` を使用
- 計算後に `i64` にキャスト
- 数値サフィックスの除去を解析対象の文字列から分離

### 6. パーサー: `__int128` 型のサポート

PostgreSQLはGCCの128ビット整数型を使用しています。
```c
typedef __int128 int128;
typedef unsigned __int128 uint128;
```

`__int128`、`__int128_t`、`__uint128_t` を認識される型識別子として追加し、内部的には `long`（64ビット）にマッピングしています。完全な128ビット算術演算は実装されていませんが、型宣言は正しくコンパイルされます。

### 7. パーサー: 宣言後の `__attribute__`

以下の箇所に `skip_attribute()` 呼び出しを追加しました。
- `extern` 宣言: `extern void func(...) __attribute__((noreturn));`
- `typedef` 宣言: `typedef __int128 int128 __attribute__((aligned(8)));`

### 8. パーサー: 配列次元における定数式

配列の次元は以前、数値リテラルのみを受け付けていました。現在は完全な定数式を受け付けます。
```c
char padding[128 - sizeof(unsigned short) - sizeof(unsigned long)];
```

すべての配列サイズ解析箇所で、`TokenKind::Num` を期待する代わりに `eval_const_expr()` を使用するように変更しました。

### 9. システムインクルードパス

Debian/Ubuntuにおけるアーキテクチャ固有のヘッダ用に、`/usr/include/x86_64-linux-gnu` をシステムヘッダ検索パスに追加しました。

## テスト結果

- 既存の578テストすべてがパス
- `-I` と `-D` フラグ用の5つの新規テスト
- PostgreSQLの `postgres.h` ヘッダ（`c.h`、`pg_config.h`、システムヘッダなどを取り込む）が正常にコンパイル

## 検証

```bash
# Compile a file that includes postgres.h
echo '#include "postgres.h"
int main() { return 0; }' > /tmp/test_pg.c
./target/debug/my_own_c_compiler -I/usr/include/postgresql/14/server /tmp/test_pg.c > /tmp/test_pg.s
# Assembly output generated successfully (7000+ lines)
```
