# Step 20.2: PostgreSQL拡張のコンパイル

## 概要

このステップでは、静的ローカル変数における構造体イニシャライザのサポートを追加し、PostgreSQL拡張のソースファイルをコンパイルできるようにします。

## 変更内容

### 静的変数の構造体イニシャライザ

PostgreSQLの `PG_MODULE_MAGIC` マクロは、静的constな構造体イニシャライザを持つ関数に展開されます。

```c
static const Pg_magic_struct Pg_magic_data = {
    sizeof(Pg_magic_struct),  // struct size
    14020 / 100,              // major version
    100,                      // minor version
    32,                       // float size
    64,                       // datum size
    1                         // something else
};
```

以前の実装では、静的変数のイニシャライザは単一の数値リテラルのみをサポートしていました。現在は以下をサポートします。

1. **波括弧で囲まれたイニシャライザ** `{ val1, val2, ... }`:
   - 各値は定数式として評価される（`sizeof`、算術演算などをサポート）
   - 値は構造体のフィールドレイアウトに従ってバイト列にパックされる
   - フィールドのサイズとオフセットは構造体型のメンバ定義から読み取られる

2. **定数式イニシャライザ**:
   - `Num(n)` リテラルのマッチングから `eval_const_expr()` 評価に変更
   - `sizeof`、算術演算、キャスト、その他のコンパイル時式をサポート

### 構造体初期化の仕組み

構造体型のイニシャライザは以下の手順で処理されます。
1. `ty.size()` バイトのゼロ埋めバイト配列を割り当てる
2. イニシャライザリストの各値について:
   - 定数式を評価する
   - 構造体のメンバリストからフィールドサイズを決定する（オフセットでマッチング）
   - 現在のオフセットに値のバイト列を書き込む
   - 次のフィールドのオフセットに進む（アライメントを考慮）
3. バイト配列は `.data` セクションに `.byte` ディレクティブとして出力される

## 検証

PostgreSQL拡張のソースファイルが正常にコンパイルされるようになりました。

```c
#include "postgres.h"
#include "fmgr.h"
#include "utils/builtins.h"

PG_MODULE_MAGIC;           // ← struct initializer in static variable
PG_FUNCTION_INFO_V1(add_one);

Datum add_one(PG_FUNCTION_ARGS) {
    int32 arg = PG_GETARG_INT32(0);
    PG_RETURN_INT32(arg + 1);
}
```

マジック構造体の静的データセクションを含む正しいアセンブリが生成されます。
