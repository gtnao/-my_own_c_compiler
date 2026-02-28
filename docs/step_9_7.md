# Step 9.7: extern宣言

## 概要

`extern` 宣言を実装する。`extern` は変数や関数が他の翻訳単位（別のCファイル）で定義されていることを示す宣言で、ストレージを割り当てない。

```c
extern int g;           // variable declaration (no storage)
extern int printf();    // function declaration (no storage)
```

## extern の意味

### 変数の extern

```c
extern int g;  // 「gはどこかで定義されている」という宣言
int g = 5;     // gの実際の定義（ストレージを割り当て）
```

`extern int g;` はコンパイラに「`g` は `int` 型のグローバル変数で、リンク時に解決される」と伝える。`.comm` や `.data` ディレクティブは生成しない。

### 関数の extern

```c
extern int printf();  // printf は libc で定義されている
```

関数プロトタイプに `extern` を付けることもできる。既存のプロトタイプ宣言と同等の効果。

## パーサーの変更

### extern の検出

トップレベルで `extern` キーワードを検出し、型と名前をパース：

```rust
if self.current().kind == TokenKind::Extern {
    self.advance();
    let ty = self.parse_type();
    let name = parse_ident();

    // Skip function prototype: extern int foo(int, int);
    if self.current().kind == TokenKind::LParen {
        // Skip parenthesized parameter list
    }
    self.expect(TokenKind::Semicolon);

    // Register as global (for type info) but mark as extern
    self.extern_names.insert(name.clone());
    self.globals.push((ty, name, None));
}
```

### extern_names セット

`extern` で宣言された名前を追跡するための `HashSet<String>` をパーサーに追加。

## コード生成の変更

グローバル変数のストレージ出力時に、extern 宣言（初期化子なし + extern_names に含まれる）をスキップ：

```rust
for (ty, name, init) in &program.globals {
    // Skip extern declarations without definition
    if init.is_none() && program.extern_names.contains(name) {
        continue;
    }
    // Emit .data or .comm as usual
}
```

### extern → 定義のパターン

```c
extern int g;  // declaration
int g = 5;     // definition
```

この場合、`g` は extern_names に含まれるが、2つ目の宣言で初期化子付きの定義が追加される。コード生成では初期化子付きの定義を `.data` セクションに出力し、初期化子なしの extern 宣言はスキップする。重複出力を防ぐため、`emitted_globals` セットで既出の名前を追跡する。

## テストケース

```bash
# extern + definition
assert 5 'extern int g; int g = 5; int main() { return g; }'

# extern function (printf from libc)
assert 0 'extern int printf(); int main() { printf("hello"); return 0; }'
```
