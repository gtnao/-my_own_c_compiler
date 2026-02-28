# Step 3.3: スタック経由の引数（7個以上）

## 概要

System V AMD64 ABI では最初の6個の整数引数はレジスタで渡されるが、7個目以降はスタック経由で渡す。

```c
int add8(int a, int b, int c, int d, int e, int f, int g, int h) {
    return a + b + c + d + e + f + g + h;
}
int main() { return add8(1, 2, 3, 4, 5, 6, 7, 8); }  // => 36
```

## スタック引数のメモリレイアウト

### 呼び出し時のスタック構造

`add8(1,2,3,4,5,6,7,8)` を呼ぶ場合：

```
High addr   [alignment pad] (必要な場合)
            [arg[7] = 8]     ← 先に push (右から左)
            [arg[6] = 7]     ← 後に push
            [return addr]    ← call 命令が push
            [saved %rbp]     ← push %rbp
Low addr    [local vars...]  ← sub $N, %rsp
```

callee（呼ばれる関数）側から見た位置：

| 引数 | 渡し方 | callee でのアクセス |
|------|--------|-------------------|
| 1-6 | レジスタ `%rdi`...`%r9` | プロローグで `-N(%rbp)` に保存 |
| 7th (index 6) | スタック | `16(%rbp)` |
| 8th (index 7) | スタック | `24(%rbp)` |
| N番目 | スタック | `16 + (N-7) * 8 (%rbp)` |

### なぜ 16(%rbp) から始まるのか

```
0(%rbp)  = saved %rbp（push %rbp で保存）
8(%rbp)  = return address（call 命令で push）
16(%rbp) = 7番目の引数 ← ここから
24(%rbp) = 8番目の引数
...
```

## 実装

### Callee 側: スタック引数のローカル変数への保存

パラメータ7個目以降は `+16(%rbp)`, `+24(%rbp)` ... から読み取り、
ローカル変数のスタックスロットにコピーする：

```rust
// Register parameters (first 6)
for (i, param) in func.params.iter().enumerate().take(6) {
    let offset = self.locals[param];
    self.emit(&format!("  mov {}, -{}(%rbp)", arg_regs[i], offset));
}

// Stack parameters (7th and beyond)
for (i, param) in func.params.iter().enumerate().skip(6) {
    let src_offset = 16 + (i - 6) * 8;   // caller's stack
    let dst_offset = self.locals[param];   // local slot
    self.emit(&format!("  mov {}(%rbp), %rax", src_offset));
    self.emit(&format!("  mov %rax, -{}(%rbp)", dst_offset));
}
```

スタック引数もローカル変数スロットにコピーすることで、
関数本体では全パラメータを統一的に `-N(%rbp)` でアクセスできる。

### Caller 側: アライメントとスタック引数の push

#### 重要なバグの教訓: アライメントの順序

最初の実装では、スタック引数を push した**後**にアライメント調整を行っていた。
これにより `sub $8, %rsp` がスタック引数の位置をずらし、callee が間違った値を読む問題が発生した。

**修正**: アライメントはスタック引数の**前**に行う。

```rust
Expr::FuncCall { name, args } => {
    let num_stack_args = if args.len() > 6 { args.len() - 6 } else { 0 };

    // 1. Align BEFORE stack args
    let needs_align = (self.stack_depth + num_stack_args) % 2 != 0;
    if needs_align {
        self.emit("  sub $8, %rsp");
        self.stack_depth += 1;
    }

    // 2. Push stack arguments (reverse order: rightmost first)
    for i in (6..args.len()).rev() {
        self.gen_expr(&args[i]);
        self.push();
    }

    // 3. Evaluate register arguments
    let reg_count = std::cmp::min(args.len(), 6);
    for i in 0..reg_count {
        self.gen_expr(&args[i]);
        self.push();
    }
    for i in (0..reg_count).rev() {
        self.pop(arg_regs[i]);
    }

    // 4. Call (stack is already aligned)
    self.emit(&format!("  call {}", name));

    // 5. Clean up: stack args first, then alignment pad
    if num_stack_args > 0 {
        self.emit(&format!("  add ${}, %rsp", num_stack_args * 8));
        self.stack_depth -= num_stack_args;
    }
    if needs_align {
        self.emit("  add $8, %rsp");
        self.stack_depth -= 1;
    }
}
```

#### スタックの正しいレイアウト

アライメントを先に行った場合のスタック：

```
[alignment pad]   ← (1) 先に push（callee からは見えない）
[arg[7]]          ← (2) 後に push（24(%rbp) でアクセス）
[arg[6]]          ← (2) 最後に push（16(%rbp) でアクセス）
```

アライメントを後に行った場合（バグ版）：

```
[arg[7]]          ← callee は 24(%rbp) でアクセス → ★ 間違い
[arg[6]]          ← callee は 16(%rbp) でアクセス → ★ 間違い
[alignment pad]   ← これが 16(%rbp) に来てしまう
```

#### アライメント判定の基準

`(current_stack_depth + num_stack_args) % 2 != 0` で判定する。
- `current_stack_depth`: 外側の式評価で push された数
- `num_stack_args`: この呼び出しで push されるスタック引数の数
- 合計が奇数なら 8 バイトのパディングが必要

## 具体例: 7引数関数

入力: `int main() { return add7(1,2,3,4,5,6,7); }`

```asm
main:
  push %rbp
  mov %rsp, %rbp

  # Alignment: stack_depth=0, stack_args=1 → need pad
  sub $8, %rsp              # alignment pad

  # Stack arg: push arg[6] = 7
  mov $7, %rax
  push %rax

  # Register args: evaluate and push
  mov $1, %rax              # push → pop %rdi
  push %rax
  mov $2, %rax              # push → pop %rsi
  push %rax
  ...
  pop %r9                   # 6
  pop %r8                   # 5
  pop %rcx                  # 4
  pop %rdx                  # 3
  pop %rsi                  # 2
  pop %rdi                  # 1

  call add7                 # stack is 16-aligned ✓

  # Clean up
  add $8, %rsp              # remove stack arg
  add $8, %rsp              # remove alignment pad

  jmp .Lreturn.main
```

callee 側:
```asm
add7:
  push %rbp
  mov %rsp, %rbp
  sub $64, %rsp

  # Register params → stack
  mov %rdi, -8(%rbp)        # a = 1
  mov %rsi, -16(%rbp)       # b = 2
  ...
  mov %r9, -48(%rbp)        # f = 6

  # Stack param → local slot
  mov 16(%rbp), %rax        # g = 7 ← alignment pad の上にある
  mov %rax, -56(%rbp)
```
