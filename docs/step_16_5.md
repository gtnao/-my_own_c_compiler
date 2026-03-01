# Step 16.5: 事前定義マクロ

## 概要

プリプロセッシング開始前に自動的に定義される、包括的な事前定義マクロのセットを追加します。これらは、`#ifdef __STDC__` や `#if __GNUC__` などを使用するPostgreSQLおよび標準Cライブラリヘッダに不可欠です。

## 追加された事前定義マクロ

### 標準C
- `__STDC__` = 1
- `__STDC_VERSION__` = 201112L（C11）
- `__STDC_HOSTED__` = 1

### プラットフォーム / アーキテクチャ
- `__LP64__` = 1（64ビットの long と ポインタ）
- `__x86_64__`、`__x86_64`、`__amd64__`、`__amd64` = 1
- `__linux__`、`__linux`、`linux` = 1
- `__unix__`、`__unix`、`unix` = 1

### GCC互換性
- `__GNUC__` = 4、`__GNUC_MINOR__` = 0、`__GNUC_PATCHLEVEL__` = 0
- これにより、機能検出マクロに対してコンパイラがGCC 4.0として認識されます

### 型サイズ
- `__SIZEOF_SHORT__` = 2、`__SIZEOF_INT__` = 4
- `__SIZEOF_LONG__` = 8、`__SIZEOF_LONG_LONG__` = 8
- `__SIZEOF_POINTER__` = 8、`__SIZEOF_FLOAT__` = 4、`__SIZEOF_DOUBLE__` = 8
- `__CHAR_BIT__` = 8

### 型名
- `__SIZE_TYPE__` = `unsigned long`
- `__PTRDIFF_TYPE__` = `long`
- `__INTMAX_TYPE__` = `long`
- `__WCHAR_TYPE__` = `int`

### 上限値
- `__INT_MAX__` = 2147483647
- `__LONG_MAX__` = 9223372036854775807L
- `__SHRT_MAX__` = 32767
- `__SCHAR_MAX__` = 127

### エンディアン
- `__BYTE_ORDER__` = 1234（リトルエンディアン）
- `__ORDER_LITTLE_ENDIAN__` = 1234
- `__ORDER_BIG_ENDIAN__` = 4321

### 便利マクロ
- `NULL` = `((void *)0)`

## これらが重要な理由

PostgreSQLのヘッダは広範な条件付きコンパイルを使用しています。
```c
#ifdef __GNUC__
#define pg_attribute_noreturn() __attribute__((noreturn))
#endif

#if __STDC_VERSION__ >= 201112L
#define StaticAssertStmt(condition, errmessage) _Static_assert(condition, errmessage)
#endif
```
