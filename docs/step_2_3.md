# Step 2.3: 複数文字の変数名

## 概要

Step 2.2 では単一文字の変数（`a`, `b`）だけをテストしていたが、
実際には Step 2.1 で `Ident(String)` を導入した時点で、
任意の長さの識別子 `[a-zA-Z_][a-zA-Z0-9_]*` は既にサポートされている。

このステップでは複数文字の変数名（`foo`, `bar`, `hello`, `a_b` 等）が
正しく動作することをテストで確認する。

## 識別子の字句解析ルール

レキサーの `read_ident()` は以下のルールで識別子を読み取る：

```
識別子 = [a-zA-Z_] [a-zA-Z0-9_]*
```

1. 先頭文字が英字またはアンダースコア → 識別子の開始
2. 以降、英数字またはアンダースコアが続く限り読み進める
3. 読み取った文字列がキーワード（`int`, `return`）に一致すればキーワードトークンに
4. そうでなければ `Ident(String)` トークンに

```rust
fn read_ident(&mut self) -> String {
    let start = self.pos;
    while self.pos < self.input.len() {
        let c = self.input[self.pos] as char;
        if c.is_ascii_alphanumeric() || c == '_' {
            self.pos += 1;
        } else {
            break;
        }
    }
    String::from_utf8(self.input[start..self.pos].to_vec()).unwrap()
}
```

### 具体例

```
入力: "int foo_bar = 42;"
       ↓
Int  Ident("foo_bar")  Eq  Num(42)  Semicolon
```

先頭文字の判定で `i` は英字なので `read_ident()` が呼ばれ、
`"int"` が読み取られ、キーワードテーブルにマッチするので `Int` トークンになる。
次の `f` も英字なので `read_ident()` が呼ばれ、
`"foo_bar"` が読み取られ、キーワードに該当しないので `Ident("foo_bar")` になる。

## 変数管理の仕組み

パーサーの `locals: Vec<String>` が変数名の出現順を記録する。
コード生成器はこの順番に基づいてスタックオフセットを割り当てる。

```
int main() {
    int foo;       // locals[0] = "foo"  → -8(%rbp)
    int bar;       // locals[1] = "bar"  → -16(%rbp)
    int baz;       // locals[2] = "baz"  → -24(%rbp)
}
```

変数名が何文字であっても、スタック上のサイズは一律8バイト（int型 = 64bit）。
変数名の文字列自体はコンパイル時の情報であり、生成されるアセンブリには含まれない。

```asm
# "foo" も "a" も、生成されるコードに名前は出ない
mov $42, %rax
mov %rax, -8(%rbp)      # ← これが "foo" に対応するとは、
                         #    アセンブリからは分からない
```

これはコンパイラの基本的な性質：
**変数名はコンパイル時に解決され、実行時にはメモリアドレスに変わる。**
