# Step 13.7: 総合テスト

## 概要

コンパイラの機能を総合的に検証するため、実用的なアルゴリズムを実装したテストケースを追加する。単純な式や文のテストではなく、複数の言語機能を組み合わせた実践的なプログラムで正しく動作することを確認する。

## テストケース

### FizzBuzz カウント

```c
int main() {
    int count = 0;
    int i;
    for (i = 1; i <= 100; i++) {
        if (i % 15 == 0) count += 4;      // "FizzBuzz" = 8文字だが簡略化
        else if (i % 3 == 0) count += 1;   // "Fizz" のカウント
        else if (i % 5 == 0) count += 1;   // "Buzz" のカウント
    }
    return count;  // => 65
}
```

使用機能: for文、if/else if、剰余演算子 `%`、複合代入 `+=`、比較演算子

**カウントの内訳:**
- `i % 15 == 0`: 15の倍数は6個（15,30,45,60,75,90）→ 6×4 = 24
- `i % 3 == 0` (15の倍数を除く): 33-6 = 27個 → 27×1 = 27
- `i % 5 == 0` (15の倍数を除く): 20-6 = 14個 → 14×1 = 14
- 合計: 24 + 27 + 14 = 65

### フィボナッチ数列（再帰）

```c
int fib(int n) {
    if (n <= 1) return n;
    return fib(n-1) + fib(n-2);
}
int main() { return fib(10); }  // => 55
int main() { return fib(11); }  // => 89
```

使用機能: 再帰呼び出し、関数定義と引数、if文、算術演算

フィボナッチ数列: 0, 1, 1, 2, 3, 5, 8, 13, 21, 34, **55**, **89**, ...

再帰呼び出しは関数フレームの正しいpush/pop、引数の正しいレジスタ渡し（System V ABI）、戻り値の正しい%rax返却が全て正しく動作することを検証する。

### 反復的階乗

```c
int main() {
    int n = 5;
    int result = 1;
    int i;
    for (i = 2; i <= n; i++)
        result *= i;
    return result;  // => 120
}
```

使用機能: for文、複合代入 `*=`、ローカル変数

5! = 1 × 2 × 3 × 4 × 5 = 120

### バブルソート

```c
int main() {
    int a[] = {5, 3, 1, 4, 2};
    int i; int j;
    for (i = 0; i < 5; i++)
        for (j = 0; j < 4; j++)
            if (a[j] > a[j+1]) {
                int t = a[j];
                a[j] = a[j+1];
                a[j+1] = t;
            }
    return a[0];  // => 1（最小値）
    return a[4];  // => 5（最大値）
}
```

使用機能: 配列初期化子、ネストしたfor文、配列添字アクセス、if文、一時変数によるスワップ

バブルソートは隣接要素の比較・交換を繰り返す。配列の読み書き、ポインタ算術（`a[i]` は `*(a+i)` にdesugarされる）、ネストしたループが正しく動作することを検証する。

### ユークリッドの互除法（GCD）

```c
int gcd(int a, int b) {
    while (b != 0) {
        int t = b;
        b = a % b;
        a = t;
    }
    return a;
}
int main() { return gcd(48, 18); }  // => 6
int main() { return gcd(7, 13); }   // => 1
```

使用機能: while文、剰余演算子、関数定義と引数

GCD(48, 18):
- 48 % 18 = 12 → (18, 12)
- 18 % 12 = 6  → (12, 6)
- 12 % 6 = 0   → (6, 0)
- → 6

GCD(7, 13):
- 7 % 13 = 7   → (13, 7)
- 13 % 7 = 6   → (7, 6)
- 7 % 6 = 1    → (6, 1)
- 6 % 1 = 0    → (1, 0)
- → 1（互いに素）

### 累乗計算

```c
int power(int base, int exp) {
    int result = 1;
    while (exp > 0) {
        result *= base;
        exp--;
    }
    return result;
}
int main() { return power(2, 3); }  // => 8
int main() { return power(3, 3); }  // => 27
```

使用機能: while文、デクリメント `--`、複合代入 `*=`、関数引数

### 配列の合計

```c
int main() {
    int vals[5] = {1, 2, 3, 4, 5};
    int sum = 0;
    int i;
    for (i = 0; i < 5; i++)
        sum += vals[i];
    return sum;  // => 15
}
```

使用機能: 配列宣言と初期化子、for文、配列添字アクセス、複合代入

## 検証される言語機能の一覧

| 機能 | テストケース |
|------|------------|
| for文 | FizzBuzz, 階乗, バブルソート, 配列合計 |
| while文 | GCD, 累乗 |
| if/else if | FizzBuzz, バブルソート |
| 再帰呼び出し | フィボナッチ |
| 配列初期化子 | バブルソート, 配列合計 |
| 配列添字アクセス | バブルソート, 配列合計 |
| 剰余演算子 `%` | FizzBuzz, GCD |
| 複合代入 `+=`, `*=` | FizzBuzz, 階乗, 累乗, 配列合計 |
| デクリメント `--` | 累乗 |
| 関数定義と引数 | フィボナッチ, GCD, 累乗 |
| ブロックスコープ | バブルソート（ループ内の一時変数） |

### エラトステネスの篩（素数計数）

```c
int main() {
    int sieve[31];
    int i, j;
    for (i = 0; i <= 30; i++) sieve[i] = 1;
    sieve[0] = 0; sieve[1] = 0;
    for (i = 2; i * i <= 30; i++)
        if (sieve[i])
            for (j = i * i; j <= 30; j += i)
                sieve[j] = 0;
    int count = 0;
    for (i = 2; i <= 30; i++)
        if (sieve[i]) count++;
    return count;  // => 10
}
```

使用機能: 配列、複数変数宣言、ネストしたfor文、複合代入 `+=`

### 行列積（2x2）

```c
int main() {
    int a[4] = {1, 2, 3, 4};
    int b[4] = {5, 6, 7, 8};
    int c[4];
    c[0] = a[0]*b[0] + a[1]*b[2];  // 1*5+2*7=19
    return c[0];
}
```

使用機能: 配列初期化子、算術式の組み合わせ

### アッカーマン関数

```c
int ack(int m, int n) {
    if (m == 0) return n + 1;
    if (n == 0) return ack(m - 1, 1);
    return ack(m - 1, ack(m, n - 1));
}
int main() { return ack(2, 3); }  // => 9
```

使用機能: 深い再帰、再帰呼び出しのネスト

### 関数ポインタによるディスパッチ

```c
int dbl(int x) { return x * 2; }
int triple(int x) { return x * 3; }
int main() {
    int (*f)(int) = dbl;
    int a = f(3);     // 6
    f = triple;
    int b = f(3);     // 9
    return a + b;     // => 15
}
```

使用機能: 関数ポインタ宣言、関数-ポインタ退化、間接呼び出し、ポインタの再代入

## テスト結果

全371テストがパス。
