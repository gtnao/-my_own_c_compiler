# ステップ 12.2: 構造体の値渡しと値返し

## 概要

構造体を関数に値渡ししたり、関数から返したり、代入でコピーしたりできるようにする。以前は構造体はポインタ（参照セマンティクス）としてのみ渡されていた。このステップでは、`rep movsb` によるバイト単位のメモリコピーを使って、真の値セマンティクスを実装する。

## 主な変更

### 1. 構造体の代入（`s2 = s1`）

代入の左辺が構造体型の場合、両辺のアドレスを計算し、ソースからデスティネーションへ構造体をバイトコピーする:

```rust
// In gen_expr for Expr::Assign
if let TypeKind::Struct(_) = &lhs_ty.kind {
    self.gen_addr(rhs);      // source address → %rax
    self.push();             // save source address
    self.gen_addr(lhs);      // destination address → %rax
    self.emit("mov %rax, %rdi");  // %rdi = dst
    self.pop("%rsi");        // %rsi = src
    self.emit(&format!("mov ${}, %rcx", size));
    self.emit("rep movsb");  // copy size bytes
}
```

### 2. 構造体の値渡し（関数パラメータ）

関数が構造体パラメータを受け取る場合、呼び出し側は構造体のアドレスをレジスタで渡す。呼ばれた側は構造体データを自身のローカルスタック領域にコピーすることで、パラメータへの変更が呼び出し側の元の構造体に影響しないことを保証する:

```asm
; Callee prologue for struct parameter:
  mov %rdi, %rsi         ; src = caller's struct address (from register)
  lea -offset(%rbp), %rdi ; dst = local stack space
  mov $size, %rcx        ; byte count
  rep movsb              ; copy struct into local frame
```

これは値セマンティクスにとって重要である。コピーがなければ、`modify(s)` は呼び出し側の構造体 `s` を変更してしまい、C のセマンティクスに違反する。

### 3. 関数からの構造体返し

関数が構造体を返す場合（`return p;`）、構造体のアドレスが `%rax` に格納される。呼び出し側は `emit_store_var` を使って返された構造体をローカル変数にコピーする。`emit_store_var` は構造体型を検出し、`rep movsb` コピーを実行する。

### 4. 構造体対応の `emit_store_var` 拡張

以前の `emit_store_var` は `Struct(_)` に対して何もしなかった。今回、ターゲット変数が構造体の場合に完全な構造体コピーを行うようになった:

```rust
if let TypeKind::Struct(_) = &ty.kind {
    self.emit("mov %rax, %rsi");    // src = address in %rax
    self.emit(&format!("lea -{}(%rbp), %rdi", offset)); // dst
    self.emit(&format!("mov ${}, %rcx", size));
    self.emit("rep movsb");
    return;
}
```

### 5. 構造体対応の `emit_store_indirect` 拡張

間接的な構造体ストア（ポインタやメンバアクセス経由など）のために、`emit_store_indirect` が構造体型を処理するようになった。`%rax` をソースアドレス、`%rdi` をデスティネーションアドレスとして扱う:

```rust
if let TypeKind::Struct(_) = &ty.kind {
    self.emit("mov %rax, %rsi"); // src
    self.emit(&format!("mov ${}, %rcx", size));
    self.emit("rep movsb");
    return;
}
```

### 6. 構造体の式の値

構造体式が値コンテキストに出現する場合（例: `Expr::Var`、`Expr::Member`、`Expr::Deref`）、式はその内容をロードするのではなく、構造体の **アドレス** に評価される。これは配列からポインタへの暗黙変換と類似している:

- `Expr::Var(name)` が構造体 → `lea -offset(%rbp), %rax`
- `Expr::Member(base, name)` が構造体メンバ → アドレス計算のみ
- `Expr::Deref(ptr)` が構造体ポインタ → ポインタ値を `%rax` に残す

### 7. 独立した構造体定義

変数名なしのトップレベル構造体定義をサポート:

```c
struct P { int x; int y; };  // Just defines the struct tag
```

パーサーは `global_var()` で `parse_type()` の直後に `;` があるかチェックすることでこれを処理する。

## `rep movsb` 命令

`rep movsb` は x86 のストリング命令で、`%rsi`（ソース）から `%rdi`（デスティネーション）へバイトをコピーし、`%rcx` がゼロになるまでデクリメントする:

```
; Before: %rsi = src, %rdi = dst, %rcx = count
rep movsb
; After: %rcx = 0, %rsi and %rdi advanced by count
```

これは任意サイズの構造体コピーに対する最もシンプルなアプローチである。最新の CPU は `rep movsb` を内部的に最適化しており（ERMS - Enhanced REP MOVSB）、ほとんどの構造体サイズに対して手書きのコピーループと同等の性能を発揮する。

## 値セマンティクスの検証

テスト `modify(a)` は値渡しが正しく動作することを確認する:

```c
struct P { int x; int y; };
void modify(struct P p) { p.x = 99; }  // modifies local copy only
int main() {
    struct P a; a.x = 3; a.y = 4;
    modify(a);
    return a.x;  // => 3 (unchanged)
}
```

## テストケース

```c
// Struct assignment
struct { int x; int y; } s1, s2;
s1.x = 1; s1.y = 2; s2 = s1; return s2.x + s2.y;  // => 3

// Tagged struct copy
struct P { int x; int y; };
struct P a; a.x = 3; a.y = 7; struct P b; b = a; return b.x + b.y;  // => 10

// Struct return
struct P make() { struct P p; p.x = 1; p.y = 2; return p; }
struct P r = make(); return r.x + r.y;  // => 3

// Struct pass-by-value
int sum(struct P p) { return p.x + p.y; }
struct P a; a.x = 3; a.y = 4; return sum(a);  // => 7

// Value semantics (no aliasing)
void modify(struct P p) { p.x = 99; }
struct P a; a.x = 3; modify(a); return a.x;  // => 3

// Struct with mixed types
struct S { char c; int n; };
int get(struct S s) { return s.c; }
struct S s; s.c = 97; s.n = 42; return get(s);  // => 97
```
