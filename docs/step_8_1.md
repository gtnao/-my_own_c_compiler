# Step 8.1: enum（列挙型）

## 概要

C言語の列挙型（`enum`）を実装する。列挙子はコンパイル時に整数定数として解決される。

```c
enum { A, B, C };    // A=0, B=1, C=2
enum { X=10, Y, Z }; // X=10, Y=11, Z=12
return B;            // => 1
```

## enumの仕様

### 自動採番

列挙子に値を指定しない場合、前の列挙子+1が自動で割り当てられる。最初の列挙子のデフォルトは0。

```c
enum { A, B, C };  // A=0, B=1, C=2
```

### 明示的な値

列挙子に `= 値` で明示的な値を指定できる。以降の列挙子はそこから+1される。

```c
enum { A=10, B, C=20, D };  // A=10, B=11, C=20, D=21
```

### 型

enumの型は `int`（4バイト）として扱われる。`sizeof(enum { ... })` は4を返す。

### タグ

`enum Tag { ... }` でタグを付けられるが、Step 8.1ではタグは無視する（パースはするが保存しない）。

## 実装

### トークンとレクサー

```rust
// token.rs
Enum,

// lexer.rs
"enum" => TokenKind::Enum,
```

### パーサー

**enum定数の辞書**を追加：

```rust
pub struct Parser<'a> {
    // ...
    enum_values: HashMap<String, i64>,
}
```

**parse_type()** で `Enum` を処理：

```rust
TokenKind::Enum => {
    self.advance();
    // Skip optional tag name
    if Ident { self.advance(); }
    // Parse body
    if LBrace {
        let mut val = 0;
        while not RBrace {
            let name = parse_ident();
            if Eq { self.advance(); val = parse_num(); }
            enum_values.insert(name, val);
            val += 1;
            if Comma { self.advance(); }
        }
        expect(RBrace);
    }
    // enum type is int
    Type::int_type()
}
```

**primary()** で列挙定数を解決：

```rust
TokenKind::Ident(name) => {
    // ...
    // Check for enum constant before variable lookup
    if let Some(&val) = self.enum_values.get(&name) {
        return Expr::Num(val);
    }
    // Normal variable lookup
    let resolved = self.resolve_var(&name);
    Expr::Var(resolved)
}
```

列挙定数は `Expr::Num(val)` に変換されるため、コード生成では単なる整数リテラルと同じ `mov $val, %rax` が生成される。

### コード生成

変更不要。列挙定数はパーサーの段階で `Expr::Num` に解決されるため、コード生成から見ると整数リテラルと同一。

### is_function() の修正

`enum` キーワードの後にはオプションのタグ名と本体 `{ ... }` が続く可能性があるため、`is_function()` でこれらをスキップする処理を追加。

## テストケース

```bash
# automatic numbering
assert 0 'int main() { enum { A, B, C }; return A; }'
assert 1 'int main() { enum { A, B, C }; return B; }'
assert 2 'int main() { enum { A, B, C }; return C; }'

# explicit values
assert 10 'int main() { enum { X = 10, Y, Z }; return X; }'
assert 11 'int main() { enum { X = 10, Y, Z }; return Y; }'
assert 12 'int main() { enum { X = 10, Y, Z }; return Z; }'

# single constant
assert 5 'int main() { enum { A = 5 }; return A; }'

# sizeof
assert 4 'int main() { return sizeof(enum { A, B }); }'
```
