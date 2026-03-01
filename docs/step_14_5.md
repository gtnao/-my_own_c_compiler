# Step 14.5: 16進数、8進数、2進数の整数リテラル

## 概要

16進数（`0xFF`）、8進数（`077`）、2進数（`0b101`）の整数リテラル構文のサポートを追加します。以前は10進数の整数リテラルのみサポートしていました。

## なぜ必要か

PostgreSQLやシステムヘッダでは、ビットマスク、フラグ、メモリ定数に16進数リテラルが広く使用されています:

```c
#define PG_DETOAST_DATUM(datum) \
    ((Datum) (VARATT_IS_1B(datum) ? (datum) : pg_detoast_datum(datum)))
#define TYPEALIGN(ALIGNVAL, LEN) \
    (((uintptr_t)(LEN) + ((ALIGNVAL)-1)) & ~((uintptr_t)((ALIGNVAL)-1)))
int flags = 0xFF;
int permissions = 0755;
```

## 実装

### レキサーの変更

`read_number_or_float()`メソッドをリファクタリングし、プレフィックスベースの数値形式を検出するようにしました:

1. **16進数（`0x`/`0X`）**: `0x`プレフィックスを消費した後、16進数字（`0-9`, `a-f`, `A-F`）を読み取り、16進算術で値を蓄積します。

2. **2進数（`0b`/`0B`）**: `0b`プレフィックスを消費した後、2進数字（`0`または`1`）を読み取り、2進算術で値を蓄積します。

3. **8進数（`0`の後に`0-7`）**: 先頭の`0`の後に8進数字が続く場合、8進数字を読み取り、8進算術で値を蓄積します。単独の`0`は単にゼロ（10進数）です。

4. **10進数/浮動小数点**: 抽出された`read_decimal_float()`ヘルパーで処理 — 以前と同じロジックです。

### コード構造

メソッドは明確化のために3つの部分に分割されました:

- `read_number_or_float()` — エントリポイント。プレフィックスを検出してディスパッチ。
- `read_decimal_float()` — 10進整数と浮動小数点数（`.`、指数、`f`サフィックス）を処理。
- `skip_int_suffix()` — 末尾の`L`/`l`/`U`/`u`サフィックスを消費（すべての整数形式で共有）。

### 数値解析のフロー

```
read_number_or_float()
  ├── starts with '.' → read_decimal_float(starts_with_dot=true)
  ├── starts with '0x'/'0X' → parse hex digits → skip_int_suffix()
  ├── starts with '0b'/'0B' → parse binary digits → skip_int_suffix()
  ├── starts with '0' + octal digit → parse octal digits → skip_int_suffix()
  └── otherwise → read_decimal_float(starts_with_dot=false)
```

### 値の計算

16進数の場合、蓄積ループ:
```rust
let mut val: i64 = 0;
while self.pos < self.input.len() && (self.input[self.pos] as char).is_ascii_hexdigit() {
    val = val * 16 + (self.input[self.pos] as char).to_digit(16).unwrap() as i64;
    self.pos += 1;
}
```

8進数の場合:
```rust
let mut val: i64 = 0;
while self.pos < self.input.len() && self.input[self.pos] >= b'0' && self.input[self.pos] <= b'7' {
    val = val * 8 + (self.input[self.pos] - b'0') as i64;
    self.pos += 1;
}
```

2進数の場合:
```rust
let mut val: i64 = 0;
while self.pos < self.input.len() && (self.input[self.pos] == b'0' || self.input[self.pos] == b'1') {
    val = val * 2 + (self.input[self.pos] - b'0') as i64;
    self.pos += 1;
}
```

3つの形式すべてが`TokenKind::Num(i64)`を生成します — ソース表記に関係なくトークン表現は同じです。整数サフィックス（`L`, `U`, `ULL`など）はすべての整数形式の後に消費されて無視されます。

## C標準に関する注記

- 16進数リテラル: C89/C90以降。プレフィックスは`0x`または`0X`。
- 8進数リテラル: C89/C90以降。先頭の`0`プレフィックス。
- 2進数リテラル: C標準には含まれない（C23で提案中）が、GCC/Clang拡張として広くサポート。
- リテラル`0`は厳密には8進数だが、いずれにしてもゼロに評価される。

## テストケース

```c
int main() { return 0xFF; }       // → 255
int main() { return 0x0F; }       // → 15
int main() { return 0xAB; }       // → 171
int main() { return 0x0; }        // → 0
int main() { return 07; }         // → 7
int main() { return 077; }        // → 63
int main() { return 010; }        // → 8
int main() { return 00; }         // → 0
int main() { return 0b101; }      // → 5
int main() { return 0b1010; }     // → 10
int main() { int x = 0xFF; return x; }  // → 255
int main() { return 0x10; }       // → 16
int main() { return 0x0A; }       // → 10
int main() { return 0xf; }        // → 15 (lowercase)
```
