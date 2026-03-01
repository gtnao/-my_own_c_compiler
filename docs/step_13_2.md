# Step 13.2: ピープホール最適化 — 冗長なpush/popの除去

## 概要

生成されたアセンブリ中の冗長な`push`/`pop`命令ペアを除去する、コード生成後のピープホール最適化パスを追加します。

## スタックマシン方式のコード生成

コンパイラはスタックマシン方式を採用しており、二項演算は以下のように処理されます:

```asm
; Compute a + b
  <compute rhs>       ; result in %rax
  push %rax            ; save rhs on stack
  <compute lhs>       ; result in %rax
  pop %rdi             ; restore rhs to %rdi
  add %rdi, %rax       ; %rax = lhs + rhs
```

この方式では多くの`push`/`pop`ペアが生成されます。これらが隣接している場合、最適化が可能です。

## 最適化パターン

### パターン1: `push %rax` + `pop %rax` → 両方削除

同じレジスタがpushされてすぐにpopされる場合、両方の命令はデッドコードです:

```asm
; Before:
  push %rax
  pop %rax
; After:
  (removed)
```

### パターン2: `push %rax` + `pop %reg` → `mov %rax, %reg`

pushの直後に別のレジスタへのpopが続く場合、レジスタ間のmovと等価であり、メモリアクセスが不要なため高速です:

```asm
; Before:
  push %rax
  pop %rdi
; After:
  mov %rax, %rdi
```

## 実装

最適化は`peephole_optimize()`において、生成されたアセンブリテキストに対する後処理パスとして適用されます:

```rust
fn peephole_optimize(&mut self) {
    let lines: Vec<&str> = self.output.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if i + 1 < lines.len() {
            let cur = lines[i].trim();
            let next = lines[i + 1].trim();

            if cur == "push %rax" && next == "pop %rax" {
                i += 2; continue;
            }
            if cur == "push %rax" {
                if let Some(reg) = next.strip_prefix("pop ") {
                    result.push(format!("  mov %rax, {}", reg));
                    i += 2; continue;
                }
            }
        }
        result.push(lines[i].to_string());
        i += 1;
    }
}
```

## 安全性

この最適化は**隣接する**`push`/`pop`ペアにのみ適用されます。間に他の命令が挟まる非隣接ペアはそのまま残されます。pushとpopの間のスタック状態が中間計算に必要な場合があるためです。

この保守的なアプローチが安全である理由:
1. 証明可能に等価なペアのみを変換する
2. スタック深度が重要な箇所で変更を加えない
3. すべてのコード生成が完了した後に最適化が実行される

## 例

```c
return 2 + 3 * 4;
```

定数畳み込みにより`mov $14, %rax`となり、最適化すべきpush/popは存在しません。定数でない式の場合、隣接するpush/popペアは`mov`命令に変換されます。

## 制限事項

- 隣接する`push %rax; pop %reg`ペアのみを最適化する
- 間に命令が挟まる`push/pop`は最適化しない
- `push %rdi; pop %rdi`などの他のレジスタペアは最適化しない
- レジスタ割り当ては行わない — スタックマシンモデルのまま
