# Step 12.1: 関数パラメータとしての配列

## 概要

関数パラメータで `int a[]` と書いた場合、ポインタ `int *a` として扱う（配列からポインタへの退化）。

```c
int sum(int a[], int n) {
    int s = 0;
    for (int i = 0; i < n; i++) s += a[i];
    return s;
}
int main() {
    int a[3] = {1, 2, 3};
    return sum(a, 3);  // => 6
}
```

## C言語の仕様

C言語では関数パラメータの配列宣言はポインタ宣言と等価：
- `int a[]` ≡ `int *a`
- `int a[10]` ≡ `int *a` （サイズは無視される）

これは「配列はポインタに退化する」という原則の一部。

## パーサーの変更

パラメータ名の後に `[]` が来たらポインタ型に変換：

```rust
// Array parameter: int a[] → int *a
if self.current().kind == TokenKind::LBracket {
    self.advance();
    // Skip optional size
    if self.current().kind != TokenKind::RBracket {
        self.advance();
    }
    self.expect(TokenKind::RBracket);
    param_ty = Type::ptr_to(param_ty);
}
```

## テストケース

```bash
assert 6 'int sum(int a[], int n) { ... } int main() { int a[3] = {1, 2, 3}; return sum(a, 3); }'
assert 3 'int first(int a[]) { return a[0]; } int main() { int a[3] = {3, 2, 1}; return first(a); }'
```
