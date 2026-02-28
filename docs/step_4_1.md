# Step 4.1: 型の内部表現

## 概要

コンパイラに**型システムの基盤**を導入する。
`types.rs` モジュールを新設し、AST 全体に型情報を付与する。
これにより、後続のステップで `char`, `short`, `long` 等の異なるサイズの型を
追加する際に、型情報に基づいたコード生成が可能になる。

## なぜ型システムが必要か

ここまでのコンパイラでは、全ての変数は暗黙的に 64 ビット整数として扱われていた。
しかし、C 言語では型によってメモリサイズやレジスタ操作が異なる：

| 型 | サイズ | ロード命令 | ストア命令 |
|-------|------|-----------|-----------|
| `char` | 1 byte | `movb` / `movsbl` | `movb` |
| `short` | 2 bytes | `movw` / `movswl` | `movw` |
| `int` | 4 bytes | `movl` | `movl` |
| `long` | 8 bytes | `movq` | `movq` |

型情報をコンパイラ内部で一元管理することで、これらの違いを正しく処理できる。

## 実装

### 1. `types.rs` — 型の定義

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    Int, // 8 bytes (64-bit) for now
}

impl Type {
    pub fn size(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Int => 8,
        }
    }

    pub fn align(&self) -> usize {
        match self {
            Type::Void => 1,
            Type::Int => 8,
        }
    }
}
```

#### `size()` と `align()` の役割

- **`size()`**: 変数が占めるメモリのバイト数。
  スタックのオフセット計算や `.comm` ディレクティブで使用。
- **`align()`**: メモリアライメント要件。
  x86-64 では、データがアライメント境界に揃っていないと
  パフォーマンスが低下する（場合によってはクラッシュする）。
  `.comm symbol, size, alignment` の第 3 引数に使用。

現時点では `Int` は 8 バイト（64 ビット）で扱っている。
これは Step 4.2〜4.3 で `char`（1 バイト）、`short`（2 バイト）、
`int`（4 バイト）、`long`（8 バイト）と細分化される。

### 2. AST の変更

型情報を AST の各所に追加：

#### `Function` 構造体

```rust
pub struct Function {
    pub name: String,
    pub return_ty: Type,              // 戻り値の型
    pub params: Vec<(Type, String)>,  // パラメータの (型, 名前)
    pub body: Vec<Stmt>,
    pub locals: Vec<(Type, String)>,  // ローカル変数の (型, 名前)
}
```

- **`return_ty`**: 関数の戻り値の型。`int` なら `Type::Int`、`void` なら `Type::Void`。
- **`params`**: パラメータに型を紐付け。後のステップで異なる型の引数を正しく扱える。
- **`locals`**: ローカル変数の型を保持。スタックオフセット計算に使用。

#### `Stmt::VarDecl`

```rust
VarDecl {
    name: String,
    ty: Type,         // 変数の型
    init: Option<Expr>,
}
```

#### `Program`

```rust
pub struct Program {
    pub globals: Vec<(Type, String)>,  // グローバル変数の (型, 名前)
    pub functions: Vec<Function>,
}
```

### 3. パーサーの変更

#### `parse_type()` メソッド

型のパースを一箇所に集約するヘルパーメソッドを追加：

```rust
fn parse_type(&mut self) -> Type {
    match self.current().kind {
        TokenKind::Int => {
            self.advance();
            Type::Int
        }
        TokenKind::Void => {
            self.advance();
            Type::Void
        }
        _ => {
            self.reporter.error_at(
                self.current().pos,
                &format!("expected type, but got {:?}", self.current().kind),
            );
        }
    }
}
```

このメソッドは以下の場所から呼ばれる：
- `function_or_prototype()` — 戻り値の型とパラメータの型
- `var_decl()` — ローカル変数の型
- `global_var()` — グローバル変数の型

#### `declare_var()` の変更

```rust
fn declare_var(&mut self, name: &str, ty: Type) -> String {
    let unique = if self.locals.iter().any(|(_, n)| n == name) {
        self.unique_counter += 1;
        format!("{}.{}", name, self.unique_counter)
    } else {
        name.to_string()
    };
    self.locals.push((ty, unique.clone()));
    // ...
}
```

変数宣言時に型情報を `locals` リストに保存する。

### 4. コード生成の変更

#### スタックオフセットの型ベース計算

以前の計算：
```rust
// 固定 8 バイト
for (i, name) in func.locals.iter().enumerate() {
    self.locals.insert(name.clone(), (i + 1) * 8);
}
self.stack_size = func.locals.len() * 8;
```

新しい計算：
```rust
// 型のサイズに基づく
let mut offset = 0;
for (ty, name) in &func.locals {
    offset += ty.size();
    self.locals.insert(name.clone(), offset);
}
self.stack_size = offset;
```

現時点では全て `Type::Int`（8 バイト）なので動作は同じだが、
`char`（1 バイト）を追加した際に自動的に正しいオフセットが計算される。

#### `.comm` ディレクティブの型対応

```rust
for (ty, name) in &program.globals {
    let size = ty.size();
    let align = ty.align();
    self.emit(&format!("  .comm {}, {}, {}", name, size, align));
}
```

グローバル変数のサイズとアライメントを型から取得する。

## 型の流れ

ソースコードから最終的なアセンブリまで、型情報がどう流れるかを示す：

```
ソースコード: int x = 5;
       ↓
トークン: [Int, Ident("x"), Eq, Num(5), Semicolon]
       ↓
パーサー: parse_type() → Type::Int
         declare_var("x", Type::Int)
       ↓
AST: Stmt::VarDecl { name: "x", ty: Type::Int, init: Some(Num(5)) }
     Function.locals: [(Type::Int, "x")]
       ↓
コード生成: offset += Type::Int.size()  → offset = 8
            locals["x"] = 8
            emit: mov $5, %rax
                  mov %rax, -8(%rbp)
```

## 今後の拡張ポイント

この基盤により、以下の拡張が容易になる：

1. **Step 4.2**: `Type::Char` を追加 → `size()` = 1, `movb`/`movsbl` 命令
2. **Step 4.3**: `Type::Short`, `Type::Long` を追加
3. **Step 4.4**: 暗黙的型変換（整数昇格）
4. **Step 4.5**: 明示的キャスト
5. **Step 4.6**: `sizeof` 演算子 → `Type::size()` を返すだけ
6. **Phase 5**: `Type::Ptr(Box<Type>)` でポインタ型を追加

## テスト

ユニットテスト 18 件（2 件追加）+ 統合テスト 126 件 = 144 件

新規ユニットテスト：
- `test_void_function`: void 関数の `return_ty` が `Type::Void` であることを確認
- `test_function_params_typed`: 関数パラメータに型が付与されていることを確認
