# Step 14.3: `__attribute__`サポート（GCC拡張）

## 概要

GCCの`__attribute__`構文を、属性アノテーションを黙って消費・無視することでサポートします。これは実際のCコード、特にPostgreSQLやシステムヘッダのコンパイルに不可欠であり、`__attribute__`が広く使用されています。

## 実コードでの一般的な使用例

```c
__attribute__((unused)) int x;
__attribute__((noreturn)) void fatal(const char *msg);
__attribute__((format(printf, 1, 2))) void log_msg(const char *fmt, ...);
__attribute__((packed)) struct S { ... };
__attribute__((aligned(16))) int data[4];
__attribute__((noinline)) int compute(int x);
```

## 実装

### トークン

`__attribute__`キーワードを認識する`Attribute`トークン種別をレキサーに追加しました。

### パーサー: `skip_attribute()`

括弧の深さを追跡しながら`__attribute__((...))` を消費する新しいメソッド:

```rust
fn skip_attribute(&mut self) {
    while self.current().kind == TokenKind::Attribute {
        self.advance(); // __attribute__
        if self.current().kind == TokenKind::LParen {
            self.advance(); // outer (
            let mut depth = 1;
            while depth > 0 {
                match self.current().kind {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    _ => {}
                }
                self.advance();
            }
        }
    }
}
```

二重括弧`((...))` は深さカウンタによって自然に処理されます — 外側の`(`でdepthが1に、内側の`(`で2になり、閉じ括弧`))`で0に戻ります。

### 属性のスキップ箇所

`skip_attribute()`は以下の箇所で呼び出されます:
1. **`parse_type()`の前** — `__attribute__((unused)) int x`を処理
2. **`parse_type()`内のポインタのアスタリスクの後** — `int * __attribute__((may_alias)) p`を処理
3. **`function_or_prototype()`内の関数パラメータリスト`)`の後** — `void f(int x) __attribute__((noreturn))`を処理
4. **`is_function()`の先読み内** — 宣言が関数かどうかを判定する際に属性を正しくスキップ

### `is_type_keyword`と`is_function`

`Attribute`を`is_type_keyword`に追加し、型の前の`__attribute__`が型宣言の一部として認識されるようにしました。`is_function`の先読みも`Alignas`と同様に`Attribute`を処理し、括弧内のコンテンツをスキップします。

## 動作

すべての属性アノテーションは消費されて黙って無視されます。コンパイラは属性のセマンティクス（アラインメント、フォーマットチェック、noreturnなど）を強制したり実行したりしません。これは互換性に必要な動作と一致しています — 属性はGCC/Clangにとって情報提供的なものであり、ほとんどの場合、正しいコンパイルには必須ではありません。

## テストケース

```c
int main() __attribute__((unused)) { return 42; }  // after function name
__attribute__((unused)) int main() { return 5; }   // before return type
int __attribute__((noinline)) add(int a, int b) { return a + b; }  // between type and name
```
