# ステップ 15.1: 配列とポインタを含む複数宣言子

## 概要

複数変数宣言（カンマ区切り）で、ポインタのスターに加えて配列の次元も正しく処理できるよう修正する。従来は `int a, b[3], *c;` の `b[3]` 部分がパースに失敗していた。

## バグの内容

複数宣言子のパースコードはポインタ修飾子（`*`）は各宣言子について処理していたが、配列の次元（`[N]`）はパースしていなかった。そのため `int a = 1, *b, c[3];` では `c[3]` が配列として認識されず、`c` は単なる `int` として宣言されていた。

## 修正内容

両方の複数宣言子パス（初期化子ありとなし）で、変数名の後に配列の次元のパースを追加した:

```rust
// After parsing the declarator name, check for array dimensions
while self.current().kind == TokenKind::LBracket {
    self.advance();
    if self.current().kind == TokenKind::RBracket {
        self.advance();
        decl_ty = Type { kind: TypeKind::Array(Box::new(decl_ty), 0), is_unsigned: false };
    } else {
        let size = self.eval_const_expr();
        self.expect(TokenKind::RBracket);
        decl_ty = Type { kind: TypeKind::Array(Box::new(decl_ty), size as usize), is_unsigned: false };
    }
}
```

## その他の更新

PLAN.mdにPhase 15-20を追加し、PostgreSQLコンパイルに必要な残りの全機能を網羅した:
- Phase 15: 高度な宣言と型システム
- Phase 16: プリプロセッサ拡張
- Phase 17: 標準ライブラリヘッダスタブ
- Phase 18: GCC拡張とビルトイン
- Phase 19: 高度なコード生成
- Phase 20: PostgreSQL統合テスト

## テストケース

```c
int a = 1, b = 2, c = 3; return a+b+c;           // → 6
int a, b, c; a=1; b=2; c=3; return a+b+c;         // → 6
int a = 1, *b, c[3]; b=&a; c[1]=20; return *b+c[1]; // → 21
```
