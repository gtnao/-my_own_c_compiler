# Step 14.1: long longと複合型指定子

## 概要

Cコードで一般的に使用される複合型指定子のサポートを追加します:
- `long long` / `long long int` — 8バイト符号付き整数（x86-64では`long`と同じ）
- `unsigned long long` — 8バイト符号なし整数
- `long int` — `long`の明示的な形式
- `short int` — `short`の明示的な形式
- `unsigned short int` — `unsigned short`の明示的な形式

これらの複合指定子は実際のCコード、特にシステムヘッダやPostgreSQLなどのライブラリで非常に頻繁に使用されます。

## なぜ必要か

Cでは型指定子をさまざまな方法で組み合わせることができます:

```c
long long x;          // 8-byte signed integer
long long int x;      // equivalent to long long
unsigned long long x; // 8-byte unsigned integer
long int x;           // equivalent to long
short int x;          // equivalent to short
unsigned short int x; // equivalent to unsigned short
```

C標準（C11 §6.7.2）はこれらを有効な型指定子の組み合わせとして定義しています。多くのコードベース、特にPostgreSQLは64ビット整数型として`long long`を広く使用しています。

## 実装

### パーサーの変更（`parse_type()`）

`Long`トークンのハンドラを拡張し、後続の`Long`または`Int`トークンをチェックするようにしました:

```rust
TokenKind::Long => {
    self.advance();
    // Skip optional "long" (long long) or "int" (long int)
    if self.current().kind == TokenKind::Long {
        self.advance();
        // Skip optional "int" after "long long"
        if self.current().kind == TokenKind::Int {
            self.advance();
        }
    } else if self.current().kind == TokenKind::Int {
        self.advance();
    }
    if is_unsigned { Type::ulong() } else { Type::long_type() }
}
```

同様に、`Short`トークンのハンドラもオプションの`Int`をスキップするようにしました:

```rust
TokenKind::Short => {
    self.advance();
    // Skip optional "int" after "short"
    if self.current().kind == TokenKind::Int {
        self.advance();
    }
    if is_unsigned { Type::ushort() } else { Type::short_type() }
}
```

### x86-64での型マッピング

x86-64 Linux（LP64モデル）では:

| C型 | サイズ | アラインメント | 内部型 |
|--------|------|-----------|---------------|
| `short` / `short int` | 2 | 2 | `Short` |
| `int` | 4 | 4 | `Int` |
| `long` / `long int` | 8 | 8 | `Long` |
| `long long` / `long long int` | 8 | 8 | `Long` |

x86-64 LP64では`long`と`long long`はどちらも8バイトであることに注意してください。C標準上は異なる型ですが、本コンパイラでは同じ内部型`Long`にマッピングしています。これはGCCやClangがこのプラットフォームで取るアプローチと同じです。

### `unsigned`プレフィックス

`unsigned`キーワードはまずフラグ（`is_unsigned`）として解析され、その後ベースの型指定子が`Type::ulong()`か`Type::long_type()`などを生成するかを決定します。これは複合指定子とシームレスに連携します:

```
unsigned long long int x;
^^^^^^^^ ^^^^ ^^^^ ^^^
   |       |    |    |
   |       |    |    +-- trailing "int"として消費
   |       |    +------- 2番目の"long"として消費
   |       +------------ 1番目の"long"として消費
   +-------------------- is_unsigned = trueを設定
```

## テストケース

```c
// long long
long long a = 42;              // basic usage
long long int b = 1;           // with trailing int
sizeof(long long) == 8         // size check
sizeof(long long int) == 8     // size check with int
unsigned long long c = 42;     // unsigned variant
sizeof(unsigned long long) == 8

// long int
long int d = 42;               // explicit long int
sizeof(long int) == 8

// short int
short int e = 42;              // explicit short int
sizeof(short int) == 2
unsigned short int f = 100;    // unsigned short int
```
