# Step 1.2: 加減算

## 概要

`+` と `-` 演算子を追加し、`5+20-4` のような加減算の式をコンパイルできるようにする。

## 入出力

**入力**:
```
5+20-4
```

**出力**:
```asm
  .globl main
main:
  mov $5, %rax
  add $20, %rax
  sub $4, %rax
  ret
```

## 実装の変更点

### main.rs

Step 1.1では入力全体を1つの数値としてパースしていたが、このステップでは簡易的なトークナイザーとして文字列を1文字ずつ走査する方式に変更した。

#### read_number 関数

```rust
fn read_number(bytes: &[u8], pos: &mut usize) -> i64 {
    let start = *pos;
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_digit() {
        *pos += 1;
    }
    // ...
    s.parse().unwrap()
}
```

バイト列から連続する数字を読み取り、`i64` に変換する。`pos` は呼び出し元と共有する読み取り位置。

#### メインループ

```rust
let val = read_number(bytes, &mut pos);    // 最初の数値
println!("  mov ${}, %rax", val);

while pos < bytes.len() {
    let ch = bytes[pos] as char;
    if ch == '+' {
        pos += 1;
        let val = read_number(bytes, &mut pos);
        println!("  add ${}, %rax", val);   // rax += val
    } else if ch == '-' {
        pos += 1;
        let val = read_number(bytes, &mut pos);
        println!("  sub ${}, %rax", val);   // rax -= val
    } else {
        // error
    }
}
```

処理の流れ:

1. 最初の数値を読み、`mov` で `%rax` にセット
2. `+` が来たら次の数値を読み、`add` で `%rax` に加算
3. `-` が来たら次の数値を読み、`sub` で `%rax` から減算
4. 入力の末尾まで繰り返す

## 生成されるアセンブリの解説

`5+20-4` の場合:

```asm
  mov $5, %rax       # rax = 5
  add $20, %rax      # rax = 5 + 20 = 25
  sub $4, %rax       # rax = 25 - 4 = 21
  ret                # return 21
```

### add 命令

```
add SRC, DST    →    DST = DST + SRC
```

`add $20, %rax` は `%rax = %rax + 20` を意味する。AT&T構文では **ソースが先、デスティネーションが後** であることに注意。

### sub 命令

```
sub SRC, DST    →    DST = DST - SRC
```

`sub $4, %rax` は `%rax = %rax - 4` を意味する。

### コード生成パターン

このステップのコード生成は非常にシンプルで、`%rax` をアキュムレータ (accumulator) として使う:

```
結果 = 最初の数値
結果 += 次の数値 (+ の場合)
結果 -= 次の数値 (- の場合)
...
```

即値を直接 `add`/`sub` しているため、各演算が1命令で完了する。乗除算が入ると（Step 1.4）、この単純な方式では対応できなくなり、スタックマシン方式に移行する。

## テスト

| 入力 | 期待値 | 目的 |
|------|--------|------|
| `5+20-4` | 21 | 加算と減算の組み合わせ |
| `0+0` | 0 | ゼロ同士の加算 |
| `10` | 10 | 演算子なし (Step 1.1 の互換性) |
| `1+2` | 3 | 単純な加算 |
| `10-5` | 5 | 単純な減算 |
| `1+2+3+4+5` | 15 | 連続した加算 |

## このステップで学べること

1. **簡易トークナイザー**: バイト列を1文字ずつ走査して数値と演算子を識別する
2. **コード生成の基本パターン**: アキュムレータ (`%rax`) に対する逐次的な演算
3. **add/sub 命令**: x86-64の基本的な算術命令とAT&T構文の語順

## 制約・未対応

- スペースは未対応 (`5 + 20` はエラー) → Step 1.3 で対応
- `*`, `/` は未対応 → Step 1.4 で対応
- 演算子の優先順位なし (左から右に逐次評価) → Step 1.4 で対応
- 括弧は未対応 → Step 1.4 で対応
- 単項マイナス (`-10`) は未対応 → Step 1.4 で対応

## 次のステップ

→ **Step 1.3: トークナイザーの分離とスペース対応** — `token.rs`, `lexer.rs` を新規作成し、スペースを許容する。
