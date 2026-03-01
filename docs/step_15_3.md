# ステップ 15.3: 複雑な型宣言子

## 概要

このステップでは、複雑なCの型宣言を扱えるようパーサーを拡張する:
- **ポインタ配列**: `int *arr[3]` — intへのポインタの3要素配列
- **配列ポインタ**: `int (*p)[3]` — 3要素int配列へのポインタ
- **関数ポインタ配列**: `int (*ops[2])(int, int)` — 2個の関数ポインタの配列

## Cの宣言構文

Cの宣言構文は「宣言は使用を反映する」という原則に従う。括弧が含まれる場合、宣言子には2つの形式がある:

### `int *arr[3]` — ポインタ配列（既に動作済み）
`*` はベース型に結合し、`[3]` は宣言子の一部である。`parse_type()` が `int *` を `Ptr(Int)` として読み取り、その後 `var_decl()` が `arr[3]` をパースして `Array(Ptr(Int), 3)` を作成する。

### `int (*p)[3]` — 配列ポインタ
括弧が `*p` をグループ化するため、`p` は `int[3]` へのポインタになる。`parse_func_ptr_or_array_ptr_decl()` がこれを処理する: `(*name)` の後に `[3]` を検出し、`Ptr(Array(Int, 3))` を作成する。

### `int (*ops[2])(int, int)` — 関数ポインタ配列
重要なポイント: `ops[2]` は `*` と共に括弧の内側にあり、`ops` の各要素がポインタであることを意味する。`(int, int)` の接尾辞によって関数ポインタとなる。結果: `Array(Ptr(Void), 2)` — 2個の関数ポインタの配列。

## 実装

`parse_func_ptr_or_array_ptr_decl()` メソッドを拡張し、括弧内の配列次元を処理できるようにした:

```rust
fn parse_func_ptr_or_array_ptr_decl(&mut self, base_ty: Type) -> Stmt {
    self.expect(TokenKind::LParen);  // (
    self.expect(TokenKind::Star);    // *
    // ... parse name ...

    // Check for array dimension inside parens: (*name[N])
    let mut array_size: Option<usize> = None;
    if self.current().kind == TokenKind::LBracket {
        self.advance();
        let size = self.eval_const_expr();
        self.expect(TokenKind::RBracket);
        array_size = Some(size as usize);
    }

    self.expect(TokenKind::RParen);  // )

    // Then dispatch based on what follows:
    // - [N] → array pointer (without inner array_size)
    // - (params) → function pointer or function pointer array (with inner array_size)
}
```

`array_size` が `Some(N)` で `(params)` が続く場合、`Array(Ptr(Void), N)` を作成する — 関数ポインタ配列である。各8バイトの要素は汎用的な関数ポインタとなる。

## テストケース

```c
// Pointer array
int main() {
    int a=1, b=2, c=3;
    int *arr[3];
    arr[0]=&a; arr[1]=&b; arr[2]=&c;
    return *arr[1];  // => 2
}

// Array pointer
int main() {
    int arr[3] = {10, 20, 30};
    int (*p)[3] = &arr;
    return (*p)[1];  // => 20
}

// Function pointer array
int add(int a, int b) { return a+b; }
int sub(int a, int b) { return a-b; }
int main() {
    int (*ops[2])(int, int);
    ops[0] = add;
    ops[1] = sub;
    return ops[0](10, 3);  // => 13
}
```
