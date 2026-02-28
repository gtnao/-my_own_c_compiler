# Step 7.2: アラインメントとパディング

## 概要

構造体のアラインメントとパディングが正しく動作することを検証するステップ。Step 7.1で既にアラインメント計算を実装済みだが、より多様な型の組み合わせでの正確性を確認する。

## アラインメントルール

x86-64 System V ABI における各型のアラインメント要件：

| 型 | サイズ | アラインメント |
|---|---|---|
| `char` / `_Bool` | 1 byte | 1 byte |
| `short` | 2 bytes | 2 bytes |
| `int` | 4 bytes | 4 bytes |
| `long` / ポインタ | 8 bytes | 8 bytes |

構造体のアラインメントは、全メンバの最大アラインメントとなる。

## パディングの発生パターン

### パターン1: メンバ間パディング

```c
struct { char a; int b; }
```

```
offset 0: a (char, 1 byte)
offset 1: [padding 3 bytes]  ← intの4バイトアラインメントのため
offset 4: b (int, 4 bytes)
total: 8 bytes, align: 4
```

### パターン2: 末尾パディング

```c
struct { char a; int b; char c; }
```

```
offset 0: a (char, 1 byte)
offset 1: [padding 3 bytes]
offset 4: b (int, 4 bytes)
offset 8: c (char, 1 byte)
offset 9: [padding 3 bytes]  ← 構造体サイズを4バイトに切り上げ
total: 12 bytes, align: 4
```

末尾パディングが発生する理由：配列 `struct S arr[N]` を使う場合、各要素のアラインメントが正しくなるように構造体サイズは自身のアラインメントの倍数でなければならない。

### パターン3: longメンバによる8バイトアラインメント

```c
struct { char a; long b; }
```

```
offset 0: a (char, 1 byte)
offset 1: [padding 7 bytes]  ← longの8バイトアラインメントのため
offset 8: b (long, 8 bytes)
total: 16 bytes, align: 8
```

### パターン4: longメンバ + 末尾パディング

```c
struct { char a; long b; char c; }
```

```
offset 0: a (char, 1 byte)
offset 1: [padding 7 bytes]
offset 8: b (long, 8 bytes)
offset 16: c (char, 1 byte)
offset 17: [padding 7 bytes]  ← 構造体アラインメント8バイトのため
total: 24 bytes, align: 8
```

### パターン5: パディングなし

```c
struct { char a; char b; short c; }
```

```
offset 0: a (char, 1 byte)
offset 1: b (char, 1 byte)
offset 2: c (short, 2 bytes)  ← shortは2バイトアラインメント、既に満たされている
total: 4 bytes, align: 2
```

## アラインメント計算の実装

`types.rs` で実装済みの計算：

### メンバオフセットの計算（パーサーで実行）

```rust
let align = mem_ty.align();
offset = (offset + align - 1) & !(align - 1);  // round up to alignment
```

ビットマスクによる効率的な切り上げ。例えば `align = 8` の場合：
- `!(8 - 1) = !0b0...0111 = 0b1...1000`
- `offset = 1` → `(1 + 7) & !7 = 8 & !7 = 8`

### 構造体サイズの計算

```rust
TypeKind::Struct(members) => {
    let last = &members[members.len() - 1];
    let raw_size = last.offset + last.ty.size();
    let align = self.align();
    (raw_size + align - 1) & !(align - 1)  // round up to struct alignment
}
```

### 構造体アラインメントの計算

```rust
TypeKind::Struct(members) => {
    members.iter().map(|m| m.ty.align()).max().unwrap_or(1)
}
```

## テストケース

```bash
# long member → 8-byte alignment, 16 bytes total
assert 16 'int main() { return sizeof(struct { char a; long b; }); }'

# no padding needed (char + char + short fits perfectly)
assert 4 'int main() { return sizeof(struct { char a; char b; short c; }); }'

# tail padding (char after int needs 3 bytes padding for struct alignment)
assert 12 'int main() { return sizeof(struct { char a; int b; char c; }); }'

# large tail padding with long alignment
assert 24 'int main() { return sizeof(struct { char a; long b; char c; }); }'

# minimal struct with no padding
assert 2 'int main() { return sizeof(struct { char a; char b; }); }'

# verify actual member access with alignment
assert 42 'int main() { struct { char a; long b; } s; s.b = 42; return s.b; }'
assert 3 'int main() { struct { char a; int b; char c; } s; s.a = 1; s.b = 2; s.c = 3; return s.c; }'
```
