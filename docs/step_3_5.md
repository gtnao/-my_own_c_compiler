# Step 3.5: 変数スコープ（ブロックスコープ、シャドウイング）

## 概要

C言語のブロックスコープを実装する。ブロック `{}` 内で宣言された変数は、
そのブロック内でのみ有効であり、外側の同名変数をシャドウできる。

```c
int main() {
    int a = 1;
    { int a = 2; }   // 内側の a は別の変数
    return a;         // => 1（外側の a）
}
```

## ブロックスコープのルール

1. **変数の可視性**: 変数は宣言されたブロックとその内側のブロックでのみ可視
2. **シャドウイング**: 内側のブロックで同名の変数を宣言すると、外側の変数を隠す
3. **スコープ終了**: ブロックを出ると、そのブロックで宣言された変数は不可視になり、外側の同名変数が再び見える
4. **代入**: シャドウされていない変数への代入は、最も近いスコープの変数を変更する

### 例

```c
int a = 1;           // a → outer
{
    int a = 2;       // a → inner (outer を隠す)
    // ここで a は inner の 2
}
// ここで a は outer の 1（inner は消えた）

{
    a = 3;           // inner の a がないので outer の a を変更
}
// ここで a は 3
```

## 実装アプローチ: スコープスタック + 名前解決

### 方針

パーサーにスコープスタックを導入し、**パース時に**全ての変数参照を解決する。
コード生成は変更不要。

各変数には一意の内部名を割り当てる：
- 最初の `a` → 内部名 `"a"`
- 2番目の `a`（シャドウ）→ 内部名 `"a.1"`
- 3番目の `a` → 内部名 `"a.2"`

これにより、コード生成は名前だけで変数を区別できる。

### パーサーの新しいフィールド

```rust
pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    reporter: &'a ErrorReporter,
    locals: Vec<String>,                       // 全変数（一意名）のリスト
    scopes: Vec<HashMap<String, String>>,      // NEW: スコープスタック
    unique_counter: usize,                     // NEW: 一意名生成用カウンタ
}
```

### スコープ操作メソッド

```rust
fn enter_scope(&mut self) {
    self.scopes.push(HashMap::new());
}

fn leave_scope(&mut self) {
    self.scopes.pop();
}
```

### 変数宣言 (`declare_var`)

```rust
fn declare_var(&mut self, name: &str) -> String {
    // 同名の変数が既に存在する場合、一意な内部名を生成
    let unique = if self.locals.iter().any(|l| l == name) {
        self.unique_counter += 1;
        format!("{}.{}", name, self.unique_counter)
    } else {
        name.to_string()
    };

    // locals に追加（スタックスロットを確保）
    self.locals.push(unique.clone());

    // 現在のスコープに名前マッピングを登録
    self.scopes.last_mut().unwrap()
        .insert(name.to_string(), unique.clone());

    unique
}
```

### 変数参照の解決 (`resolve_var`)

```rust
fn resolve_var(&self, name: &str) -> String {
    // 内側のスコープから外側に向かって検索
    for scope in self.scopes.iter().rev() {
        if let Some(unique) = scope.get(name) {
            return unique.clone();
        }
    }
    // 見つからない場合はそのまま返す
    name.to_string()
}
```

## 変更箇所

### 1. 関数パース開始時

スコープスタックを初期化し、関数レベルスコープを作成：

```rust
self.scopes.clear();
self.unique_counter = 0;
self.enter_scope();     // 関数スコープ
```

パラメータもこのスコープに宣言される。

### 2. ブロック文

ブロックの前後でスコープを push/pop：

```rust
TokenKind::LBrace => {
    self.advance();
    self.enter_scope();         // NEW
    // ... parse statements ...
    self.expect(TokenKind::RBrace);
    self.leave_scope();         // NEW
    Stmt::Block(stmts)
}
```

### 3. 変数宣言

`declare_var()` を使用して一意名を生成：

```rust
fn var_decl(&mut self) -> Stmt {
    self.expect(TokenKind::Int);
    let name = /* parse identifier */;
    let unique = self.declare_var(&name);   // NEW
    let init = /* parse init expr */;
    Stmt::VarDecl { name: unique, init }    // 一意名を使用
}
```

### 4. 変数参照

`resolve_var()` で現在のスコープに基づいて解決：

```rust
TokenKind::Ident(name) => {
    self.advance();
    if /* function call */ { ... }
    let resolved = self.resolve_var(&name);  // NEW
    Expr::Var(resolved)                       // 解決済みの名前を使用
}
```

## 具体例: 動作の追跡

入力: `int main() { int a = 1; { int a = 2; } return a; }`

### パース時

1. 関数スコープ開始: `scopes = [{}]`
2. `int a = 1;`: `declare_var("a")` → `"a"`, `scopes = [{"a": "a"}]`, `locals = ["a"]`
3. `{`: `enter_scope()`, `scopes = [{"a": "a"}, {}]`
4. `int a = 2;`: `declare_var("a")` → `"a.1"` (衝突), `scopes = [{"a": "a"}, {"a": "a.1"}]`, `locals = ["a", "a.1"]`
5. `}`: `leave_scope()`, `scopes = [{"a": "a"}]`
6. `return a;`: `resolve_var("a")` → `scopes[0]` の `"a"` → `Expr::Var("a")`

### コード生成時

- `locals = ["a", "a.1"]`
- `"a"` → offset 8, `"a.1"` → offset 16

```asm
main:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp

  # int a = 1;  (内部名: "a", offset 8)
  mov $1, %rax
  mov %rax, -8(%rbp)

  # { int a = 2; }  (内部名: "a.1", offset 16)
  mov $2, %rax
  mov %rax, -16(%rbp)   # 別のスロット！

  # return a;  (内部名: "a", offset 8)
  mov -8(%rbp), %rax    # → 1
  jmp .Lreturn.main
```

## 設計のポイント

### なぜパース時に解決するか

- **コード生成に変更不要**: codegen は名前でスロットを引くだけ。スコープの概念不要
- **シンプル**: 変数参照は解決済みの一意名を使うだけ
- **正しさ**: スコープルールの正しさをパーサー内で保証

### `locals` は grow-only

ブロックスコープを出ても、`locals` から変数を削除しない。
スタックスロットは関数全体で一度確保され、再利用はしない。
これはメモリ効率は良くないが、実装がシンプルで正しい。

実際の C コンパイラ（GCC や clang）も最適化なしではこの方式を使うことが多い。
スタックスロットの再利用はレジスタ割り付けの一部として後で最適化できる。

### 代入のスコープ解決

`a = 3;` のような代入文は、`assign()` → `Expr::Assign { lhs: Var("a"), ... }` として
パースされる。`Var("a")` の時点で `resolve_var("a")` が呼ばれ、
現在のスコープで最も近い `a` の一意名が使われる。

これにより:
- シャドウされている場合: 内側の変数が変更される
- シャドウされていない場合: 外側の変数が変更される

```c
int a = 1;
{ a = 3; }   // 外側の a を変更 → a は 3
return a;     // => 3
```
