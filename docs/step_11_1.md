# Step 11.1: printf呼び出し対応（libc リンク）

## 概要

printfなどのlibc関数を正しく呼び出せるようにする。x86-64 System V ABIでは可変長引数関数を呼ぶ前に `%al` にベクトルレジスタの使用数を設定する必要がある。

```c
int printf();
int main() {
    printf("hello");
    printf("%d + %d = %d", 3, 4, 3+4);
    return 0;
}
```

## x86-64 ABI と可変長引数

### System V AMD64 ABI の規約

可変長引数関数（printf, scanfなど）を呼ぶ際の特別な規約：

- **`%al`** にベクトルレジスタ（XMM0-XMM7）の使用数を設定
- これは浮動小数点引数がXMMレジスタで渡される場合にカウントされる
- 整数引数のみの場合は `%al = 0`

### なぜ必要か

可変長引数関数の実装側（libc内部）は、引数の型と数を実行時に判断する必要がある。`%al` の値を見て、XMMレジスタからの引数取得をスキップできる。`%al` が未初期化だとクラッシュする可能性がある。

### 実装

全ての関数呼び出しの前に `mov $0, %al` を追加：

```asm
  mov $0, %al       # no vector register args
  call printf
```

通常の（非可変長）関数では `%al` は無視されるため、常に設定しても問題ない。浮動小数点引数は現在未サポートなので、常に0で正しい。

## テスト方法

printfの出力を検証するため、テストスクリプトに `assert_output` ヘルパーを追加：

```bash
assert_output() {
  expected_output="$1"
  input="$2"
  echo "$input" > "$TMPDIR/tmp.c"
  $COMPILER "$TMPDIR/tmp.c" > "$TMPDIR/tmp.s"
  gcc -o "$TMPDIR/tmp" "$TMPDIR/tmp.s"
  actual_output=$("$TMPDIR/tmp")
  # compare actual vs expected output
}
```

GCCでリンクする際にlibcが自動的にリンクされるため、printfのシンボルが解決される。

## テストケース

```bash
# basic printf
assert_output 'hello' 'int printf(); int main() { printf("hello"); return 0; }'

# printf with format specifier
assert_output '42' 'int printf(); int main() { printf("%d", 42); return 0; }'

# printf with multiple args
assert_output '3 + 4 = 7' 'int printf(); int main() {
  printf("%d + %d = %d", 3, 4, 3+4); return 0; }'
```
