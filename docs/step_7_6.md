# Step 7.6: ネストされた構造体/ユニオン

## 概要

構造体のメンバとして別の構造体やユニオンを持てることを確認・テストするステップ。

```c
struct {
    struct { int x; int y; } inner;
    int z;
} s;
s.inner.x = 1;
s.inner.y = 2;
s.z = 3;
return s.inner.x + s.inner.y + s.z;  // => 6
```

## なぜ追加実装なしで動作するか

構造体のネストは、既存の実装が再帰的な構造になっているため自然に動作する：

1. **parse_type()** は再帰的に呼ばれるため、メンバの型として `struct { ... }` を解析できる
2. **StructMember** の `ty` フィールドに `TypeKind::Struct(...)` が入り、サイズ・アラインメント計算も再帰的に動作する
3. **gen_addr** の `Member` 処理が再帰的にアドレスを計算するため、`s.inner.x` は以下のように展開される：
   - `s.inner` → `gen_addr(s)` + `inner` のオフセット加算
   - `.x` → さらに `x` のオフセット加算
4. **expr_type** の `Member` 処理が再帰的に型を追跡するため、メンバの型が正しく推論される

### メモリレイアウト例

```c
struct { struct { int x; int y; } inner; int z; }
```

```
offset 0: inner (struct, 8 bytes)
  offset 0: x (int, 4 bytes)
  offset 4: y (int, 4 bytes)
offset 8: z (int, 4 bytes)
total: 12 bytes, align: 4
```

### コード生成例: `s.inner.x`

```asm
# gen_addr for s.inner.x
#   = gen_addr(Member(Member(Var("s"), "inner"), "x"))
# 1. gen_addr(Member(Var("s"), "inner"))
#    → gen_addr(Var("s")) = lea -12(%rbp), %rax
#    → offset of inner = 0, no add
# 2. Member "x" of inner struct
#    → expr_type of s.inner = Struct([x:int@0, y:int@4])
#    → offset of x = 0, no add
# Result: lea -12(%rbp), %rax  (address of s.inner.x)
```

### 構造体内のユニオン

```c
struct { union { int a; int b; } u; int c; } s;
s.u.a = 42;
return s.u.b;  // => 42 (union members share memory)
```

ユニオンメンバは全てオフセット0なので、`u.a` と `u.b` は同じアドレスを参照する。

### タグ付きネスト

```c
struct O { struct I { int x; } inner; };
struct O o;
o.inner.x = 5;
```

内部の構造体も独立したタグとして登録されるため、`struct I` を別の場所で参照することも可能。

## テストケース

```bash
# nested struct access with chained dots
assert 6 'int main() { struct { struct { int x; int y; } inner; int z; } s;
  s.inner.x = 1; s.inner.y = 2; s.z = 3;
  return s.inner.x + s.inner.y + s.z; }'

# sizeof nested struct
assert 12 'int main() { return sizeof(struct { struct { int x; int y; } inner; int z; }); }'

# union inside struct
assert 42 'int main() { struct { union { int a; int b; } u; int c; } s;
  s.u.a = 42; return s.u.b; }'

# tagged nested struct
assert 5 'int main() { struct O { struct I { int x; } inner; };
  struct O o; o.inner.x = 5; return o.inner.x; }'
```
