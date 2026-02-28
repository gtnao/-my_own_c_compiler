# Step 4.8: _Bool型

## 概要

C99 で導入された `_Bool` 型を実装する。
`_Bool` は 1 バイトの整数型で、値は常に `0` または `1` に正規化される。

```c
_Bool a = 1;     // => 1
_Bool b = 0;     // => 0
_Bool c = 42;    // => 1（非ゼロは1に正規化）
_Bool d = 255;   // => 1
```

## _Bool の特性

### C標準での定義

- サイズ: 1 バイト
- アライメント: 1 バイト
- 値域: `0` または `1` のみ
- 非ゼロ値を代入すると **自動的に `1` に正規化** される
- 本質的には `unsigned` 型（`is_unsigned = true`）

### 他の型との違い

`char` と同じ 1 バイトだが、決定的な違いがある：

| 特性 | char | _Bool |
|---|---|---|
| 値域 | 0〜255 (unsigned) / -128〜127 (signed) | 0 または 1 |
| 代入 | 値をそのまま格納 | 非ゼロ → 1 に正規化 |
| ロード | ゼロ/符号拡張 | ゼロ拡張 |

## 実装

### 1. 型システム

```rust
// types.rs
pub enum TypeKind {
    Void,
    Bool,  // new
    Char,
    Short,
    Int,
    Long,
}

impl Type {
    pub fn bool_type() -> Self {
        Self { kind: TypeKind::Bool, is_unsigned: true }
    }
}
```

`_Bool` は常に `is_unsigned = true`。符号の概念が不要な型だが、
整数昇格時に `unsigned` として扱われるため。

### 2. トークンとレキサー

```rust
// token.rs
Bool,  // "_Bool" keyword

// lexer.rs
"_Bool" => TokenKind::Bool,
```

`_Bool` はアンダースコアで始まるが C の予約語であるため、
識別子のパース結果としてキーワードマッチングで処理する。

### 3. パーサー

`parse_type()` に `TokenKind::Bool` のケースを追加：

```rust
TokenKind::Bool => {
    self.advance();
    Type::bool_type()
}
```

`is_type_keyword()` と `stmt()` の変数宣言分岐にも追加。

### 4. コード生成

#### ロード

`_Bool` のロードはゼロ拡張（`movzbl`）。
値は常に 0 か 1 なので、符号拡張もゼロ拡張も結果は同じだが、
概念的にゼロ拡張が正しい。

```asm
movzbl -1(%rbp), %eax    # local _Bool load
movzbl var(%rip), %eax   # global _Bool load
```

#### ストア（正規化）

`_Bool` の核心はストア時の正規化。非ゼロ値を `1` に変換する：

```asm
# _Bool への代入時、rax の値を 0/1 に正規化
cmp $0, %rax
setne %al        # rax != 0 なら al = 1, そうでなければ al = 0
movb %al, -1(%rbp)
```

**`setne` 命令の動作：**
- `cmp $0, %rax` でフラグを設定
- `setne %al` は ZF（Zero Flag）が **クリア** されていれば `%al = 1`、
  **セット** されていれば `%al = 0`
- つまり `%rax != 0` なら `%al = 1`、`%rax == 0` なら `%al = 0`

これにより、`_Bool b = 42;` のような代入が `b = 1` として格納される。

#### キャスト

`(_Bool)expr` は式の値を 0/1 に正規化：

```asm
cmp $0, %rax
setne %al
movzbl %al, %eax    # 0/1 を 32-bit に拡張（上位自動ゼロクリア）
```

### 5. 整数昇格

`common_type()` において `_Bool` は `char`/`short` と同じく
`int` に昇格する：

```rust
TypeKind::Bool | TypeKind::Char | TypeKind::Short => Type::int_type(),
```

## _Bool はなぜ `_Bool` なのか

C99 では `bool` という名前ではなく `_Bool` が正式な型名。
理由は既存のコードとの互換性：

- 多くのCコードが独自に `bool` を `typedef` や `#define` で定義していた
- 新しいキーワードとして `bool` を追加すると既存コードが壊れる
- C標準は `_` + 大文字 で始まる識別子を予約しており、`_Bool` はこの規則に従う
- `<stdbool.h>` で `#define bool _Bool` として互換レイヤーを提供

将来 `<stdbool.h>` を実装する際に `bool` マクロを追加予定。

## テスト

ユニットテスト 22 件 + 統合テスト 170 件（7 件追加）= 192 件
