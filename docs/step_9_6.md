# Step 9.6: static ローカル変数

## 概要

`static` ローカル変数を実装する。`static` 変数は関数呼び出し間で値を保持し、プログラムの実行中ずっと存在する。実体はグローバル変数として配置されるが、名前のスコープは関数内に限定される。

```c
int count() {
    static int c = 0;
    c++;
    return c;
}
int main() {
    count();  // => 1
    count();  // => 2
    return count();  // => 3
}
```

## static ローカル変数の特性

### 通常のローカル変数との違い

| 特性 | 通常のローカル変数 | static ローカル変数 |
|------|-------------------|-------------------|
| 寿命 | 関数実行中のみ | プログラム全体 |
| 配置場所 | スタック上 | `.data` or `.bss` セクション |
| 初期化 | 毎回実行される | プログラム起動時に1回だけ |
| デフォルト値 | 未定義 | ゼロ |

### メモリ配置

通常のローカル変数はスタックフレーム内（`-offset(%rbp)`）に配置されるが、static ローカル変数はグローバル変数と同じ場所（`.data` セクション）に配置される。

## 実装方法

### パーサーでの処理

`static` キーワードを検出したら、以下の手順で処理：

1. 型と変数名をパース
2. ユニークなグローバル名を生成（名前衝突回避）
3. オプションの初期化子をパースしてバイト列に変換
4. グローバル変数リストに登録
5. 現在のスコープに名前マッピングを登録

```rust
fn static_local_var(&mut self) -> Stmt {
    let ty = self.parse_type();
    let name = parse_ident();

    // Generate unique global name
    self.unique_counter += 1;
    let global_name = format!("__static.{}.{}", name, self.unique_counter);

    // Parse optional initializer (constant only)
    let init_bytes = if eq_token { parse_constant() } else { None };

    // Register as global variable
    self.globals.push((ty, global_name, init_bytes));

    // Map local name → global name in current scope
    scope.insert(name, global_name);

    Stmt::Block(vec![]) // no local declaration needed
}
```

### 名前解決

`static int c = 0;` で宣言された `c` は、スコープ内で `__static.c.1` のようなグローバル名に解決される。関数内で `c` を参照すると、スコープ解決により `__static.c.1` に変換され、コード生成ではグローバル変数としてRIP相対アドレッシングでアクセスされる。

```asm
# static int c = 0; c++;
movslq __static.c.1(%rip), %rax   # load from .data section
add $1, %rax
movl %eax, __static.c.1(%rip)     # store back to .data section
```

### 生成されるアセンブリ

```asm
# Static variable in .data section (initialized to 0)
  .data
  .align 4
  .globl __static.c.1
__static.c.1:
  .byte 0,0,0,0
  .text
```

## テストケース

```bash
# Persistent counter across function calls
assert 3 'int count() { static int c = 0; c++; return c; } int main() { count(); count(); return count(); }'

# Accumulator
assert 10 'int add(int x) { static int sum = 0; sum += x; return sum; } int main() { add(1); add(2); add(3); return add(4); }'
```
