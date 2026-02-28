# Step 12.3: 文字列初期化による配列

## 概要

`char s[] = "hello";` のように文字列リテラルでchar配列を初期化する機能を実装する。

```c
char s[] = "hello";
// s[0]='h', s[1]='e', s[2]='l', s[3]='l', s[4]='o', s[5]='\0'
// sizeof(s) = 6
```

## 仕組み

文字列リテラルでchar配列を初期化する場合、文字列の各バイトを配列の要素に代入する形に展開（desugar）する。null終端文字も含む。

### 変換例

```c
char s[] = "hi";
```

→ 以下に展開：

```c
char s[3];   // "hi" + '\0' = 3 bytes
s[0] = 'h';
s[1] = 'i';
s[2] = '\0';
```

## パーサーの変更

`var_decl()` の初期化子パースで、文字列リテラルかつ配列型の場合に特別処理：

```rust
if matches!(token, TokenKind::Str(_)) && is_array_type {
    // Parse string bytes
    // Determine array size (bytes.len() + 1 for null terminator)
    // Generate VarDecl + byte-by-byte assignment
    // Include null terminator at the end
}
```

### `char *s = "hello"` との区別

`char *s = "hello"` はポインタ変数への文字列リテラルの代入であり、配列初期化ではない。条件分岐で配列型（`has_empty_bracket` または `TypeKind::Array`）の場合のみ文字列初期化を行う。

## テストケース

```bash
assert 104 'int main() { char s[] = "hello"; return s[0]; }'  # 'h'
assert 111 'int main() { char s[] = "hello"; return s[4]; }'  # 'o'
assert 0 'int main() { char s[] = "hello"; return s[5]; }'    # '\0'
assert 6 'int main() { char s[] = "hello"; return sizeof(s); }'
```
