# Step 17.1〜17.6: 標準ライブラリヘッダスタブ

## 概要

Phase 17 では、`#include <stdio.h>` や `#include <stdlib.h>` などを使用するCプログラムを本コンパイラでコンパイルできるようにするための、標準ライブラリヘッダスタブの完全なセットを実装します。これらは「スタブ」であり、型定義、関数宣言、マクロ定数を提供しますが、実際の関数の実装はリンク時にシステムのlibcから取得されます。

## 問題

実世界のCコード（特にPostgreSQL）をコンパイルする際、ソースファイルは多くの標準ヘッダを `#include` します。独自のヘッダスタブがなければ、プリプロセッサは以下のいずれかの問題に直面します。
1. ヘッダが見つからない（システムパスを検索しない場合）
2. 実際のシステムヘッダをインクルードしてしまう（本コンパイラがサポートしないGCC固有の拡張を使用している）

解決策は、必要なものだけを正確に宣言する、最小限かつ互換性のあるヘッダスタブを提供することです。

## プリプロセッサのヘッダ検索パス

プリプロセッサは、複数の場所からヘッダを検索するように変更されました。

```
#include <header> の検索順序:
1. バイナリ相対パス: ../../include, ../include, include（コンパイラバイナリからの相対）
2. CARGO_MANIFEST_DIR/include（開発モード）
3. ./include（カレントワーキングディレクトリ）
4. ソースファイルのディレクトリ
5. /usr/include, /usr/local/include（システムヘッダ）
```

`#include "header"` の場合、ソースファイルのディレクトリが最初に検索されます（コンパイラのインクルードパスよりも前）。

これにより、本コンパイラの組み込みヘッダがシステムヘッダよりも優先され、利用可能な定義を正確に制御できます。

## 作成されたヘッダファイル

### stddef.h
ほぼすべての他のヘッダで使用されるコア型定義です。
- `size_t` -- `unsigned long`（x86-64で8バイト）
- `ptrdiff_t` -- `long`（ポインタ差分型）
- `wchar_t` -- `int`（ワイド文字型）
- `NULL` -- `((void *)0)`
- `offsetof(type, member)` -- `__builtin_offsetof` にマッピング

### stdint.h
固定幅整数型です。
- `int8_t` から `int64_t`（符号付き）
- `uint8_t` から `uint64_t`（符号なし）
- `intptr_t`、`uintptr_t` -- ポインタサイズの整数
- `intmax_t`、`uintmax_t` -- 最大幅整数
- 最小/最大定数: `INT8_MIN`、`INT32_MAX`、`UINT64_MAX` など

### stdbool.h
ブーリアン型のサポート（C99）です。
- `bool` → `_Bool`
- `true` → `1`
- `false` → `0`

### stdio.h
標準I/O宣言です。
- `FILE` 型（不透明な構造体ポインタ）
- `stdin`、`stdout`、`stderr` -- 標準ストリーム
- `printf`、`fprintf`、`sprintf`、`snprintf` -- 書式付き出力
- `scanf`、`fscanf`、`sscanf` -- 書式付き入力
- `fopen`、`fclose`、`fread`、`fwrite` -- ファイル操作
- `fgetc`、`fgets`、`fputc`、`fputs` -- 文字/文字列I/O
- `fseek`、`ftell`、`rewind` -- ファイル位置操作
- 定数: `EOF`、`SEEK_SET`、`SEEK_CUR`、`SEEK_END`、`BUFSIZ`

### stdlib.h
汎用ユーティリティです。
- `malloc`、`calloc`、`realloc`、`free` -- メモリ割り当て
- `exit`、`abort`、`_exit`、`atexit` -- プログラム制御
- `atoi`、`atol`、`strtol`、`strtoul` -- 文字列から数値への変換
- `qsort`、`bsearch` -- ソートと検索
- `rand`、`srand` -- 乱数
- `getenv`、`setenv`、`system` -- 環境
- 定数: `EXIT_SUCCESS`、`EXIT_FAILURE`、`RAND_MAX`

### string.h
文字列およびメモリ操作です。
- `memcpy`、`memmove`、`memset`、`memcmp`、`memchr` -- メモリ操作
- `strlen`、`strcpy`、`strncpy`、`strcmp`、`strncmp` -- 文字列操作
- `strcat`、`strncat`、`strchr`、`strrchr`、`strstr` -- 文字列操作
- `strdup`、`strndup` -- 文字列複製
- `strtok`、`strtok_r` -- トークン化
- `strcasecmp`、`strncasecmp` -- 大文字小文字を無視した比較

### stdarg.h
可変引数サポートです。
- `va_list` → `__builtin_va_list`
- `va_start(ap, last)` → `__builtin_va_start(ap, last)`
- `va_arg(ap, type)` → `__builtin_va_arg(ap, type)`
- `va_end(ap)` → `__builtin_va_end(ap)`
- `va_copy(dest, src)` → `__builtin_va_copy(dest, src)`

これらはコンパイラ組み込みの可変引数処理にマッピングされます。

### errno.h
エラー番号サポートです。
- `errno` → `(*__errno_location())`（Linuxにおけるスレッドセーフなerrno）
- エラー定数: `EPERM`、`ENOENT`、`EINTR`、`EINVAL`、`ENOMEM` など

### limits.h
処理系定義の上限値です。
- `CHAR_BIT` = 8
- `INT_MIN`、`INT_MAX`、`UINT_MAX` -- int の上限値
- `LONG_MIN`、`LONG_MAX`、`ULONG_MAX` -- long の上限値
- `LLONG_MIN`、`LLONG_MAX`、`ULLONG_MAX` -- long long の上限値
- `PATH_MAX` = 4096、`NAME_MAX` = 255

### assert.h
アサーションマクロです。
- `assert(expr)` -- 式が偽の場合 `abort()` を呼び出す
- `NDEBUG` が定義されている場合は無効化される

### ctype.h
文字分類です。
- `isalnum`、`isalpha`、`isdigit`、`isxdigit` -- 文字判定
- `islower`、`isupper`、`isspace`、`isprint` -- その他の判定
- `tolower`、`toupper` -- 文字変換

### unistd.h（POSIX）
POSIXオペレーティングシステムAPIです。
- `read`、`write`、`close`、`lseek` -- 基本I/O
- `dup`、`dup2`、`pipe` -- ファイルディスクリプタ操作
- `fork`、`execv`、`execvp`、`execve` -- プロセス制御
- `getpid`、`getppid`、`getuid`、`getgid` -- プロセス情報
- `chdir`、`getcwd`、`access`、`unlink`、`rmdir` -- ファイルシステム
- `sleep`、`usleep` -- 時間制御
- `symlink`、`readlink` -- シンボリックリンク
- 定数: `STDIN_FILENO`、`STDOUT_FILENO`、`STDERR_FILENO`
- アクセスモード定数: `F_OK`、`R_OK`、`W_OK`、`X_OK`

### fcntl.h
ファイル制御です。
- `open`、`creat`、`fcntl` -- ファイル操作
- オープンフラグ: `O_RDONLY`、`O_WRONLY`、`O_RDWR`、`O_CREAT`、`O_TRUNC`、`O_APPEND` など
- `fcntl` コマンド: `F_DUPFD`、`F_GETFD`、`F_SETFD`、`F_GETFL`、`F_SETFL`

### sys/types.h
POSIX型定義です。
- `pid_t`、`uid_t`、`gid_t` -- プロセス/ユーザー/グループID
- `off_t`、`ssize_t` -- ファイルオフセットと符号付きサイズ
- `mode_t`、`dev_t`、`ino_t`、`nlink_t` -- ファイルシステム型
- `blksize_t`、`blkcnt_t` -- ブロック型
- `time_t`、`suseconds_t` -- 時間型

## 主な設計方針

1. **不透明なFILE型**: `typedef struct _IO_FILE FILE;` -- FILEを不透明な構造体へのポインタとして宣言します。実際の構造体定義はglibc内にあり、ポインタ型のみが必要です。

2. **スレッドセーフなerrno**: `#define errno (*__errno_location())` -- これはerrnoがスレッドごとの変数として関数経由でアクセスされるLinux/glibcの実装に一致します。

3. **インクルードガード**: すべてのヘッダは `#ifndef _HEADER_H` / `#define _HEADER_H` ガードを使用し、多重インクルードを防止します。

4. **ヘッダ間の依存関係**: ヘッダは必要に応じて互いにインクルードします（例: `stdio.h` は `size_t` のために `stddef.h` を、`va_list` のために `stdarg.h` をインクルード）。

5. **コンパイラ組み込みヘッダの優先**: 本コンパイラのヘッダはシステムヘッダよりも先に検索され、異なるLinuxディストリビューション間で一貫した動作を保証します。

## テストケース

```c
// stddef.h: size_t type
#include <stddef.h>
int main() { size_t s = 8; return s - 8; }  // => 0

// stdbool.h: bool type
#include <stdbool.h>
int main() { bool b = true; return b; }  // => 1

// stdint.h: fixed-width types
#include <stdint.h>
int main() { int32_t a = 42; uint64_t b = 100; return a + b - 142; }  // => 0

// stdio.h: EOF constant
#include <stdio.h>
int main() { return EOF + 1; }  // => 0

// stdlib.h: EXIT_SUCCESS/EXIT_FAILURE
#include <stdlib.h>
int main() { return EXIT_SUCCESS; }  // => 0

// string.h: strlen, strcmp (linked with libc)
#include <string.h>
int main() { return strlen("hello"); }  // => 5

// errno.h: error constants
#include <errno.h>
int main() { return EINVAL; }  // => 22

// limits.h: integer limits
#include <limits.h>
int main() { return CHAR_BIT; }  // => 8

// unistd.h: POSIX constants
#include <unistd.h>
int main() { return STDOUT_FILENO; }  // => 1

// fcntl.h: file control constants
#include <fcntl.h>
int main() { return O_RDONLY; }  // => 0

// sys/types.h: POSIX types
#include <sys/types.h>
int main() { pid_t p = 0; return p; }  // => 0

// Cross-header test: malloc + strcpy + strcmp + free
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main() {
    char *s = malloc(10);
    strcpy(s, "test");
    int r = strcmp(s, "test");
    free(s);
    return r;  // => 0
}
```

合計: 19個の新規テストを追加（537 → 556）。
