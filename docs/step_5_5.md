# Step 5.5: グローバル配列

## 概要

グローバルスコープでの配列宣言を実装する。

```c
int a[3];
int b[2][3];
int main() {
    a[0] = 1;
    b[1][2] = 42;
    return a[0] + b[1][2];
}
```

## 実装

### パーサーの変更

`global_var()` メソッドに配列次元のパースを追加。
ローカル変数の `var_decl()` と同じく、`[size]` を0個以上読み取り
型を内側から構築する。

```rust
fn global_var(&mut self) {
    let ty = self.parse_type();
    let name = ...;
    // Array dimensions
    let mut dims = Vec::new();
    while self.current().kind == TokenKind::LBracket {
        self.advance();
        let len = ...;
        self.advance();
        self.expect(TokenKind::RBracket);
        dims.push(len);
    }
    let mut ty = ty;
    for &len in dims.iter().rev() {
        ty = Type::array_of(ty, len);
    }
    self.expect(TokenKind::Semicolon);
    self.globals.push((ty, name));
}
```

### コード生成

コード生成の変更は不要。既存の仕組みで自動的に動作する：

1. **`.comm` ディレクティブ**: `ty.size()` と `ty.align()` が
   Array 型に対応済み。`int a[3]` なら `.comm a, 12, 4` が生成される。

2. **RIP相対アドレッシング**: `emit_load_var` の Array パターンが
   `lea name(%rip), %rax` を生成（array-to-pointer decay）。

3. **添字アクセス**: `a[i]` は `*(a + i)` に脱糖され、
   ポインタ算術のスケーリングが適用される。

### 生成されるアセンブリ例

```c
int a[3];
int main() { a[1] = 42; return a[1]; }
```

```asm
  .comm a, 12, 4          # 12バイト, 4バイトアライメント

  # a[1] = 42
  mov $42, %rax
  push %rax
  mov $1, %rax            # index
  push %rax
  lea a(%rip), %rax       # array decay (global)
  pop %rdi
  imul $4, %rdi           # scale by sizeof(int)
  add %rdi, %rax          # &a[1]
  mov %rax, %rdi
  pop %rax
  movl %eax, (%rdi)       # store
```

## なぜ追加の変更が最小限なのか

Step 5.3–5.4 で配列の基盤（型システム、添字脱糖、array decay、
emit_load_indirect の no-op）を構築済み。グローバル配列は
既存のグローバル変数の仕組み（`.comm`、RIP相対）と配列の仕組みの
組み合わせで動作するため、パーサーの変更のみで実現できた。

## テスト

ユニットテスト 22 件 + 統合テスト 193 件（4 件追加）= 215 件
