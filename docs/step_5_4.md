# Step 5.4: 多次元配列

## 概要

`int a[2][3]` のような多次元配列を実装する。
C言語の多次元配列は「配列の配列」として表現され、
メモリ上では行優先（row-major）に連続配置される。

```c
int a[2][3];
a[0][0] = 1; a[0][1] = 2; a[0][2] = 3;
a[1][0] = 4; a[1][1] = 5; a[1][2] = 6;
```

## 型の表現

### Array(Array(Int, 3), 2) の構造

`int a[2][3]` は内部的に `Array(Array(Int, 3), 2)` と表現される。

```
Array(Array(Int, 3), 2)
  ├─ 外側: 要素数 2 の配列
  └─ 要素型: Array(Int, 3)
      ├─ 内側: 要素数 3 の配列
      └─ 要素型: Int
```

- `sizeof(a)` = 2 × sizeof(int[3]) = 2 × 12 = 24
- `sizeof(a[0])` = sizeof(int[3]) = 12
- `a[0]` の型: `Array(Int, 3)`（= int[3]）

### パースの順序

`int a[2][3]` のパース時、次元を左から右に読み取り、
型は右（内側）から左（外側）に構築する。

```rust
// Parse dimensions left to right: [2], [3]
let mut dims = Vec::new();
while self.current().kind == TokenKind::LBracket {
    // ... parse each dimension
    dims.push(len);
}
// dims = [2, 3]

// Build type right to left:
// Int → Array(Int, 3) → Array(Array(Int, 3), 2)
let mut ty = base_ty;
for &len in dims.iter().rev() {
    ty = Type::array_of(ty, len);
}
```

なぜ逆順に構築するのか: C言語では `a[2][3]` は
「2個の要素があり、各要素は3個のintからなる配列」を意味する。
最も内側の次元（[3]）が要素型に最も近い。

## 添字アクセスの仕組み

### a[i][j] の脱糖

`a[i][j]` は2段階の添字脱糖で処理される：

1. `a[i]` → `*(a + i)` — 型: `Array(Int, 3)` → `int[3]`
2. `(a[i])[j]` → `*(*(a + i) + j)` — 型: `Int` → `int`

### 型の伝搬

```
式            型                    意味
a             Array(Array(Int,3),2) 配列全体
a + i         Ptr(Array(Int,3))     行ポインタ (i * 12 バイトオフセット)
*(a + i)      Array(Int, 3)         i番目の行 (int[3])
*(a + i) + j  Ptr(Int)              要素ポインタ (j * 4 バイトオフセット)
*(*(a+i) + j) Int                   要素値
```

### Array-to-Pointer Decay の重要性

中間の `*(a + i)` の結果型は `Array(Int, 3)` だが、
このときメモリからの値のロードは行わない。
配列型の「値」はそのアドレスそのもの（ポインタに暗黙変換される）。

```rust
// emit_load_indirect で Array 型は no-op
TypeKind::Array(_, _) => {} // address is the value
```

これにより `*(a + i)` は行のアドレスを %rax に保持したまま、
次の `+ j` でそのアドレスに要素オフセットを加算できる。

## メモリレイアウト

```c
int a[2][3];  // 24バイト (4 * 3 * 2)
```

```
    rbp-24  rbp-20  rbp-16  rbp-12  rbp-8   rbp-4
    a[0][0] a[0][1] a[0][2] a[1][0] a[1][1] a[1][2]
    |-------- a[0] ---------|-------- a[1] ---------|
    |------------------- a -----------------------|
```

- `a` のアドレス: rbp-24
- `a[0]` のアドレス: rbp-24（= a と同じ）
- `a[1]` のアドレス: rbp-24 + 1×12 = rbp-12
- `a[1][2]` のアドレス: rbp-24 + 1×12 + 2×4 = rbp-4

## 生成されるアセンブリ例

`int a[2][3]; a[1][2] = 42;` の場合：

```asm
# a[1][2] = 42
# 脱糖: *(*(a + 1) + 2) = 42

  mov $42, %rax           # rhs = 42
  push %rax               # save rhs

  # gen_addr(Deref(Add(Deref(Add(Var(a), 1)), 2)))
  # → gen_expr(Add(Deref(Add(Var(a), 1)), 2))

  # 内側: Deref(Add(Var(a), 1))
  mov $1, %rax            # index = 1
  push %rax
  lea -24(%rbp), %rax     # array decay: &a
  pop %rdi                # rdi = 1
  imul $12, %rdi          # 1 * sizeof(int[3]) = 12
  add %rdi, %rax          # rax = &a[1]
  # emit_load_indirect(Array(Int,3)) → no-op!

  # 外側: Add(..., 2)
  push %rax               # save &a[1]
  mov $2, %rax            # ... wait, eval order
  # (actual: rhs first, then lhs)
  ...
  imul $4, %rdi           # 2 * sizeof(int) = 4
  add %rdi, %rax          # rax = &a[1][2]

  mov %rax, %rdi          # address in %rdi
  pop %rax                # value 42 in %rax
  movl %eax, (%rdi)       # store int to a[1][2]
```

## テスト

ユニットテスト 22 件 + 統合テスト 189 件（5 件追加）= 211 件
