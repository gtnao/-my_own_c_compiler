# Step 5.1: アドレス演算子 `&` とデリファレンス `*`

## 概要

ポインタの基礎となる2つの演算子を実装する：
- `&expr` — 変数のアドレスを取得
- `*expr` — ポインタが指す先の値を読む/書く

```c
int a = 3;
int *p = &a;    // p は a のアドレスを保持
return *p;      // p が指す先（= a）の値を返す → 3

*p = 10;        // ポインタ経由で a を書き換え
// a は 10 になる
```

## 型システムの拡張

### TypeKind に Ptr を追加

```rust
pub enum TypeKind {
    Void, Bool, Char, Short, Int, Long,
    Ptr(Box<Type>),  // pointer to base type
}
```

`Ptr` は再帰的に型を保持する。`int **pp` は
`Ptr(Box(Ptr(Box(Int))))` と表現される。

### ポインタのサイズとアライメント

x86-64 では全てのポインタは 8 バイト：

```rust
TypeKind::Ptr(_) => 8,  // size
TypeKind::Ptr(_) => 8,  // align
```

ポインタが何を指していても、ポインタ自体のサイズは常に 8 バイト。
これは64ビットアドレス空間を持つ x86-64 の特性。

### コンストラクタ

```rust
pub fn ptr_to(base: Type) -> Self {
    Self { kind: TypeKind::Ptr(Box::new(base)), is_unsigned: false }
}
```

## パーサーの変更

### ポインタ型の構文

```
type = ("unsigned")? base_type ("*")*
```

base_type パース後に `*` を0個以上消費し、ネストした Ptr 型を構築：

```rust
// Parse pointer stars: type "*"*
while self.current().kind == TokenKind::Star {
    self.advance();
    ty = Type::ptr_to(ty);
}
```

`int *p` → `Ptr(Int)`, `int **pp` → `Ptr(Ptr(Int))`

### `&` と `*` の単項演算子

`unary()` に2つの新しいケースを追加：

```rust
TokenKind::Amp => {
    self.advance();
    Expr::Addr(Box::new(self.unary()))
}
TokenKind::Star => {
    self.advance();
    Expr::Deref(Box::new(self.unary()))
}
```

### `*` の多義性と解決

`*` トークンは2つの意味を持つ：
- **乗算**: `a * b`（二項演算子、`mul()` で処理）
- **デリファレンス**: `*p`（前置単項演算子、`unary()` で処理）

再帰下降パーサーの構造により、これは自然に解決される：

```
expr → ... → mul → unary → postfix → primary
```

- `mul()` がオペランドとして `unary()` を呼ぶ
- `unary()` は前置位置の `*` をデリファレンスとして処理
- `mul()` は中置位置の `*` を乗算として処理

### is_function() の改良

`int *foo()` のようなポインタ返り値の関数に対応するため、
型キーワードの後に `*` もスキップするよう変更：

```rust
// Skip pointer stars
while self.tokens[i].kind == TokenKind::Star {
    i += 1;
}
```

## AST の変更

```rust
pub enum Expr {
    // ...
    Addr(Box<Expr>),    // &expr
    Deref(Box<Expr>),   // *expr
}
```

`UnaryOp` に追加するのではなく独立したノードにする理由：
- `Addr` は値の評価ではなくアドレスの計算で、コード生成が根本的に異なる
- `Deref` は代入の左辺（lvalue）としても使えるため、`Assign` との連携が必要

## コード生成

### gen_addr: アドレスの計算

新メソッド `gen_addr(expr)` を追加。
式の「値」ではなく「アドレス」を %rax に入れる：

```rust
fn gen_addr(&mut self, expr: &Expr) {
    match expr {
        Expr::Var(name) => {
            if global → lea name(%rip), %rax
            else     → lea -offset(%rbp), %rax
        }
        Expr::Deref(inner) => {
            // *p のアドレス = p の値
            self.gen_expr(inner);
        }
        _ => { /* error */ }
    }
}
```

#### lea 命令（Load Effective Address）

`lea` は「メモリアドレスを計算するが、メモリにはアクセスしない」命令：

```asm
lea -8(%rbp), %rax    # rbp-8 のアドレスを %rax に入れる
                       # メモリの読み書きは行わない

mov -8(%rbp), %rax    # rbp-8 のメモリから値を読む（比較用）
```

`&a` は「a のアドレスを取得する」操作であり、`lea` がまさにこの目的に合致する。

### Addr 式のコード生成

```rust
Expr::Addr(inner) => {
    self.gen_addr(inner);  // inner のアドレスを %rax に
}
```

`&a` の生成コード例（a がスタック上の rbp-8 にある場合）：
```asm
lea -8(%rbp), %rax
```

### Deref 式のコード生成（rvalue）

```rust
Expr::Deref(inner) => {
    self.gen_expr(inner);        // ポインタ値を %rax に
    let ty = self.expr_type(expr);
    self.emit_load_indirect(&ty);  // (%rax) から値をロード
}
```

`*p` の生成コード例：
```asm
mov -16(%rbp), %rax     # p の値（アドレス）を %rax に
movslq (%rax), %rax     # そのアドレスから int を読む
```

### 間接ロード命令（emit_load_indirect）

ポインタが指す先の型に応じた命令を選択：

```rust
fn emit_load_indirect(&mut self, ty: &Type) {
    match ty.kind {
        TypeKind::Bool          => "movzbl (%rax), %eax",
        TypeKind::Char (signed) => "movsbq (%rax), %rax",
        TypeKind::Char (unsigned) => "movzbl (%rax), %eax",
        TypeKind::Short (signed)  => "movswq (%rax), %rax",
        TypeKind::Int (signed)    => "movslq (%rax), %rax",
        TypeKind::Long | Ptr(_)   => "mov (%rax), %rax",
    }
}
```

### 代入の拡張（lvalue としての Deref）

`*p = 10;` のような「ポインタ経由の書き込み」に対応：

```rust
Expr::Assign { lhs: Deref(inner), rhs } => {
    gen_expr(rhs);           // 値を %rax に
    push();                  // 値を退避
    gen_addr(lhs);           // lhs のアドレスを %rax に
    mov %rax, %rdi           // アドレスを %rdi に
    pop(%rax);               // 値を %rax に復元
    emit_store_indirect(&ty); // %rax の値を (%rdi) に格納
}
```

生成コード例（`*p = 10;`）：
```asm
mov $10, %rax         # rhs の値
push %rax             # 退避
mov -16(%rbp), %rax   # p の値（= a のアドレス）
mov %rax, %rdi        # アドレスを %rdi に
pop %rax              # 10 を %rax に復元
movl %eax, (%rdi)     # *p = 10（int サイズで書き込み）
```

### expr_type: 式の型推論

Deref/Addr 式の型を正しく決定するため、簡易的な型推論を追加：

```rust
fn expr_type(&self, expr: &Expr) -> Type {
    match expr {
        Expr::Var(name) => self.get_var_type(name),
        Expr::Deref(inner) => {
            let inner_ty = self.expr_type(inner);
            match inner_ty.kind {
                TypeKind::Ptr(base) => *base,
                _ => Type::long_type(),
            }
        }
        Expr::Addr(inner) => {
            Type::ptr_to(self.expr_type(inner))
        }
        _ => Type::long_type(),
    }
}
```

これにより `int *p` を `*p` でデリファレンスすると、型推論が
`Ptr(Int)` → `Int` と解決し、`movslq` （int 用ロード命令）が正しく生成される。

## メモリレイアウト例

```c
int main() {
    int a = 3;      // rbp-4 (4 bytes)
    int *p = &a;    // rbp-16 (8 bytes, aligned to 8)
    return *p;
}
```

スタックフレーム：
```
rbp-0  : (saved rbp)
rbp-4  : a (int, 4 bytes)
rbp-8  : (padding for alignment)
rbp-16 : p (int*, 8 bytes)
```

## テスト

ユニットテスト 22 件 + 統合テスト 174 件（4 件追加）= 196 件
