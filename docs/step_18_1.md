# Phase 18: GCC拡張とビルトイン

## Step 18.1: __attribute__ のセマンティックサポート
既に実装済みです。`__attribute__((...))` は構文レベルで解析され、スキップされます。パーサーは型の前、型の後、関数パラメータリストの後、構造体メンバ上でこれを処理します。

## Step 18.2: 文式（Statement Expressions）
既に実装済みです。`({ stmt1; stmt2; expr; })` はすべての文を実行し、最後の式が値になります。`primary()` 内で `(` の後に `{` が続く場合に解析されます。

## Step 18.3: 拡張 __builtin 関数
追加のGCCビルトインのサポートを追加しました。

- `__builtin_choose_expr(const_expr, expr1, expr2)` -- コンパイル時の条件選択
- `__builtin_trap()` -- `abort()` 関数呼び出しにマッピング
- `__builtin_classify_type(expr)` -- 0を返す（簡略化実装）
- `__builtin_huge_val()`、`__builtin_inf()`、`__builtin_nan()` -- 0を返す（簡略化実装）
- `__builtin_clz/ctz/popcount/bswap/ffs/abs` -- GCCビルトインへの関数呼び出しとしてパススルー（libgcc経由でリンク）

以前に実装済みのもの:
- `__builtin_expect(expr, val)` → exprを返す
- `__builtin_constant_p(expr)` → 0を返す
- `__builtin_unreachable()` → 何もしない（no-op）
- `__builtin_offsetof(type, member)` → バイトオフセット
- `__builtin_types_compatible_p(type1, type2)` → 1または0

## Step 18.4: インラインアセンブリ
`asm()`、`__asm()`、`__asm__()` はオプションの `volatile` 修飾子付きで解析され、スキップされます。

```c
__asm__ volatile("" : : : "memory");  // memory barrier — skipped
asm("nop");                            // skipped
```

実装ではバランスの取れた括弧を解析し、内容を破棄します。PostgreSQLではインラインアセンブリが主にメモリバリアとスピンロック（Cのフォールバックがある）に使用されているため、これで十分です。

## Step 18.5: 計算されたgoto（Computed Goto）
間接ジャンプのためのGCC拡張です。

```c
void *p = &&target;  // &&label — address of label
goto *p;             // goto *expr — computed goto
```

### 実装

**ASTの追加:**
- `Expr::LabelAddr(String)` -- `&&label` 式
- `Stmt::GotoExpr(Expr)` -- `goto *expr` 文

**パーサー:**
- 単項演算子の位置で `&&` が出現 → ラベル名を解析し、`LabelAddr` を生成
- `goto *` → 式を解析し、`GotoExpr` を生成

**コード生成:**
- `LabelAddr`: `lea .Lnn(%rip), %rax` -- ラベルアドレスをロード
- `GotoExpr`: 式を評価し、`jmp *%rax` -- 間接ジャンプ

ラベルは通常の `goto`/`label:` と同じラベルマップを再利用し、一貫した命名を行います。

## Step 18.6: __extension__ キーワード
既に実装済みです。`__extension__` はキーワードとして認識され、型や式の前でスキップされます。

## Step 18.7: _Thread_local / __thread
ストレージクラス指定子として認識され、スキップされるキーワードとして追加しました。

```c
_Thread_local int counter = 0;  // parsed, thread-local ignored
__thread int tls_var;            // same
```

スレッドローカルストレージ修飾子は解析されますが、通常の変数として扱われます。完全なTLSサポート（fsセグメントレジスタ、.tbss/.tdataセクション）はリンカとの連携が必要なため実装していません。

## Step 18.8: __builtin_types_compatible_p
既に実装済みです。2つの型が同じ種類と符号性を持つ場合は1を、そうでない場合は0を返します。
