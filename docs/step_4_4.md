# Step 4.4: 暗黙的型変換

## 概要

C言語の **暗黙的型変換**（implicit type conversion）の仕組みを実装する。
異なる型のオペランドが式で使われた際に、共通の型に自動変換される。

## C言語の型変換ルール

### 1. 整数昇格（Integer Promotion）

`char` や `short` は、式の中で使われると自動的に `int` に昇格される：

```c
char a = 1;
char b = 2;
// a + b の計算は int として行われる
int result = a + b;  // 3
```

### 2. 通常の算術変換（Usual Arithmetic Conversion）

二項演算の両オペランドを共通の型に変換するルール：

1. 両方を整数昇格する（char/short → int）
2. 同じ型なら変換不要
3. 異なる型なら、より大きい型に合わせる

```
char + char   → int + int   → int
short + int   → int + int   → int
int + long    → long + long → long
char + long   → int + long  → long + long → long
```

### 3. 代入時の変換（Assignment Conversion）

代入先の型に暗黙的に変換（truncation が起きうる）：

```c
int a = 256;
char b = a;   // b = 0 (256 の下位 8 ビットは 0)
```

## 実装

### `Type::common_type()`

```rust
pub fn common_type(a: &Type, b: &Type) -> Type {
    // Integer promotion: char/short → int
    let a = match a {
        Type::Char | Type::Short => Type::Int,
        other => other.clone(),
    };
    let b = match b {
        Type::Char | Type::Short => Type::Int,
        other => other.clone(),
    };
    // Usual arithmetic conversion: wider type wins
    if a.size() >= b.size() { a } else { b }
}
```

### なぜ現在のアーキテクチャで「自然に」動作するか

本コンパイラのコード生成は以下のアーキテクチャを採用している：

1. **全ての演算は 64 ビットレジスタ（`%rax`）で実行**
2. **ロード時に符号拡張**（`movsbq`, `movswq`, `movslq`）
3. **ストア時にサイズに応じて truncate**（`movb`, `movw`, `movl`）

この設計により、暗黙的型変換の主要なケースが自動的に処理される：

#### 小さい型 → 大きい型（安全な変換）

```c
char a = 65;
int b = a;    // movsbq で sign-extend → movl で store
```

1. `movsbq -1(%rbp), %rax` — char を 64 ビットに符号拡張
2. `movl %eax, -8(%rbp)` — 下位 32 ビットを int として格納

値 65 は全ての型で正確に表現できるので問題ない。

#### 大きい型 → 小さい型（truncation）

```c
int a = 256;
char b = a;   // movslq で load → movb で store (truncate)
```

1. `movslq -4(%rbp), %rax` — int 256 を 64 ビットに拡張
2. `movb %al, -1(%rbp)` — 下位 1 バイト（0x00）だけ格納

256 = 0x100 なので、下位バイトは 0。

#### 混合型の算術

```c
char a = 1;
long b = 4;
long c = a + b;
```

1. `movsbq` で a をロード → `%rax = 1`（64 ビット）
2. push → pop で `%rdi = 1`
3. `mov` で b をロード → `%rax = 4`（64 ビット）
4. `add %rdi, %rax` → `%rax = 5`（64 ビット加算）

両オペランドが 64 ビットに拡張されてから加算されるため、
型の違いは加算の正確さに影響しない。

## 型変換の一覧

| 変換元 | 変換先 | 動作 |
|--------|--------|------|
| char → short | 安全 | 符号拡張で値保持 |
| char → int | 安全 | 符号拡張で値保持 |
| char → long | 安全 | 符号拡張で値保持 |
| short → int | 安全 | 符号拡張で値保持 |
| short → long | 安全 | 符号拡張で値保持 |
| int → long | 安全 | 符号拡張で値保持 |
| long → int | truncation | 上位 32 ビットを失う |
| int → short | truncation | 上位 16 ビットを失う |
| int → char | truncation | 上位 24 ビットを失う |

## テスト

ユニットテスト 21 件（2 件追加）+ 統合テスト 144 件（4 件追加）= 165 件
