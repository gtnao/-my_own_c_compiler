# Step 7.5: ユニオン

## 概要

ユニオン（`union`）を実装する。ユニオンは構造体と似ているが、全メンバが同じメモリ領域を共有する。

```c
union { int a; int b; } u;
u.a = 42;
return u.b;  // => 42 (same memory)
```

## ユニオンと構造体の違い

### メモリレイアウト

**構造体**: メンバを順次配置、各メンバは異なるオフセット
```
struct { int x; int y; }
offset 0: x (4 bytes)
offset 4: y (4 bytes)
total: 8 bytes
```

**ユニオン**: 全メンバがオフセット0、サイズは最大メンバのサイズ
```
union { int a; long b; }
offset 0: a (4 bytes) }  same memory
offset 0: b (8 bytes) }
total: 8 bytes
```

### 用途

ユニオンは同じメモリ領域を異なる型で解釈するために使われる：
- メモリ節約（同時に使わないフィールドを共有）
- 型パニング（同じビットパターンを別の型で読む）
- バリアント型の実装

## 実装方針

構造体とユニオンの唯一の違いは「メンバのオフセット計算」なので、
内部表現は同じ `TypeKind::Struct(Vec<StructMember>)` を使い、
パーサーでオフセットを変えるだけで実装できる。

### パーサー

`parse_struct_or_union(is_union: bool)` ヘルパーメソッドを追加：

```rust
if is_union {
    // Union: all members at offset 0
    members.push(StructMember { name, ty, offset: 0 });
} else {
    // Struct: sequential with alignment padding
    offset = align_up(offset, ty.align());
    members.push(StructMember { name, ty, offset });
    offset += ty.size();
}
```

### サイズ計算

`types.rs` の `size()` を修正して、「最後のメンバ」ではなく「全メンバの offset + size の最大値」を使う：

```rust
TypeKind::Struct(members) => {
    let raw_size = members.iter()
        .map(|m| m.offset + m.ty.size())
        .max()
        .unwrap_or(0);
    let align = self.align();
    (raw_size + align - 1) & !(align - 1)
}
```

これは構造体では従来と同じ結果（最後のメンバが最大）を返し、
ユニオンでは全メンバの最大サイズを返す。

### コード生成

変更不要。ユニオンは内部的に `TypeKind::Struct` として扱われるため、
既存のメンバアクセスコード（gen_addr + offset加算）がそのまま動作する。
ユニオンの場合はoffsetが0なので `add` 命令はスキップされる。

## トークンとレクサー

```rust
// token.rs
Union,

// lexer.rs
"union" => TokenKind::Union,
```

## テストケース

```bash
# union size is max member size
assert 4 'int main() { return sizeof(union { int a; int b; }); }'
assert 8 'int main() { return sizeof(union { int a; long b; }); }'
assert 4 'int main() { return sizeof(union { char a; int b; }); }'

# members share memory
assert 42 'int main() { union { int a; int b; } u; u.a = 42; return u.b; }'
assert 3 'int main() { union { int x; char y; } u; u.x = 3; return u.y; }'

# tagged union
assert 10 'int main() { union U { int a; int b; }; union U u; u.a = 10; return u.b; }'
```
