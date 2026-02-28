# Step 1.5: 剰余演算子

## 概要

`%` (剰余/モジュロ) 演算子を追加する。`*`, `/` と同じ優先順位で扱う。

## 入出力

**入力**: `10 % 3`

**出力** (抜粋):
```asm
  cqto
  idiv %rdi
  mov %rdx, %rax
```

## 実装の変更点

| ファイル | 変更内容 |
|---------|---------|
| `token.rs` | `Percent` トークン追加 |
| `lexer.rs` | `'%'` → `TokenKind::Percent` |
| `ast.rs` | `BinOp::Mod` 追加 |
| `parser.rs` | `mul` 規則に `%` 追加 |
| `codegen.rs` | `idiv` 後に `mov %rdx, %rax` |

### コード生成のポイント

`idiv` 命令は商を `%rax` に、**余りを `%rdx`** に格納する。除算 (`/`) では `%rax` をそのまま使うが、剰余 (`%`) では `%rdx` を `%rax` にコピーする:

```rust
BinOp::Mod => {
    self.emit("  cqto");              // sign-extend rax → rdx:rax
    self.emit("  idiv %rdi");         // rdx:rax / rdi → rax=商, rdx=余り
    self.emit("  mov %rdx, %rax");    // 余りを戻り値レジスタに移動
}
```

## テスト

| 入力 | 期待値 |
|------|--------|
| `10 % 3` | 1 |
| `6 % 3` | 0 |
| `11 % 3` | 2 |
| `10 % 5` | 0 |
| `7 % 2` | 1 |

## 次のステップ

→ **Step 1.6: 比較演算子**
