# Step 3.2: 関数引数（最大6個、System V AMD64 ABI）

## 概要

関数に引数を渡せるようにする。System V AMD64 ABI に従い、最初の6個の整数引数はレジスタで渡す。

```c
int add(int a, int b) { return a + b; }
int main() { return add(3, 4); }  // => 7
```

再帰関数も動作する：
```c
int fact(int n) { if (n <= 1) return 1; return n * fact(n - 1); }
int main() { return fact(5); }  // => 120
```

## System V AMD64 ABI の引数渡し規約

### レジスタ割り当て

x86-64 Linux では System V AMD64 ABI に従い、整数引数は以下のレジスタで渡される：

| 引数番号 | レジスタ |
|---------|---------|
| 第1引数 | `%rdi` |
| 第2引数 | `%rsi` |
| 第3引数 | `%rdx` |
| 第4引数 | `%rcx` |
| 第5引数 | `%r8` |
| 第6引数 | `%r9` |

戻り値は `%rax` に格納される。

### なぜレジスタで渡すのか

レジスタアクセスはメモリアクセスよりはるかに高速（1サイクル vs 数百サイクル）。
ほとんどの関数は6引数以下なので、スタックを使わずにすむ。

## 実装

### 1. AST の変更

`Function` に `params` フィールドを追加：

```rust
pub struct Function {
    pub name: String,
    pub params: Vec<String>,    // NEW: パラメータ名のリスト
    pub body: Vec<Stmt>,
    pub locals: Vec<String>,    // params も含む
}
```

パラメータはローカル変数の一種として扱う。`locals` にもパラメータ名が含まれるため、
スタック上のスロットが自動的に割り当てられる。

### 2. パーサーの変更

関数定義のパラメータリストをパース：

```rust
// function = "int" ident "(" params? ")" "{" stmt* "}"
// params = "int" ident ("," "int" ident)*

self.expect(TokenKind::LParen);
let mut params = Vec::new();

if self.current().kind != TokenKind::RParen {
    self.expect(TokenKind::Int);
    let name = /* 識別子を読む */;
    params.push(name.clone());
    self.locals.push(name);

    while self.current().kind == TokenKind::Comma {
        self.advance();
        self.expect(TokenKind::Int);
        let name = /* 識別子を読む */;
        params.push(name.clone());
        self.locals.push(name);
    }
}
self.expect(TokenKind::RParen);
```

**重要**: パラメータは `locals` にも追加される。これにより：
- パラメータにスタックスロットが割り当てられる
- 関数本体でパラメータを普通のローカル変数として参照できる

### 3. コード生成の変更

#### 呼ばれる側（callee）: レジスタ → スタック

関数のプロローグ直後に、レジスタの値をスタックスロットに保存する：

```rust
fn gen_function(&mut self, func: &Function) {
    // ... prologue ...

    // Store register parameters to stack
    let arg_regs = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
    for (i, param) in func.params.iter().enumerate() {
        let offset = self.locals[param];
        self.emit(&format!("  mov {}, -{}(%rbp)", arg_regs[i], offset));
    }

    // ... body ...
}
```

**なぜスタックに保存するのか？**

レジスタは呼び出しの際に上書きされる可能性がある（caller-saved）。
パラメータをスタックに保存することで：
1. 関数本体で変数として自由にアクセスできる
2. 再帰呼び出しでレジスタが上書きされても安全
3. ローカル変数と同じ仕組みで読み書きできる

#### 呼ぶ側（caller）: 引数の評価とレジスタへの設定

```rust
Expr::FuncCall { name, args } => {
    // 1. Evaluate args and push results onto stack
    for arg in args.iter() {
        self.gen_expr(arg);
        self.push();      // 結果をスタックに一時保存
    }

    // 2. Pop args into registers (reverse order)
    let arg_regs = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
    for i in (0..args.len()).rev() {
        self.pop(arg_regs[i]);
    }

    // 3. Align and call
    let needs_align = self.stack_depth % 2 != 0;
    if needs_align { self.emit("  sub $8, %rsp"); }
    self.emit(&format!("  call {}", name));
    if needs_align { self.emit("  add $8, %rsp"); }
}
```

### なぜ一度スタックに push してから pop するのか

引数の評価中に他の引数のレジスタを上書きしてしまう問題を防ぐため。

例: `add(a, b + c)` の場合
- 引数1の評価: `a` の値を `%rax` に → push
- 引数2の評価: `b + c` を計算中に `%rdi` を使う可能性がある
- 全ての評価が終わったら、まとめてレジスタに pop

#### pop の順序

push は左から右（arg0, arg1, arg2...）の順で行う。
スタックは LIFO なので、pop は逆順で行い、正しいレジスタに入れる：

```
Push: arg0 → arg1 → arg2
Stack (top → bottom): arg2, arg1, arg0

Pop reverse:
  i=2: pop → arg2 → %rdx ✓
  i=1: pop → arg1 → %rsi ✓
  i=0: pop → arg0 → %rdi ✓
```

## 具体例: 再帰的階乗

入力: `int fact(int n) { if (n <= 1) return 1; return n * fact(n - 1); }`

```asm
  .globl fact
fact:
  push %rbp
  mov %rsp, %rbp
  sub $16, %rsp            # n 用のスタックスロット

  # Store parameter n from %rdi to stack
  mov %rdi, -8(%rbp)       # n = 第1引数

  # if (n <= 1)
  mov -8(%rbp), %rax       # %rax = n
  push %rax                # save lhs
  mov $1, %rax             # %rax = 1
  pop %rdi                 # %rdi = n
  # (ここでは lhs=n, rhs=1 の比較)
  ...
  je .L0                   # false なら else へ

  # return 1
  mov $1, %rax
  jmp .Lreturn.fact

  # return n * fact(n - 1)
  # rhs: fact(n - 1)
  #   arg: n - 1
  mov $1, %rax
  push %rax
  mov -8(%rbp), %rax       # n
  pop %rdi
  sub %rdi, %rax           # n - 1
  push %rax                # push arg
  pop %rdi                 # arg → %rdi
  call fact                # fact(n-1), result in %rax
  push %rax                # save result

  # lhs: n
  mov -8(%rbp), %rax
  pop %rdi                 # %rdi = fact(n-1)
  imul %rdi, %rax          # n * fact(n-1)
  jmp .Lreturn.fact

.Lreturn.fact:
  mov %rbp, %rsp
  pop %rbp
  ret
```

### 再帰の安全性

各 `call fact` で新しいスタックフレームが作られる：
1. `fact(5)`: フレーム1に `n=5` を保存
2. `fact(4)`: フレーム2に `n=4` を保存（フレーム1は保持される）
3. ...
4. `fact(1)`: `return 1` で巻き戻し開始

`%rdi` レジスタは caller-saved なので `call` で上書きされるが、
`n` はスタックの `-8(%rbp)` に保存されているため安全。
各関数呼び出しが独自の `%rbp` を持つため、異なる `n` の値が混ざることはない。

## テストケース

```
assert 7   'int add(int a, int b) { return a + b; } int main() { return add(3, 4); }'
assert 1   'int sub(int a, int b) { return a - b; } int main() { return sub(4, 3); }'
assert 120 'int fact(int n) { if (n<=1) return 1; return n*fact(n-1); } int main() { return fact(5); }'
assert 55  'int fib(int n) { if (n<=1) return n; return fib(n-1)+fib(n-2); } int main() { return fib(10); }'
assert 21  'int add6(int a,int b,int c,int d,int e,int f) { return a+b+c+d+e+f; } int main() { return add6(1,2,3,4,5,6); }'
```

再帰的フィボナッチ（`fib(10) = 55`）は二重再帰を含み、
スタックフレーム管理とスタックアライメントの正確性を厳しくテストする。
