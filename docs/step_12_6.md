# ステップ 12.6: for ループのスコープ

## 概要

`for` ループの初期化句で宣言された変数のスコープを修正する。C99 以降では、for ループの初期化で宣言された変数はループ本体にスコープが限定される:

```c
int i = 100;
for (int i = 0; i < 5; i++) {
    // inner i is 0..4
}
return i;  // should be 100, not 5
```

## 問題

この修正前は、`for (int i = 0; ...)` は `i` を外側のスコープで宣言していた。外側に変数 `i` が存在する場合、for ループの `i` がそれを永続的にシャドウイングし、ループ終了後も外側の `i` はループの値を持ったままだった。

## 修正

for ループの初期化句に変数宣言が含まれる場合、新しいスコープで囲む:

```rust
let has_decl_init = self.is_type_start(&self.current().kind.clone());
if has_decl_init {
    self.enter_scope();
}

// ... parse init, cond, inc, body ...

if has_decl_init {
    self.leave_scope();
}
```

これにより、`for(int i = ...)` で宣言された `int i` は外側の `i` と衝突しないユニークなマングル名を取得する。スコープ終了時に内側の `i` はスコープマップから除去され、以降の `i` への参照は外側の変数に解決される。

## スコープの仕組み

コンパイラは名前マングリングを伴うスコープスタックを使用する:
- `enter_scope()` は新しい HashMap をスコープスタックにプッシュ
- `declare_var("i", ...)` は `i__2` のようなユニークな名前を作成し、`"i"` → `"i__2"` をマッピング
- `resolve_var("i")` は最も内側のスコープから外側に向かって検索
- `leave_scope()` は最も内側のスコープをポップ

for ループスコープの修正なし:
```
Scope: { i → i__0 }
for (int i = 0; ...) → declares i__1, but in the SAME scope!
After loop: i still resolves to i__1 (the loop variable)
```

修正あり:
```
Scope: { i → i__0 }
enter_scope() → new scope: { }
for (int i = 0; ...) → declares i__1 in inner scope: { i → i__1 }
leave_scope() → removes inner scope
After loop: i resolves to i__0 (the outer variable) ✓
```

## テストケース

```c
int main() { int i = 100; for (int i = 0; i < 5; i++) {} return i; }  // => 100
int main() { int s = 0; for (int i = 0; i < 10; i++) s += i; return s; }  // => 45
```
