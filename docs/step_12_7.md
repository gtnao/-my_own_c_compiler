# ステップ 12.7: 複合型宣言

## 概要

ポインタと配列を組み合わせた複合型宣言をサポートする:

- **ポインタの配列**: `int *arr[3]` — `arr` は3つの `int*` 要素を持つ配列
- **配列へのポインタ**: `int (*p)[3]` — `p` は `int[3]` へのポインタ

## ポインタの配列

`int *arr[3]` は既存のパーサーで既にサポートされていた。`parse_type()` 関数が `int*` を生成し、次に変数宣言のパースが `arr[3]` を処理して `Array(Ptr(Int), 3)` を作成する。

```c
int a = 1, b = 2, c = 3;
int *arr[3];
arr[0] = &a; arr[1] = &b; arr[2] = &c;
return *arr[0] + *arr[1] + *arr[2];  // => 6
```

`arr` の各要素は x86-64 では8バイトのポインタである。`arr[i]` はポインタを取得し、`*arr[i]` はそれをデリファレンスする。

## 配列へのポインタ

`int (*p)[3]` は括弧が束縛を変えるため、特別なパースが必要である。括弧がなければ `int *p[3]` はポインタの配列になってしまう。`(*p)` がポインタ宣言子をグループ化する。

### パース

既存の `parse_func_ptr_decl` を `parse_func_ptr_or_array_ptr_decl` にリネームして拡張した。`(*name)` をパースした後、次のトークンで型を判定する:

- `(` → 関数ポインタ: `type (*name)(param_types)`
- `[` → 配列へのポインタ: `type (*name)[size]`

```rust
if self.current().kind == TokenKind::LBracket {
    // Array pointer: type (*name)[size]
    self.advance();
    let size = /* parse size */;
    self.expect(TokenKind::RBracket);
    let arr_ty = Type::array_of(base_ty, size);
    let ptr_ty = Type::ptr_to(arr_ty);
    // declare variable with ptr_ty
} else {
    // Function pointer: type (*name)(param_types)
    // ... existing logic
}
```

### 型の構築

`int (*p)[3]` の場合:
1. 基本型: `int`
2. 配列型: `Array(Int, 3)` — 3つの int からなる12バイトの配列
3. ポインタ型: `Ptr(Array(Int, 3))` — その配列への8バイトポインタ

### 使用方法

```c
int a[3] = {10, 20, 30};
int (*p)[3] = &a;    // p points to the whole array
return (*p)[1];       // dereference p to get the array, then index → 20
```

`*p` は配列を生成し（ポインタに暗黙変換される）、`[1]` でインデックスアクセスする。

## C の宣言の読み方

C の宣言を読むための「時計回り/スパイラル」ルール:

| 宣言 | 読み方 | 型 |
|---|---|---|
| `int *a[3]` | a は int へのポインタの3要素配列 | `Array(Ptr(Int), 3)` |
| `int (*a)[3]` | a は int の3要素配列へのポインタ | `Ptr(Array(Int, 3))` |
| `int (*f)(int)` | f は int を受け取り int を返す関数へのポインタ | `Ptr(Void)`（簡略化） |

## テストケース

```c
// Pointer to array
int a[3] = {10, 20, 30};
int (*p)[3] = &a;
return (*p)[1];  // => 20

// Array of pointers
int a=1, b=2, c=3;
int *arr[3]; arr[0]=&a; arr[1]=&b; arr[2]=&c;
return *arr[0] + *arr[1] + *arr[2];  // => 6
```
