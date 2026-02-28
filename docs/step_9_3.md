# Step 9.3: 指示子付き初期化子

## 概要

C99で導入された指示子付き初期化子（designated initializer）を実装する。配列では `[idx] = val`、構造体では `.member = val` の形式で、初期化する要素やメンバを明示的に指定できる。

```c
// Array designated initializer
int a[5] = {[2] = 10, [4] = 20};
// a = {0, 0, 10, 0, 20}

// Struct designated initializer
struct { int a; int b; int c; } s = {.b = 20, .c = 30};
// s.a = undefined, s.b = 20, s.c = 30
```

## 仕組み

### 配列指示子 `[idx] = val`

配列初期化子のパース中に `[` を検出した場合：

1. `[` を消費
2. インデックス（整数リテラル）を読む
3. `]` と `=` を消費
4. 値の式をパース
5. そのインデックスへの代入を生成

指示子と非指示子を混在させることもできる：
```c
int a[5] = {1, 2, [3] = 20, 4};
// a[0]=1, a[1]=2, a[3]=20, a[4]=4
```
非指示子の要素は、直前の指示子の次のインデックスから順番に配置される。

#### 実装の詳細

```rust
let mut indexed_exprs: Vec<(usize, Expr)> = Vec::new();
let mut seq_idx: usize = 0;   // sequential index for non-designated
let mut max_idx: usize = 0;   // track max index for empty bracket arrays

while self.current().kind != TokenKind::RBrace {
    if self.current().kind == TokenKind::LBracket {
        // Designated: [idx] = val
        let idx = parse_num();
        self.expect(RBracket);
        self.expect(Eq);
        let val = self.assign();
        indexed_exprs.push((idx, val));
        seq_idx = idx + 1;  // next sequential starts after this
    } else {
        // Sequential
        let val = self.assign();
        indexed_exprs.push((seq_idx, val));
        seq_idx += 1;
    }
    // track max for empty bracket size inference
    if seq_idx > max_idx { max_idx = seq_idx; }
}
```

空ブラケット `int a[] = {[4] = 10}` の場合、`max_idx` から配列サイズを推定する（この場合5）。

### 構造体指示子 `.member = val`

構造体初期化子のパース中に `.` を検出した場合：

1. `.` を消費
2. メンバ名（識別子）を読む
3. `=` を消費
4. 値の式をパース
5. そのメンバへの代入を生成

```rust
if self.current().kind == TokenKind::Dot {
    self.advance();
    let mem_name = parse_ident();
    self.expect(Eq);
    let val = self.assign();
    // Generate: s.member = val
    stmts.push(assign_to_member(unique, mem_name, val));
    // Update sequential index
    seq_idx = member_position(mem_name) + 1;
}
```

指示子と非指示子を混在させた場合、非指示子は直前の指示子メンバの次のメンバに配置される。

### desugar（展開）

指示子付き初期化子も、最終的には通常の初期化子と同様に個別の代入文に展開される：

```c
struct { int a; int b; int c; } s = {.c = 30, .a = 10};
```

→ 以下に展開：

```c
struct { int a; int b; int c; } s;
s.c = 30;
s.a = 10;
```

## テストケース

```bash
# Array designated initializer
assert 10 'int main() { int a[5] = {[2] = 10}; return a[2]; }'
assert 0 'int main() { int a[5] = {[2] = 10}; return a[0]; }'
assert 20 'int main() { int a[5] = {1, 2, [3] = 20, 4}; return a[3]; }'
assert 4 'int main() { int a[5] = {1, 2, [3] = 20, 4}; return a[4]; }'

# Struct designated initializer
assert 30 'int main() { struct { int a; int b; int c; } s = {.b = 20, .c = 30}; return s.c; }'
assert 20 'int main() { struct { int a; int b; int c; } s = {.b = 20, .c = 30}; return s.b; }'
assert 5 'int main() { struct { int x; int y; } p = {.x = 5, .y = 10}; return p.x; }'
```
