# Step 9.5: グローバル変数の静的初期化

## 概要

グローバル変数に初期値を持たせられるようにする。初期値付きグローバル変数は `.data` セクションに配置し、初期値なしのグローバル変数は従来通り `.bss` セクション（`.comm`）に配置する。

```c
int g = 42;
int arr[3] = {1, 2, 3};
char msg[] = "hello";
```

## `.data` セクションと `.bss` セクション

### `.bss` セクション（初期値なし）

ゼロ初期化される変数。ファイルサイズに影響しない（サイズ情報のみ記録）。

```asm
  .comm g, 4, 4    # 4バイト、4バイトアライメント
```

### `.data` セクション（初期値あり）

初期値を持つ変数。実際のバイト列がバイナリに埋め込まれる。

```asm
  .data
  .align 4
  .globl g
g:
  .byte 42,0,0,0   # int g = 42; (リトルエンディアン)
  .text
```

## リトルエンディアン

x86-64はリトルエンディアン（下位バイトが低いアドレス）。`int g = 42;` の場合：

```
アドレス:  g+0  g+1  g+2  g+3
値:        42    0    0    0
```

42 = 0x0000002A なので、バイト列は `[0x2A, 0x00, 0x00, 0x00]`。

## パーサーの変更

### `global_var()` メソッドの拡張

`=` トークンがある場合、イニシャライザをパースして生バイト列に変換：

```rust
// Scalar: int g = 42;
// → bytes: [42, 0, 0, 0]

// Array: int g[3] = {1, 2, 3};
// → bytes: [1,0,0,0, 2,0,0,0, 3,0,0,0]

// String: char g[] = "hello";
// → bytes: [104,101,108,108,111,0]
```

#### バイト変換

整数値 `val` を `elem_size` バイトのリトルエンディアンに変換：

```rust
for i in 0..elem_size {
    bytes.push(((val >> (i * 8)) & 0xff) as u8);
}
```

#### 空ブラケット配列

`int g[] = {1, 2, 3};` は初期化子の要素数から配列サイズを推定：

```rust
let ty = if matches!(ty.kind, TypeKind::Array(_, 0)) {
    let base = ty.base_type().unwrap().clone();
    Type::array_of(base, vals.len())
} else {
    ty
};
```

### AST の変更

`Program.globals` のタプルに `Option<Vec<u8>>` を追加：

```rust
pub globals: Vec<(Type, String, Option<Vec<u8>>)>,
```

- `Some(bytes)`: 初期化済み → `.data` セクション
- `None`: 未初期化 → `.comm`（`.bss` セクション）

## コード生成の変更

```rust
for (ty, name, init) in &program.globals {
    if let Some(bytes) = init {
        // Initialized: .data section
        self.emit("  .data");
        self.emit(&format!("  .align {}", ty.align()));
        self.emit(&format!("  .globl {}", name));
        self.emit(&format!("{}:", name));
        self.emit(&format!("  .byte {}", byte_strs.join(",")));
        self.emit("  .text");
    } else {
        // Uninitialized: .bss (via .comm)
        self.emit(&format!("  .comm {}, {}, {}", name, ty.size(), ty.align()));
    }
}
```

`.data` セクションに出力後、`.text` セクションに戻すことで、後続の関数コードが正しいセクションに配置される。

## テストケース

```bash
assert 42 'int g = 42; int main() { return g; }'
assert 3 'int a = 1; int b = 2; int main() { return a + b; }'
assert 8 'int g[3] = {1, 2, 3}; int main() { return g[0] + g[1] + g[2] + g[0] * g[1]; }'
assert 104 'char s[] = "hello"; int main() { return s[0]; }'
assert 0 'char s[] = "hello"; int main() { return s[5]; }'
```
