# Step 21.2: コード生成の修正とアセンブル検証

## 概要

このステップでは、コンパイラが生成するアセンブリコードが実際にアセンブル可能であることを検証し、発見されたコード生成の問題を修正します。

## 問題

### switch文のcase値における大きな即値

x86-64の`cmp`命令は、即値オペランドとして32ビット符号拡張値のみをサポートします。しかし、PostgreSQLのコードでは`unsigned int`のビットマスクなど、32ビット符号付き整数の範囲（-2147483648 〜 2147483647）を超える値がswitch文のcase値として使われています。

例：
```c
// PostgreSQL trigger.c のコード
switch (tgattr & 0xC0000000) {
    case 0x80000000:  // 2147483648 — 32ビット符号付き最大値を超える
        ...
}
```

生成されるアセンブリ：
```asm
# 修正前（アセンブルエラー）
cmp $2147483648, %rax    # Error: operand type mismatch for 'cmp'

# 修正後
movabs $2147483648, %rdi  # 64ビット即値をレジスタにロード
cmp %rdi, %rax            # レジスタ同士で比較
```

## 実装

`src/codegen.rs` のswitch文コード生成部分で、case値が32ビット符号付き整数の範囲外の場合、`movabs`命令でレジスタにロードしてから`cmp`で比較するように修正しました。

```rust
for (val, _) in cases {
    let label = self.new_label();
    if *val > i32::MAX as i64 || *val < i32::MIN as i64 {
        self.emit(&format!("  movabs ${}, %rdi", val));
        self.emit("  cmp %rdi, %rax");
    } else {
        self.emit(&format!("  cmp ${}, %rax", val));
    }
    self.emit(&format!("  je {}", label));
    case_labels.push(label);
}
```

### なぜ `movabs` が必要か

x86-64では、ほとんどの命令で即値は32ビットに制限されています。`mov`命令だけが例外的に64ビット即値をサポートしますが（`movabs`エンコーディング）、`cmp`命令にはそのようなエンコーディングがありません。そのため、大きな即値はまずレジスタにロードしてから比較する必要があります。

## 検証

- **統合テスト**: 578 パス、0 失敗
- **PostgreSQLバックエンドファイル**: 331/331 パース＋アセンブル成功（100%）

## 変更されたファイル

- `src/codegen.rs` — switch文のcase値比較で大きな即値をレジスタ経由に変更
