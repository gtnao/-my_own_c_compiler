# ステップ 11.2: 可変長引数（`...`, `va_list`, `va_start`, `va_arg`）

## 概要

このステップでは、可変長引数をサポートする関数を実装する。これにより、関数が可変個の引数を受け取れるようになる。`printf` のような関数やカスタムの可変長引数関数に不可欠な機能である。

主な構成要素は以下の通り:
- `...`（省略記号）: 関数パラメータリストで可変長引数関数を宣言する
- `va_list` 型: 可変長引数を走査するための型
- `va_start(ap, last_param)`: `va_list` を初期化する
- `va_arg(ap, type)`: 次の引数を取得する
- `va_end(ap)`: 後始末（本実装では何もしない）

## System V AMD64 ABI とレジスタ保存領域

x86-64 Linux（System V AMD64 ABI）では、最初の6つの整数/ポインタ引数はレジスタで渡される:

| レジスタ | 引数インデックス |
|----------|-----------------|
| `%rdi`   | 0               |
| `%rsi`   | 1               |
| `%rdx`   | 2               |
| `%rcx`   | 3               |
| `%r8`    | 4               |
| `%r9`    | 5               |

可変長引数関数では、`va_arg` が順番に引数を走査できるように、すべてのレジスタ引数を連続したメモリ領域（**レジスタ保存領域**）に保存する必要がある。

### レジスタ保存領域のレイアウト

スタック上に48バイト（6レジスタ × 各8バイト）を確保する:

```
Higher addresses (toward %rbp)
┌──────────────────────────────────┐
│ %r9  (arg 5)  [rbp - base + 40] │
├──────────────────────────────────┤
│ %r8  (arg 4)  [rbp - base + 32] │
├──────────────────────────────────┤
│ %rcx (arg 3)  [rbp - base + 24] │
├──────────────────────────────────┤
│ %rdx (arg 2)  [rbp - base + 16] │
├──────────────────────────────────┤
│ %rsi (arg 1)  [rbp - base +  8] │
├──────────────────────────────────┤
│ %rdi (arg 0)  [rbp - base     ] │  ← va_save_area_offset
└──────────────────────────────────┘
Lower addresses (toward %rsp)
```

ここで `base` は `va_save_area_offset` であり、`%rbp` から保存領域の先頭（最も低いアドレス、`%rdi` が格納される場所）までの距離を表す。

引数はアドレスの昇順で格納される: `%rdi` が最も低いアドレスに、`%r9` が最も高いアドレスに配置される。つまり、引数を順番に読み進めるには、ポインタに **8を加算** する必要がある。

### 可変長引数関数の関数プロローグ

```asm
  push %rbp
  mov %rsp, %rbp
  sub $N, %rsp           # N includes locals + 48 bytes for save area

  # Save all 6 register arguments to the save area
  mov %rdi, -72(%rbp)    # arg 0 (example offset)
  mov %rsi, -64(%rbp)    # arg 1
  mov %rdx, -56(%rbp)    # arg 2
  mov %rcx, -48(%rbp)    # arg 3
  mov %r8,  -40(%rbp)    # arg 4
  mov %r9,  -32(%rbp)    # arg 5
```

## `va_list` の実装

`va_list` は `char *`（レジスタ保存領域へのシンプルなポインタ）として実装している。これは完全な ABI の `va_list` 構造体（`gp_offset`、`fp_offset`、`overflow_arg_area`、`reg_save_area` を含む）の簡略版だが、整数引数が6個までであれば動作する。

## `va_start` の実装

`va_start(ap, last_param)` は、レジスタ保存領域内の最初の **無名引数** を指すように `ap` を初期化する。

関数が `n` 個の名前付きパラメータを持つ場合、最初の無名引数はレジスタインデックス `n` にあり、`%rbp` からのオフセットは `va_save_area_offset - n * 8` となる。

```asm
  # va_start(ap, last_param) where function has 1 named param
  lea -(va_save_area_offset - 1*8)(%rbp), %rax   # address of arg 1 in save area
  # Store this address into ap variable
  push %rax
  lea -ap_offset(%rbp), %rax    # address of ap
  mov %rax, %rdi
  pop %rax
  mov %rax, (%rdi)              # ap = &save_area[param_count]
```

## `va_arg` の実装

`va_arg(ap, type)` は、現在の `ap` 位置の値を読み取り、`ap` を8バイト進める。

```asm
  # va_arg(ap, int)
  lea -ap_offset(%rbp), %rax    # address of ap variable
  mov %rax, %rcx                # save address of ap in %rcx
  mov (%rcx), %rdi              # load current ap value (pointer to next arg)
  movslq (%rdi), %rax           # load int value from *ap (sign-extended)
  push %rax                     # save the loaded value
  add $8, %rdi                  # advance ap to next argument
  mov %rdi, (%rcx)              # store updated ap back
  pop %rax                      # restore loaded value to %rax
```

引数はアドレスの昇順に格納されている（arg 0 が最低アドレス、arg 5 が最高アドレス）ため、進行方向は `add $8` となる。

型ごとの処理:
- `int`: `movslq (%rdi), %rax`（32ビットを64ビットに符号拡張）
- `long`/ポインタ: `mov (%rdi), %rax`（64ビット全体をロード）
- `char`: `movsbl (%rdi), %eax`（8ビットを符号拡張）

## `va_end` の実装

`va_end(ap)` は何も行わない。レジスタ保存領域はスタックフレームの一部であり、関数リターン時に自動的にクリーンアップされる。

パーサーでは、`va_end(expr)` は式を評価（副作用のため）して結果を破棄し、`Num(0)` ノードを生成する。

## パーサーの変更

### 省略記号トークン

`...` 用に新しい `Ellipsis` トークン種別を追加:

```rust
// In lexer: recognize three consecutive dots
if ch == '.' && self.peek_next() == Some('.') && self.peek_at(2) == Some('.') {
    self.pos += 3;
    tokens.push(Token { kind: TokenKind::Ellipsis, pos });
    continue;
}
```

### 関数宣言

パーサーは最後の名前付きパラメータの後に `...` があるかチェックする:

```rust
// After parsing named parameters
if self.current().kind == TokenKind::Ellipsis {
    is_variadic = true;
    self.advance();
}
```

### `va_list` の型としての扱い

`va_list` は型キーワードとして認識され、`char *`（char へのポインタ）にマッピングされる:

```rust
if name == "va_list" {
    Type::ptr_to(Type::char_type())
}
```

### 組み込み関数の処理

`va_start`、`va_arg`、`va_end` は `primary()` で特殊な組み込み式としてパースされる:

- `va_start(ap, last_param)` → `Expr::VaStart { ap, last_param }`
- `va_arg(ap, type)` → `Expr::VaArg { ap, ty }`
- `va_end(ap)` → `Expr::Num(0)`（何もしない）

## AST の変更

2つの新しい式ノードを追加:

```rust
pub enum Expr {
    // ...
    VaStart {
        ap: Box<Expr>,
        last_param: String,
    },
    VaArg {
        ap: Box<Expr>,
        ty: Type,
    },
}
```

`Function` 構造体に `is_variadic` フィールドを追加:

```rust
pub struct Function {
    pub name: String,
    pub return_ty: Type,
    pub params: Vec<(Type, String)>,
    pub is_variadic: bool,   // NEW
    pub body: Vec<Stmt>,
    pub locals: Vec<(Type, String)>,
}
```

## コード生成の変更

`Codegen` 構造体に2つの新しいフィールドを追加:

```rust
struct Codegen {
    // ...
    va_save_area_offset: usize,       // offset from rbp to register save area
    current_func_param_count: usize,  // number of named parameters
}
```

### 可変長引数関数のスタックフレームレイアウト

```
                    %rbp
┌─────────────────┐  ↑
│ saved %rbp      │  │
├─────────────────┤  │
│ local variables │  │  (normal locals)
├─────────────────┤  │
│ register params │  │  (named params stored on stack)
├─────────────────┤  │
│ save area (48B) │  │  (all 6 register args saved here)
├─────────────────┤  │
│ alignment pad   │  │
└─────────────────┘  ↓  %rsp
```

## テストケース

```c
// Basic variadic sum
int sum(int n, ...) {
    va_list ap;
    va_start(ap, n);
    int total = 0;
    int i;
    for (i = 0; i < n; i++)
        total += va_arg(ap, int);
    va_end(ap);
    return total;
}
int main() { return sum(3, 10, 20, 30); }  // => 60

int main() { return sum(3, 1, 2, 3); }     // => 6
```

## 制限事項

1. **可変長引数は最大5個**: レジスタ引数は6個しか保存せず、そのうち1つは名前付きパラメータ `n` に使用されるため、アクセスできる可変長引数は5個までとなる。合計7個以上の引数はスタック経由で渡されるが、それらは処理されない。

2. **浮動小数点非対応**: レジスタ保存領域は汎用レジスタのみを保存する。`%xmm0`〜`%xmm7` で渡される浮動小数点引数は保存されない。

3. **簡略化された `va_list`**: 実際の ABI では `gp_offset`、`fp_offset`、`overflow_arg_area`、`reg_save_area` を含む構造体を使用する。本実装では単純な `char *` ポインタを使用している。

4. **スタック渡し引数の非対応**: スタック経由で渡される7番目以降の引数は `va_arg` でアクセスできない。
