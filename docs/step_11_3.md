# ステップ 11.3: 関数ポインタ

## 概要

このステップでは関数ポインタを実装し、関数を変数に格納して間接的に呼び出せるようにする。関数ポインタはコールバック、ディスパッチテーブル、高階プログラミングパターンの実装に不可欠である。

主な機能:
- 関数ポインタの宣言: `int (*fp)(int, int)`
- 関数からポインタへの暗黙変換: `fp = add`（裸の関数名がポインタになる）
- 間接関数呼び出し: `fp(3, 4)` でポインタ経由の呼び出し

## 関数ポインタの宣言構文

C の関数ポインタ構文は複雑なことで有名である:

```c
int (*fp)(int, int);
```

これは次のように読む: `fp` は `(int, int)` を受け取り `int` を返す **関数** への **ポインタ** (`*`) である。

`*fp` を囲む括弧が重要で、括弧がなければ `int *fp(int, int)` は `int *` を返す関数の宣言になってしまう。

## 実装

### 型の表現

関数ポインタは内部的に `Ptr(Void)` として表現される（単純な8バイトポインタ）。型システムでは関数の完全なシグネチャを追跡しない。この簡略化が成立する理由は:

1. x86-64 の関数ポインタはシグネチャに関係なく常に8バイト
2. 呼び出し規約はすべての関数型で同一
3. 関数ポインタ引数の型チェックはまだ実装されていない

### AST の変更

間接呼び出しと直接呼び出しを区別するために、新しい `FuncPtrCall` 式ノードを追加:

```rust
pub enum Expr {
    // Direct function call: call label
    FuncCall { name: String, args: Vec<Expr> },
    // Indirect function call: call *%r10
    FuncPtrCall { fptr: Box<Expr>, args: Vec<Expr> },
    // ...
}
```

### パーサーの変更

#### 関数ポインタ宣言のパース

`var_decl()` で基本型をパースした後、`(*` パターンをチェックして関数ポインタ宣言を識別する:

```rust
// After parse_type() returns the return type
if current == '(' && next == '*' {
    parse_func_ptr_decl(return_ty);
}
```

`parse_func_ptr_decl` メソッドの処理:
1. `(` `*` `name` `)` を消費する
2. パラメータ型リスト `(type, type, ...)` をパースする
3. `Ptr(Void)` 型の変数を作成する
4. オプションの初期化子 `= expr` を処理する

#### 関数からポインタへの暗黙変換

関数名が式として出現した場合（例: `fp = add`）、`add` が宣言済みの変数でなければ、`emit_load_var` はそれを関数名として扱い、以下を生成する:

```asm
lea add(%rip), %rax
```

これは RIP 相対アドレッシングを使って関数のアドレスを `%rax` にロードする。

#### 直接呼び出しと間接呼び出しの区別

`primary()` で `name(args)` をパースする際:
- `name` が宣言済みの変数 → `FuncPtrCall`（間接呼び出し）
- それ以外 → `FuncCall`（直接呼び出し）

`is_var_declared()` メソッドはローカルスコープとグローバル変数をチェックするが、`extern` 宣言（ポインタ変数ではなく関数プロトタイプ）は除外する。

### コード生成

#### 直接呼び出し（既存）

```asm
call function_name
```

#### 関数ポインタ経由の間接呼び出し

```asm
  mov -8(%rbp), %rax    # load function pointer from variable
  mov %rax, %r10        # save to %r10 (caller-saved, not used for args)

  # ... set up arguments in registers ...

  mov $0, %al           # clear AL (no vector register args)
  call *%r10            # indirect call through %r10
```

関数ポインタの保持に `%r10` を使用する理由:
- caller-saved レジスタである（呼ばれた側が自由に書き換えてよい）
- 引数渡しに使用されない（引数は `%rdi`〜`%r9` を使用）
- 引数評価コード（`%rax`、`%rdi`、スタックのみ使用）によって上書きされない

`call *%r10` 命令は間接呼び出しを行う。`%r10` からアドレスを読み取り、そこにジャンプする。

### スタックアライメント

間接呼び出しも直接呼び出しと同じ16バイトスタックアライメント要件に従う。アライメントチェックでは `stack_depth` を使って、呼び出し前に追加の8バイトパディングが必要かどうかを判定する。

## 使用例

### C ソースコード

```c
int add(int a, int b) { return a + b; }
int main() {
    int (*fp)(int, int) = add;
    return fp(3, 4);
}
```

### 生成されるアセンブリ

```asm
  .globl main
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # int (*fp)(int, int) = add;
  lea add(%rip), %rax     # get address of 'add' function
  mov %rax, -8(%rbp)      # store in fp variable

  # fp(3, 4)
  mov -8(%rbp), %rax      # load fp
  mov %rax, %r10          # save to %r10
  mov $3, %rax            # arg 1
  push %rax
  mov $4, %rax            # arg 2
  push %rax
  pop %rsi                # arg 2 → %rsi
  pop %rdi                # arg 1 → %rdi
  mov $0, %al
  call *%r10              # indirect call

  jmp .Lreturn.main
```

## テストケース

```c
// Basic function pointer call
int add(int a, int b) { return a + b; }
int main() { int (*fp)(int, int) = add; return fp(3, 4); }  // => 7

// Function pointer with different function
int sub(int a, int b) { return a - b; }
int main() { int (*fp)(int, int) = sub; return fp(5, 3); }  // => 2

// Nullary function pointer
int ret42() { return 42; }
int main() { int (*fp)() = ret42; return fp(); }  // => 42
```

## 制限事項

1. **シグネチャの追跡なし**: 型システムは関数ポインタを `Ptr(Void)` として格納するため、引数の型や個数のコンパイル時型チェックは行われない。

2. **関数ポインタ型の `typedef` 非対応**: `typedef int (*BinOp)(int, int);` のようなパターンはまだサポートされていない。

3. **`%r10` の上書きリスク**: 関数ポインタ呼び出しの引数にさらに関数呼び出しが含まれる場合、`%r10` レジスタが上書きされる可能性がある。ただし、引数式が単純なものである通常の使用では発生しない。
