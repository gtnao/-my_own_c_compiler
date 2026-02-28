# Step 5.2: ポインタ算術

## 概要

ポインタに対する加減算を実装する。C言語のポインタ算術では、
整数のオフセットが自動的に「指す先の型のサイズ」でスケーリングされる。

```c
int a[4];
int *p = &a[0];
p + 1;     // 次の int へ（アドレス的には +4 バイト）
p + 2;     // 2つ先の int へ（+8 バイト）
```

## ポインタ算術の3パターン

### 1. ptr + int / int + ptr

ポインタに整数を加算。整数は `sizeof(*ptr)` 倍にスケーリングされる。

```c
int *p = ...;
p + 3;   // 実際のアドレス: p + 3 * sizeof(int) = p + 12
```

### 2. ptr - int

ポインタから整数を減算。同様にスケーリング。

```c
int *p = ...;
p - 1;   // 実際のアドレス: p - 1 * sizeof(int) = p - 4
```

### 3. ptr - ptr

2つのポインタの差。バイト差を `sizeof(*ptr)` で割って要素数を返す。

```c
int a, b;
int *pa = &a, *pb = &b;
pa - pb;  // 1 (隣接する int 変数なので 4 バイト差 / sizeof(int) = 1)
```

## 実装

BinOp の Add と Sub の処理を拡張。`expr_type()` で
オペランドがポインタかどうかを判定し、スケーリングを追加する。

### Add のコード生成

```rust
BinOp::Add => {
    if lhs_ty.is_pointer() {
        // ptr + int: scale rhs by sizeof(*ptr)
        let size = lhs_ty.base_type().unwrap().size();
        if size > 1 {
            self.emit(&format!("  imul ${}, %rdi", size));
        }
    } else if rhs_ty.is_pointer() {
        // int + ptr: scale lhs by sizeof(*ptr)
        let size = rhs_ty.base_type().unwrap().size();
        if size > 1 {
            self.emit(&format!("  imul ${}, %rax", size));
        }
    }
    self.emit("  add %rdi, %rax");
}
```

この時点で `%rax = lhs`, `%rdi = rhs`。
ポインタ側はそのまま、整数側をスケーリングしてから加算する。

### Sub のコード生成

```rust
BinOp::Sub => {
    if lhs_ty.is_pointer() && rhs_ty.is_pointer() {
        // ptr - ptr: byte diff / sizeof(*ptr)
        self.emit("  sub %rdi, %rax");
        let size = ...;
        if size > 1 {
            self.emit(&format!("  mov ${}, %rdi", size));
            self.emit("  cqto");
            self.emit("  idiv %rdi");
        }
    } else if lhs_ty.is_pointer() {
        // ptr - int: scale rhs
        let size = ...;
        if size > 1 {
            self.emit(&format!("  imul ${}, %rdi", size));
        }
        self.emit("  sub %rdi, %rax");
    } else {
        self.emit("  sub %rdi, %rax");
    }
}
```

### 生成されるアセンブリ例

`int *p = &a; p = p + 2;` の場合：

```asm
# p + 2 の計算
mov $2, %rax          # rhs = 2
push %rax
mov -16(%rbp), %rax   # lhs = p (ポインタ値)
pop %rdi              # rdi = 2
imul $4, %rdi         # 2 * sizeof(int) = 8
add %rdi, %rax        # p + 8
```

`char *p` の場合は `sizeof(char) = 1` なので `imul` は省略される
（`size > 1` のガード）。

## なぜスケーリングが必要か

C言語の設計思想として、ポインタ算術は「要素単位」で考える。

```c
int arr[4] = {10, 20, 30, 40};
int *p = arr;
*(p + 2);  // 30（3番目の要素）
```

プログラマは「2要素先」と考え、コンパイラが
`2 * sizeof(int) = 8` バイトのオフセットに変換する。
これにより、型のサイズを意識せずに配列を走査できる。

## テスト

ユニットテスト 22 件 + 統合テスト 176 件（2 件追加）= 198 件
