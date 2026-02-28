# Step 5.3: 配列

## 概要

配列型 `int a[3]` を実装する。C言語の配列は連続するメモリ領域で、
添字演算子 `a[i]` は `*(a + i)` に脱糖（desugar）される。
これにより、Step 5.1（ポインタ）と Step 5.2（ポインタ算術）の
上に配列を構築できる。

```c
int a[3];
a[0] = 1;
a[1] = 2;
a[2] = 3;
return a[0] + a[1] + a[2]; // 6
```

## 配列型の内部表現

### TypeKind に Array を追加

```rust
pub enum TypeKind {
    Void, Bool, Char, Short, Int, Long,
    Ptr(Box<Type>),
    Array(Box<Type>, usize),  // Array(要素型, 要素数)
}
```

- `size()`: `base.size() * len` — 要素サイズ × 要素数
- `align()`: `base.align()` — 要素型のアライメントに従う
- `base_type()`: Ptr と同様に `Some(base)` を返す
- `is_pointer()`: `true` を返す（配列はポインタ的に使える）

### なぜ is_pointer() が true なのか

C言語では、配列名は「ほとんどの文脈で」先頭要素へのポインタに暗黙変換（decay）される。
例外は `sizeof` と `&` の場合のみ。このため、ポインタ算術（`a + i`）や
関数に渡す際に配列がポインタとして動作する必要がある。
`is_pointer()` が true を返すことで、BinOp::Add/Sub のポインタスケーリングが
配列にも自動的に適用される。

## 添字演算子の脱糖

### a[i] → *(a + i)

パーサーの `postfix()` メソッドで添字演算子を処理し、
内部的に `Deref(BinOp::Add(a, i))` に変換する。

```rust
// postfix = primary ("[" expr "]" | "++" | "--")*
TokenKind::LBracket => {
    self.advance();
    let index = self.expr();
    self.expect(TokenKind::RBracket);
    // a[i] is *(a + i)
    node = Expr::Deref(Box::new(Expr::BinOp {
        op: BinOp::Add,
        lhs: Box::new(node),
        rhs: Box::new(index),
    }));
}
```

この脱糖により、配列アクセスは既存のポインタ算術 + デリファレンスの
コード生成パスを完全に再利用できる。新しい AST ノードは不要。

### なぜ脱糖が有効なのか

C言語の仕様上、`a[i]` は `*(a + i)` と厳密に等価。
コンパイラの実装として、添字を専用のノードにする代わりに脱糖すると：

1. コード生成の追加実装が不要（Add + Deref の組み合わせ）
2. ポインタ算術のスケーリングが自動適用される
3. `p[i]`（ポインタ変数の添字）も同じ仕組みで動作する

## Array-to-Pointer Decay（配列のポインタ変換）

### 配列変数の式としての評価

配列変数が式として使われるとき、その値は「配列の先頭アドレス」である。
通常の変数は値をロードする（`mov`命令）が、配列は値ではなく
アドレスを計算する（`lea`命令）。

```rust
// emit_load_var の配列対応
TypeKind::Array(_, _) => {
    // Array-to-pointer decay: load address of the array
    self.emit(&format!("  lea -{}(%rbp), %rax", offset));
}
```

### lea vs mov の違い

```asm
# int a = 5;
movslq -4(%rbp), %rax    # メモリから値 5 をロード

# int a[3]; (配列変数 a を式として使う)
lea -12(%rbp), %rax      # メモリのアドレスを %rax に格納
```

- `mov`: メモリの **内容** を読み出す
- `lea`: メモリの **アドレス** を計算する（Load Effective Address）

配列は「まとまったメモリ領域」なので、値としては意味がない。
式として使うとき、配列名はその先頭アドレス（= 先頭要素へのポインタ）に変換される。

## 式の型推論の拡張

ポインタ算術と配列を正しく組み合わせるため、`expr_type()` を拡張した。

### Deref の型推論

```rust
Expr::Deref(inner) => {
    let inner_ty = self.expr_type(inner);
    match inner_ty.kind {
        TypeKind::Ptr(base) | TypeKind::Array(base, _) => *base,
        _ => Type::long_type(),
    }
}
```

Ptr だけでなく Array も処理。`*(int_array)` の結果型は `int`。

### BinOp の型推論

```rust
Expr::BinOp { op, lhs, rhs } => {
    let lhs_ty = self.expr_type(lhs);
    let rhs_ty = self.expr_type(rhs);
    match op {
        BinOp::Add => {
            if lhs_ty.is_pointer() {
                Type::ptr_to(lhs_ty.base_type().unwrap().clone())
            } else if rhs_ty.is_pointer() {
                Type::ptr_to(rhs_ty.base_type().unwrap().clone())
            } else {
                Type::long_type()
            }
        }
        // ...
    }
}
```

`int a[3]; a + 1` の型推論:
1. `a` の型は `Array(Int, 3)`
2. `a + 1` の結果型は `Ptr(Int)`（ポインタ型に変換）
3. `*(a + 1)` の結果型は `Int`

これにより、`a[1] = 5` の代入で `emit_store_indirect(&Int)` が呼ばれ、
正しく `movl %eax, (%rdi)` が生成される。

## メモリレイアウト例

```c
int a[3];  // 12バイト (4 * 3), 4バイトアライメント
```

```
                rbp
                 |
   ... [-12] [-8] [-4] [rbp]
       a[0]  a[1] a[2]
```

`a` (= `&a[0]`) のアドレスは `rbp - 12`。
`a[1]` は `*(a + 1)` = `*(rbp-12 + 1*4)` = `*(rbp-8)`。
`a[2]` は `*(a + 2)` = `*(rbp-12 + 2*4)` = `*(rbp-4)`。

## 生成されるアセンブリ例

`int a[3]; a[1] = 42; return a[1];` の場合：

```asm
# a[1] = 42
# Desugar: *(a + 1) = 42
# Assign { lhs: Deref(Add(Var(a), Num(1))), rhs: Num(42) }

  mov $42, %rax           # rhs = 42
  push %rax               # save rhs

  # gen_addr for Deref(Add(Var(a), Num(1)))
  # → gen_expr for Add(Var(a), Num(1))
  mov $1, %rax            # rhs of Add = 1
  push %rax
  lea -12(%rbp), %rax     # lhs of Add = &a[0] (array decay)
  pop %rdi                # rdi = 1
  imul $4, %rdi           # 1 * sizeof(int) = 4
  add %rdi, %rax          # rax = &a[0] + 4 = &a[1]

  mov %rax, %rdi          # address in %rdi
  pop %rax                # value 42 in %rax
  movl %eax, (%rdi)       # store int value to a[1]

# return a[1]
  mov $1, %rax
  push %rax
  lea -12(%rbp), %rax     # array decay
  pop %rdi
  imul $4, %rdi
  add %rdi, %rax          # rax = &a[1]
  movslq (%rax), %rax     # load int value from a[1] (sign-extend)
  jmp .Lreturn.main
```

## 変更のまとめ

| ファイル | 変更内容 |
|---------|---------|
| `src/types.rs` | `size()` で `match &self.kind` に修正（Array の Box 借用のため） |
| `src/parser.rs` | `postfix()` に `[expr]` → `*(... + expr)` 脱糖を追加 |
| `src/codegen.rs` | Array パターンを全 match 文に追加、array decay で `lea` 生成、`expr_type()` に BinOp/Array 対応追加 |
| `tests/test.sh` | 配列テスト 8 件追加 |

## テスト

ユニットテスト 22 件 + 統合テスト 184 件（8 件追加）= 206 件
