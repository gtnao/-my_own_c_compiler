# Step 21.1: GCCプリプロセッサ互換性 -- パーシング修正

## 概要

このステップでは、`gcc -E` でプリプロセスされたPostgreSQLバックエンドソースファイルのコンパイル時に発見された複数のパーシング問題を修正します。目標は、すべての331個のPostgreSQLバックエンド `.c` ファイルを本コンパイラで正常にパースすることです。

## 変更内容

### 1. extern関数定義

GCCプリプロセッサの出力では、`extern` 付きの関数定義（プロトタイプだけでなく）が生成されることがあります。

```c
extern tuplehash_hash *tuplehash_create(MemoryContext ctx, uint32 nelements,
    void *private_data)
{
    // function body
}
```

C言語では関数定義に `extern` を付けることは合法です（デフォルトのリンケージ）が、本コンパイラのパーサーはexternプロトタイプ（パラメータリスト後に `;` を期待）のみを処理していました。

**修正**: 既存のexternプロトタイプハンドラがパラメータリストを深さマッチングでスキップした後、現在のトークンが `{`（関数本体）かどうかを確認します。そうであれば、パーサー位置を復元し、`function_or_prototype()` を使って再解析します。

重要なポイント: 堅牢な波括弧深さベースのパラメータスキッピングはプロトタイプ用に維持し（複雑なglibcパラメータ宣言を処理するため）、パラメータの後に `{` が検出された場合にのみ完全な関数解析にフォールバックします。

### 2. extern以外のカンマ区切りグローバル変数

PostgreSQLは単一の宣言で複数のグローバル変数を宣言します。

```c
sigset_t UnBlockSig, BlockSig, StartupBlockSig;
```

externのカンマ区切りハンドラは既に存在していましたが、extern以外のグローバル変数は同じパターンに対応していませんでした。

**修正**: `global_var()` で最初の変数名が解析された後に、カンマ区切りの処理を追加しました。後続の名前についてもポインタスター（`*`）と配列次元をサポートします。

### 3. GCC文式（Statement Expressions） `__extension__ ({...})`

GCCの文式はPostgreSQLヘッダに登場します。

```c
__extension__ ({ __typeof__(a) _a = (a); __typeof__(b) _b = (b); _a > _b ? _a : _b; })
```

2つの問題が見つかりました。

#### キャスト検出の誤検知

`unary()` におけるキャスト検出は `(type)expr` のパターンを確認します。`__extension__` が `is_type_start()` に含まれているため、`(__extension__ ({...}))` パターンがキャスト式として誤認識されていました。

**修正**: キャスト検出に除外条件を追加しました。`tokens[pos+1]` が `Extension` で、その後に `LParen LBrace` が続く場合、キャスト解析をスキップします。

#### 文コンテキスト

文コンテキストで `__extension__` の後に `({` が続く場合、`__extension__` が `is_type_start` をトリガーするため、変数宣言として扱われていました。

**修正**: `stmt()` に `Extension` ケースを追加し、`__extension__ ({` パターンを確認して式文として解析するようにしました。

### 4. 静的ローカル変数の波括弧イニシャライザ二重消費バグ

波括弧イニシャライザを持つ静的ローカル変数で "expected RBrace" エラーが発生していました。

```c
static const Oid funcargs[] = {23, 23, 2275, 2281, 23, 16};
```

**根本原因**: `parse_global_brace_init()` が既に閉じ `}` を消費しているにもかかわらず、`static_local_var()` がその後にさらに `self.expect(TokenKind::RBrace)` を呼び出していました。

**修正**: `static_local_var()` の冗長な `expect(RBrace)` 呼び出しを削除しました。

## 実装の詳細

### extern関数定義の検出戦略

「スキップしてから判定」パターンを使用しています。

```
extern handler:
  1. Save position (extern_start)
  2. Skip 'extern', qualifiers
  3. Parse type
  4. Get function name
  5. Skip parameter list (brace-depth matching)
  6. Skip attributes, __asm__
  7. Check current token:
     - If '{' → restore to extern_start, call function_or_prototype()
     - If ';' → register as extern prototype (existing behavior)
```

これは `is_function()` + `function_or_prototype()` を直接呼び出すよりも堅牢です。なぜなら、`function_or_prototype()` はパラメータ宣言を完全にパースする必要がありますが、`char[20]` パラメータや `__restrict` 修飾子などを含む複雑なglibcプロトタイプでは失敗するためです。

## 結果

- **統合テスト**: 578 パス、0 失敗
- **PostgreSQLバックエンドファイル**: 331/331 パス（100%）

以前: 318/331（96.1%）

## 変更されたファイル

- `src/parser.rs` -- 上記4つの修正すべて
