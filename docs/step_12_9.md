# ステップ 12.9: 構造体ビットフィールド

## 概要

構造体内のビットフィールド宣言をサポートし、複数の値を1つのストレージユニット（例: 32ビットの `int`）にパッキングできるようにする:

```c
struct {
    int a : 4;   // 4 bits (values 0-15)
    int b : 4;   // 4 bits, packed into same int
} s;
s.a = 5;
s.b = 3;
// Both fields share a single 4-byte int
```

## 実装

### 1. StructMember の拡張（types.rs）

`StructMember` に `bit_width` と `bit_offset` フィールドを追加:

```rust
pub struct StructMember {
    pub name: String,
    pub ty: Type,
    pub offset: usize,
    pub bit_width: usize,   // 0 = normal member
    pub bit_offset: usize,  // bit offset within storage unit
}
```

### 2. パース（parser.rs）

メンバ名をパースした後、`: width` があるかチェックする:

```c
struct { int a : 4; int b : 4; }
```

パーサーはストレージユニット内の現在のビットオフセットを追跡する。ビットフィールドが現在のユニットに収まらない場合、次のアライメントされたストレージユニットに移動する。

**レイアウトアルゴリズム:**
1. 最初のビットフィールドでは、`offset` を型のアライメントに合わせる
2. `bit_offset + bit_width > storage_bits` かチェック
3. 収まる場合、現在のユニットの `bit_offset` 位置にパッキング
4. 収まらない場合、`offset` を次のストレージユニットに進め、`bit_offset = 0` にリセット
5. 通常の（ビットフィールドでない）メンバが続く場合、現在のストレージユニットを終了

### 3. ビットフィールドの読み取り（codegen.rs）

ビットフィールドの値を読み取る手順:
1. ストレージユニットのアドレスを計算（`gen_addr`）
2. ストレージユニット全体をロード（`emit_load_indirect`）
3. `bit_offset` だけ右シフトしてフィールドをビット0に移動
4. `(1 << bit_width) - 1` のマスクで AND を取り、フィールドのビットのみを抽出

```asm
; Read s.b where bit_width=4, bit_offset=4
  lea -4(%rbp), %rax      ; address of storage unit
  movslq (%rax), %rax      ; load full 32-bit int
  shr $4, %rax             ; shift field to bit 0
  and $15, %rax            ; mask to 4 bits (0xF)
```

### 4. ビットフィールドの書き込み（codegen.rs）

ビットフィールドの値を書き込むには、読み取り-変更-書き戻し（read-modify-write）が必要:
1. 新しい値（右辺値）を評価
2. `bit_width` ビットにマスク
3. 正しい `bit_offset` にシフト
4. 現在のストレージユニットの値をロード
5. 古いフィールドビットをクリア（反転マスクで AND）
6. 新しいフィールドビットをセット（シフト済みの値で OR）
7. 結果を書き戻す

```asm
; Write s.b = 3 where bit_width=4, bit_offset=4
  mov $3, %rax             ; new value
  and $15, %rax            ; mask to 4 bits
  shl $4, %rax             ; shift to position
  push %rax                ; save shifted value
  lea -4(%rbp), %rax       ; storage unit address
  mov %rax, %rdi           ; save address for store
  movslq (%rax), %rax      ; load current value
  mov $-241, %rcx          ; clear mask: ~(0xF << 4) = ~0xF0
  and %rcx, %rax           ; clear old bits
  pop %rcx                 ; get shifted new value
  or %rcx, %rax            ; set new bits
  movl %eax, (%rdi)        ; store back
```

## ストレージレイアウトの例

```c
struct {
    int x : 3;   // bits [0:2], offset=0, bit_offset=0
    int y : 5;   // bits [3:7], offset=0, bit_offset=3
} s;
```

`x` と `y` はどちらもオフセット0の1つの4バイト `int` を共有する:
```
bit:  7 6 5 4 3 2 1 0
      [  y      ][x  ]
```

## 制限事項

- 符号なし抽出のみ実装済み（符号付きビットフィールドの符号拡張は未対応）
- アライメント強制用のゼロ幅ビットフィールド（`int : 0`）は未対応
- 無名ビットフィールドは未対応
- ストレージユニットより幅の広いビットフィールドの検証は未実装

## テストケース

```c
struct { int a : 4; int b : 4; } s;
s.a = 5; s.b = 3; return s.a;  // => 5
s.a = 5; s.b = 3; return s.b;  // => 3

struct { int x : 3; int y : 5; } s;
s.x = 7; s.y = 31; return s.x;  // => 7
s.x = 7; s.y = 31; return s.y;  // => 31
```
