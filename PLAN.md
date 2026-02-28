# Cコンパイラ インクリメンタル構築 マスタープラン

## Context

RustでCコンパイラをゼロから構築する。chibicc (Rui Ueyama) に着想を得たインクリメンタルアプローチで、各ステップが動作するコンパイラを生成し、段階的にC言語の機能を追加していく。最終的にC言語の主要機能を網羅する完全なコンパイラを目指す。

- **言語**: Rust (edition 2024)
- **ターゲット**: x86-64 Linux, AT&T syntax assembly
- **ツールチェーン**: GCC/as/ld でアセンブル・リンク
- **外部依存**: なし (純Rust実装)

## ソースコード構成 (最終形)

```
src/
  main.rs          -- CLI entry point
  token.rs         -- Token definitions
  lexer.rs         -- Tokenizer
  ast.rs           -- AST node definitions
  parser.rs        -- Recursive descent parser
  types.rs         -- Type system
  sema.rs          -- Semantic analysis (type checking)
  codegen.rs       -- x86-64 code generation
  preprocess.rs    -- Preprocessor
  error.rs         -- Error reporting
tests/
  test.sh          -- Integration test runner
```

## テスト戦略

- `test.sh`: Cソースをコンパイル → GCCでアセンブル・リンク → 実行 → 終了コード/出力を検証
- `cargo test`: Rust内のユニットテスト

---

## Phase 1: 基礎 — 整数、算術、基本的な式 (7 steps)

### Step 1.1: 単一の整数リテラルをコンパイル

**追加するC機能**: 数値 → `main` 関数の `return` として生成

**入力例**: `42`

**生成するアセンブリ**:
```asm
.globl main
main:
  mov $42, %rax
  ret
```

**変更するコンポーネント**:
- `main.rs` のみ (モジュール分割なし)
- コマンドライン引数から入力を受け取り、アセンブリを標準出力に出力

**テスト**:
- `assert 0 '0'`
- `assert 42 '42'`
- `assert 255 '255'`

---

### Step 1.2: 加減算

**追加するC機能**: `+` と `-` 演算子

**入力例**: `5+20-4`

**生成するアセンブリ**:
```asm
.globl main
main:
  mov $5, %rax
  add $20, %rax
  sub $4, %rax
  ret
```

**変更するコンポーネント**:
- `main.rs` 内に簡易トークナイザーを追加

**テスト**:
- `assert 21 '5+20-4'`
- `assert 0 '0+0'`
- `assert 10 '10'`

---

### Step 1.3: トークナイザーの分離とスペース対応

**追加するC機能**: トークン間のスペースを許容

**入力例**: `5 + 20 - 4`

**変更するコンポーネント**:
- `src/token.rs` を新規作成: `Token` enum (Num, Plus, Minus, EOF)
- `src/lexer.rs` を新規作成: スペースをスキップするトークナイザー
- `main.rs` をリファクタリングしてモジュールを使用

**テスト**:
- `assert 41 ' 12 + 34 - 5 '`
- 不正入力に対するエラーメッセージのテスト

---

### Step 1.4: 乗除算、優先順位、括弧、単項演算子

**追加するC機能**: `*`, `/`, `()`, 単項 `+`, `-`

**入力例**: `5 + 6 * 7`, `(2 + 3) * 4`, `-10 + 20`

**変更するコンポーネント**:
- `token.rs`: `Star`, `Slash`, `LParen`, `RParen` トークン追加
- `src/ast.rs` を新規作成: `ASTNode` enum (Num, BinOp)
- `src/parser.rs` を新規作成: 再帰下降パーサー
  - `expr = mul ("+" mul | "-" mul)*`
  - `mul = unary ("*" unary | "/" unary)*`
  - `unary = ("+" | "-")? primary`
  - `primary = num | "(" expr ")"`
- `src/codegen.rs` を新規作成: ASTからアセンブリ生成
  - スタックマシン方式: 各式の結果を `%rax` に入れ、二項演算時にスタックを使う

**生成するアセンブリ** (例: `2 * 3 + 4`):
```asm
.globl main
main:
  push %rbp
  mov %rsp, %rbp
  mov $2, %rax
  push %rax
  mov $3, %rax
  pop %rdi
  imul %rdi, %rax
  push %rax
  mov $4, %rax
  pop %rdi
  add %rdi, %rax
  mov %rbp, %rsp
  pop %rbp
  ret
```

**テスト**:
- `assert 47 '5+6*7'`
- `assert 15 '5*(9-6)'`
- `assert 4 '(3+5)/2'`
- `assert 10 '-10+20'`
- `assert 10 '- -10'`
- `assert 10 '- - +10'`

---

### Step 1.5: 剰余演算子

**追加するC機能**: `%` (剰余)

**入力例**: `10 % 3`

**変更するコンポーネント**:
- `token.rs`: `Percent` トークン追加
- `parser.rs`: `mul` 規則に `%` 追加
- `codegen.rs`: `cqto` + `idiv` で `%rdx` に剰余

**テスト**:
- `assert 1 '10 % 3'`
- `assert 0 '6 % 3'`
- `assert 2 '11 % 3'`

---

### Step 1.6: 比較演算子

**追加するC機能**: `==`, `!=`, `<`, `<=`, `>`, `>=`

**入力例**: `1 == 1`, `3 < 5`, `10 >= 10`

**変更するコンポーネント**:
- `token.rs`: `EqEq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge` トークン追加
- `lexer.rs`: 2文字トークンの対応
- `parser.rs`: 優先順位追加
  - `expr = relational ("==" relational | "!=" relational)*`
  - `relational = add ("<" add | "<=" add | ">" add | ">=" add)*`
  - `add = mul ("+" mul | "-" mul)*`
- `codegen.rs`: `cmp` + `sete`/`setne`/`setl`/`setle`/`setg`/`setge` + `movzb`

**生成するアセンブリ** (例: `1 == 1`):
```asm
  mov $1, %rax
  push %rax
  mov $1, %rax
  pop %rdi
  cmp %rax, %rdi
  sete %al
  movzb %al, %rax
```

**テスト**:
- `assert 1 '1==1'`, `assert 0 '1==2'`
- `assert 1 '1!=2'`, `assert 0 '1!=1'`
- `assert 1 '1<2'`, `assert 0 '2<1'`
- `assert 1 '1<=1'`, `assert 1 '1<=2'`, `assert 0 '2<=1'`
- `assert 1 '2>1'`, `assert 1 '2>=2'`

---

### Step 1.7: 入力をファイルから読む

**追加するC機能**: コマンドライン引数でファイルパスを受け取り、ファイル内容をコンパイル

**変更するコンポーネント**:
- `main.rs`: ファイル読み込みロジック追加
- `src/error.rs` を新規作成: ソース位置付きエラーメッセージ (行番号、列番号)

**テスト**:
- ファイルベースのテストに切り替え

---

## Phase 2: 文と制御フロー (15 steps)

### Step 2.1: return文と式文、セミコロン

**追加するC機能**: `return` 文、式文、セミコロン、`int main() { ... }` の骨格

**コンパイル可能になるCコード**:
```c
int main() { return 42; }
int main() { 1; 2; return 3; }
```

**変更するコンポーネント**:
- `token.rs`: `Return`, `Int`, `LBrace`, `RBrace`, `Semicolon`, `Ident` トークン追加
- `lexer.rs`: キーワード認識 (`return`, `int`)、識別子、`{`, `}`, `;`
- `ast.rs`: `Function`, `ReturnStmt`, `ExprStmt`, `Block` ノード追加
- `parser.rs`: `int main() { stmt* }` をパース
  - `program = function`
  - `function = type ident "(" ")" "{" stmt* "}"`
  - `stmt = "return" expr ";" | expr ";"`
- `codegen.rs`: 関数プロローグ/エピローグ生成、複数文対応

**テスト**:
- `assert 3 'int main() { return 3; }'`
- `assert 3 'int main() { 1; 2; return 3; }'`

---

### Step 2.2: ローカル変数 (単一文字)

**追加するC機能**: 1文字変数、代入演算子 `=`

**コンパイル可能になるCコード**:
```c
int main() { int a; a = 3; return a; }
int main() { int a; int b; a = 3; b = 5; return a + b; }
```

**変更するコンポーネント**:
- `token.rs`: `Assign` トークン追加
- `ast.rs`: `VarDecl`, `Assign`, `Var` ノード追加
- `parser.rs`: 変数宣言と代入文のパース
- `codegen.rs`: ローカル変数をスタック上に配置
  - `a` → `rbp-8`, `b` → `rbp-16` 等

**スタックフレーム**:
```
rbp     -> saved rbp
rbp-8   -> variable a
rbp-16  -> variable b
```

**テスト**:
- `assert 3 'int main() { int a; a = 3; return a; }'`
- `assert 8 'int main() { int a; int b; a = 3; b = 5; return a + b; }'`

---

### Step 2.3: 複数文字の変数名

**追加するC機能**: 任意の長さの識別子 `[a-zA-Z_][a-zA-Z0-9_]*`

**コンパイル可能になるCコード**:
```c
int main() { int foo; int bar; foo = 1; bar = 2; return foo + bar; }
```

**変更するコンポーネント**:
- `lexer.rs`: 識別子の正規化
- `parser.rs`: 変数テーブルをハッシュマップに変更

**テスト**:
- `assert 3 'int main() { int foo; int bar; foo = 1; bar = 2; return foo + bar; }'`

---

### Step 2.4: if文

**追加するC機能**: `if`, `else`

**コンパイル可能になるCコード**:
```c
int main() { if (1) return 1; return 0; }
int main() { if (0) return 1; else return 2; }
```

**変更するコンポーネント**:
- `token.rs`: `If`, `Else` トークン
- `ast.rs`: `IfStmt` ノード (condition, then_body, else_body)
- `parser.rs`: `if "(" expr ")" stmt ("else" stmt)?`
- `codegen.rs`: ラベルベースの条件分岐

**生成するアセンブリ**:
```asm
  # condition
  cmp $0, %rax
  je .Lelse_N
  # then body
  jmp .Lend_N
.Lelse_N:
  # else body
.Lend_N:
```

**テスト**:
- `assert 1 'int main() { if (1) return 1; return 0; }'`
- `assert 0 'int main() { if (0) return 1; return 0; }'`
- `assert 2 'int main() { if (0) return 1; else return 2; }'`

---

### Step 2.5: while文

**追加するC機能**: `while` ループ

**コンパイル可能になるCコード**:
```c
int main() { int i; i = 0; while (i < 10) i = i + 1; return i; }
```

**生成するアセンブリ**:
```asm
.Lbegin_N:
  # condition
  cmp $0, %rax
  je .Lend_N
  # body
  jmp .Lbegin_N
.Lend_N:
```

**テスト**:
- `assert 10 'int main() { int i; i=0; while(i<10) i=i+1; return i; }'`
- `assert 55 'int main() { int s; int i; s=0; i=1; while(i<=10){s=s+i; i=i+1;} return s; }'`

---

### Step 2.6: for文

**追加するC機能**: `for` ループ

**コンパイル可能になるCコード**:
```c
int main() { int s; s = 0; for (int i = 0; i < 10; i = i + 1) s = s + i; return s; }
```

**生成するアセンブリ**:
```asm
  # init
.Lbegin_N:
  # condition
  cmp $0, %rax
  je .Lend_N
  # body
  # increment
  jmp .Lbegin_N
.Lend_N:
```

**テスト**:
- `assert 45 'int main() { int s=0; for(int i=0;i<10;i=i+1) s=s+i; return s; }'`

---

### Step 2.7: ブロック文 (複合文)

**追加するC機能**: `{ }` によるネストされたブロック

**コンパイル可能になるCコード**:
```c
int main() { { int a; a = 1; return a; } }
```

**テスト**:
- `assert 3 'int main() { { return 3; } }'`

---

### Step 2.8: 変数宣言と初期化の統合

**追加するC機能**: `int a = 5;`

**コンパイル可能になるCコード**:
```c
int main() { int a = 3; int b = 5; return a + b; }
```

**変更するコンポーネント**:
- `parser.rs`: `type ident ("=" expr)? ";"` のパース
- `ast.rs`: `VarDecl` に初期化式を追加

**テスト**:
- `assert 8 'int main() { int a = 3; int b = 5; return a + b; }'`

---

### Step 2.9: 複合代入演算子

**追加するC機能**: `+=`, `-=`, `*=`, `/=`

**コンパイル可能になるCコード**:
```c
int main() { int a = 10; a += 5; return a; }
```

**変更するコンポーネント**:
- `token.rs`: `PlusAssign`, `MinusAssign`, `StarAssign`, `SlashAssign`
- `parser.rs`: 複合代入を糖衣構文として展開 (`a += b` → `a = a + b`)

**テスト**:
- `assert 15 'int main() { int a=10; a+=5; return a; }'`
- `assert 5 'int main() { int a=10; a-=5; return a; }'`
- `assert 20 'int main() { int a=10; a*=2; return a; }'`
- `assert 5 'int main() { int a=10; a/=2; return a; }'`

---

### Step 2.10: インクリメント/デクリメント

**追加するC機能**: `++`, `--` (前置・後置)

**コンパイル可能になるCコード**:
```c
int main() { int a = 5; a++; return a; }    // 6
int main() { int a = 5; return a++; }       // 5
int main() { int a = 5; return ++a; }       // 6
```

**変更するコンポーネント**:
- `token.rs`: `PlusPlus`, `MinusMinus`
- `ast.rs`: `PreInc`, `PreDec`, `PostInc`, `PostDec` ノード
- `parser.rs`: 前置は `unary` に、後置は `postfix` に追加
- `codegen.rs`: 後置は古い値を保存してから変更

**テスト**:
- `assert 6 'int main() { int a=5; a++; return a; }'`
- `assert 5 'int main() { int a=5; return a++; }'`
- `assert 6 'int main() { int a=5; return ++a; }'`
- `assert 4 'int main() { int a=5; a--; return a; }'`

---

### Step 2.11: 論理演算子

**追加するC機能**: `&&`, `||`, `!` (短絡評価)

**コンパイル可能になるCコード**:
```c
int main() { return 1 && 2; }
int main() { return 0 || 1; }
int main() { return !0; }
```

**変更するコンポーネント**:
- `token.rs`: `And`, `Or`, `Not` トークン
- `ast.rs`: `LogicalAnd`, `LogicalOr`, `LogicalNot` ノード
- `parser.rs`: 優先順位追加
- `codegen.rs`: 短絡評価の実装

**生成するアセンブリ** (例: `a && b`):
```asm
  # eval a
  cmp $0, %rax
  je .Lfalse_N      # short-circuit
  # eval b
  cmp $0, %rax
  je .Lfalse_N
  mov $1, %rax
  jmp .Lend_N
.Lfalse_N:
  mov $0, %rax
.Lend_N:
```

**テスト**:
- `assert 1 'int main() { return 1&&1; }'`
- `assert 0 'int main() { return 1&&0; }'`
- `assert 1 'int main() { return 0||1; }'`
- `assert 0 'int main() { return 0||0; }'`
- `assert 1 'int main() { return !0; }'`
- `assert 0 'int main() { return !1; }'`

---

### Step 2.12: ビット演算子

**追加するC機能**: `&`, `|`, `^`, `~`, `<<`, `>>`

**コンパイル可能になるCコード**:
```c
int main() { return 3 & 1; }
int main() { return 1 << 3; }
```

**変更するコンポーネント**:
- `token.rs`: `Amp`, `Pipe`, `Caret`, `Tilde`, `LShift`, `RShift`
- `lexer.rs`: `&` vs `&&`, `|` vs `||` の区別
- `parser.rs`: C標準の完全な優先順位
- `codegen.rs`: `and`, `or`, `xor`, `not`, `sal`, `sar` 命令

**テスト**:
- `assert 1 'int main() { return 3&1; }'`
- `assert 3 'int main() { return 1|2; }'`
- `assert 3 'int main() { return 1^2; }'`
- `assert 8 'int main() { return 1<<3; }'`
- `assert 2 'int main() { return 8>>2; }'`

---

### Step 2.13: コンマ演算子と三項演算子

**追加するC機能**: `,` (コンマ演算子)、`? :` (三項演算子)

**コンパイル可能になるCコード**:
```c
int main() { return (1, 2, 3); }      // 3
int main() { return 1 ? 10 : 20; }    // 10
int main() { return 0 ? 10 : 20; }    // 20
```

**テスト**:
- `assert 3 'int main() { return (1,2,3); }'`
- `assert 10 'int main() { return 1?10:20; }'`
- `assert 20 'int main() { return 0?10:20; }'`

---

### Step 2.14: do-while文、switch/case/default/break

**追加するC機能**: `do { } while();`, `switch`/`case`/`default`/`break`

**コンパイル可能になるCコード**:
```c
int main() {
  int i = 0;
  do { i++; } while (i < 5);
  return i;
}
```
```c
int main() {
  int a = 2;
  switch (a) {
    case 1: return 10;
    case 2: return 20;
    default: return 30;
  }
}
```

**変更するコンポーネント**:
- `token.rs`: `Do`, `Switch`, `Case`, `Default`, `Break` トークン
- `ast.rs`: `DoWhileStmt`, `SwitchStmt`, `CaseClause`, `BreakStmt` ノード
- `codegen.rs`: break用のラベルスタック管理

**テスト**:
- `assert 5 'int main() { int i=0; do{i++;}while(i<5); return i; }'`
- `assert 20 'int main() { int a=2; switch(a){case 1:return 10;case 2:return 20;default:return 30;} }'`

---

### Step 2.15: continue文、goto文、ラベル

**追加するC機能**: `continue`, `goto`, ラベル付き文

**コンパイル可能になるCコード**:
```c
int main() {
  int s = 0;
  for (int i = 0; i < 10; i++) {
    if (i % 2 == 0) continue;
    s += i;
  }
  return s; // 1+3+5+7+9 = 25
}
```
```c
int main() { goto end; return 1; end: return 2; }
```

**テスト**:
- `assert 25 'int main() { int s=0; for(int i=0;i<10;i++){if(i%2==0)continue; s+=i;} return s; }'`
- `assert 2 'int main() { goto end; return 1; end: return 2; }'`

---

## Phase 3: 関数 (6 steps)

### Step 3.1: 関数呼び出し (引数なし)・複数関数定義

**追加するC機能**: 引数なしの関数呼び出し、同一ファイル内の複数関数定義

**コンパイル可能になるCコード**:
```c
int ret3() { return 3; }
int ret5() { return 5; }
int main() { return ret3() + ret5(); }
```

**変更するコンポーネント**:
- `ast.rs`: `FuncCall` ノード、`Program` が Function のリスト
- `parser.rs`: `primary` に関数呼び出し追加、複数関数定義
- `codegen.rs`: `call` 命令、RSP 16バイトアライメント

**テスト**:
- `assert 3 'int ret3(){return 3;} int main(){return ret3();}'`
- `assert 8 'int ret3(){return 3;} int ret5(){return 5;} int main(){return ret3()+ret5();}'`

---

### Step 3.2: 関数引数 (最大6個)

**追加するC機能**: 関数パラメータ (System V AMD64 ABI: rdi, rsi, rdx, rcx, r8, r9)

**コンパイル可能になるCコード**:
```c
int add(int a, int b) { return a + b; }
int main() { return add(3, 5); }
```

**変更するコンポーネント**:
- `ast.rs`: パラメータリスト、引数リスト追加
- `codegen.rs`: 呼び出し側→レジスタに配置、被呼び出し側→レジスタからスタックにコピー

**テスト**:
- `assert 8 'int add(int a,int b){return a+b;} int main(){return add(3,5);}'`
- `assert 21 'int add6(int a,int b,int c,int d,int e,int f){return a+b+c+d+e+f;} int main(){return add6(1,2,3,4,5,6);}'`
- `assert 120 'int fact(int n){if(n<=1)return 1;return n*fact(n-1);} int main(){return fact(5);}'` (再帰)

---

### Step 3.3: スタック経由の引数 (7個以上)

**追加するC機能**: 7個以上の引数をスタック経由で渡す

**コンパイル可能になるCコード**:
```c
int add8(int a, int b, int c, int d, int e, int f, int g, int h) {
  return a + b + c + d + e + f + g + h;
}
int main() { return add8(1,2,3,4,5,6,7,8); }
```

**テスト**:
- `assert 36 '...(上記コード)...'`

---

### Step 3.4: 前方宣言とvoid関数

**追加するC機能**: 関数プロトタイプ宣言、`void` 戻り値型

**コンパイル可能になるCコード**:
```c
int add(int a, int b);
int main() { return add(3, 5); }
int add(int a, int b) { return a + b; }
```

**テスト**:
- 前方宣言後に定義された関数を呼べること
- void関数が正常に動作すること

---

### Step 3.5: 変数スコープ (ブロックスコープ、シャドウイング)

**追加するC機能**: ブロックスコープ、シャドウイング

**コンパイル可能になるCコード**:
```c
int main() {
  int a = 1;
  { int a = 2; }
  return a; // 1
}
```

**変更するコンポーネント**:
- スコープのスタック管理: ブロック開始でpush、終了でpop
- 変数検索は内側→外側

**テスト**:
- `assert 1 'int main() { int a=1; {int a=2;} return a; }'`
- `assert 2 'int main() { int a=1; {a=2;} return a; }'` (同じ変数への再代入)

---

### Step 3.6: グローバル変数

**追加するC機能**: グローバル変数の宣言と使用

**コンパイル可能になるCコード**:
```c
int g;
int main() { g = 5; return g; }
```

**変更するコンポーネント**:
- `codegen.rs`: `.data`/`.bss` セクション、RIP相対アドレッシング (`g(%rip)`)

**生成するアセンブリ**:
```asm
.data
.globl g
g:
  .long 10

.text
.globl main
main:
  ...
  mov g(%rip), %eax
```

**テスト**:
- `assert 5 'int g; int main(){g=5; return g;}'`
- `assert 10 'int g=10; int main(){return g;}'`

---

## Phase 4: 型システム (8 steps)

### Step 4.1: 型の内部表現

**追加するC機能**: 型システムの基盤構築 (新しいC機能は追加しない)

**変更するコンポーネント**:
- `src/types.rs` を新規作成:
  ```rust
  pub enum Type {
      Int, Char, Short, Long, Void,
      Ptr(Box<Type>), Array(Box<Type>, usize),
      Func { ret: Box<Type>, params: Vec<Type> },
      Struct { name: Option<String>, members: Vec<Member> },
      Union { name: Option<String>, members: Vec<Member> },
      Enum,
  }
  ```
- `src/sema.rs` を新規作成: 型推論・型チェックの基盤
- 既存のASTノードに型情報を付加

**テスト**: 既存テストのリグレッションテスト

---

### Step 4.2: char型
- サイズ1バイト、`movb`/`movsbl` 命令
- `assert 65 'int main() { char a=65; return a; }'`

### Step 4.3: short型、long型
- short=2バイト (`movw`/`movswl`), long=8バイト (`movq`)

### Step 4.4: 暗黙的型変換 (整数昇格)
- char/short → int、int + long → long

### Step 4.5: 明示的型キャスト
- `(type)expr`
- `assert 0 'int main() { int a=256; return (char)a; }'`

### Step 4.6: sizeof演算子
- `sizeof(type)`, `sizeof(expr)`
- `assert 4 'int main() { return sizeof(int); }'`

### Step 4.7: unsigned型
- `unsigned char/short/int/long`、符号なし比較・除算

### Step 4.8: _Bool型
- サイズ1、0/1に正規化

---

## Phase 5: ポインタと配列 (6 steps)

### Step 5.1: アドレス演算子とデリファレンス
- `&` (lea命令), `*` (間接参照)
- `assert 3 'int main() { int a=3; int *p=&a; return *p; }'`

### Step 5.2: ポインタ算術
- ptr+int → `sizeof(*ptr)` 倍のオフセット加算

### Step 5.3: 配列
- `int a[3]; a[0]=1;` — `a[i]` は `*(a+i)` に変換
- `assert 6 'int main() { int a[3]; a[0]=1;a[1]=2;a[2]=3; return a[0]+a[1]+a[2]; }'`

### Step 5.4: 多次元配列
- `int a[2][3]`

### Step 5.5: グローバル配列

### Step 5.6: sizeof と配列
- `assert 40 'int main() { int a[10]; return sizeof(a); }'`

---

## Phase 6: 文字列 (4 steps)

### Step 6.1: 文字列リテラル
- `.data` セクションに配置、エスケープシーケンス
- `assert 104 'int main() { char *s="hello"; return s[0]; }'`

### Step 6.2: エスケープシーケンス完全対応
- `\n`, `\t`, `\r`, `\\`, `\"`, `\'`, `\0`, `\a`, `\b`, `\f`, `\v`, `\x??`, `\???`

### Step 6.3: 文字リテラル
- `'A'` → 65

### Step 6.4: 文字列連結
- `"hello" " " "world"`

---

## Phase 7: 構造体とユニオン (6 steps)

### Step 7.1: 構造体の定義とメンバアクセス `.`
- `struct { int a; int b; } s; s.a = 1;`

### Step 7.2: アライメントとパディング
- `sizeof(struct { char a; int b; })` → 8

### Step 7.3: アロー演算子 `->`
- `p->a` == `(*p).a`

### Step 7.4: タグ付き構造体、構造体の値コピー
- `struct Point { int x; int y; };`、構造体代入は memcpy 相当

### Step 7.5: ユニオン
- 全メンバがオフセット0、サイズは最大メンバ

### Step 7.6: ネストされた構造体/ユニオン

---

## Phase 8: enum と typedef (2 steps)

### Step 8.1: enum
- 自動採番、明示的値指定
- `assert 1 'int main() { enum{A,B,C}; return B; }'`

### Step 8.2: typedef
- `typedef struct { int x; int y; } Point;`
- typedef名は型名として認識 (パーサーがtypedefテーブルを参照)

---

## Phase 9: 初期化子と記憶域クラス (7 steps)

### Step 9.1: 配列初期化子
- `int a[] = {1,2,3};`

### Step 9.2: 構造体初期化子
- `struct S s = {1, 2};`

### Step 9.3: 指示子付き初期化子
- `.member = val`, `[idx] = val`

### Step 9.4: 複合リテラル
- `(int[]){1,2,3}`

### Step 9.5: グローバル変数の静的初期化
- `.data` セクションでの `.long 42` 等

### Step 9.6: static ローカル変数
- `static int c = 0;`

### Step 9.7: extern宣言

---

## Phase 10: プリプロセッサ (8 steps)

### Step 10.1: コメント (`//`, `/* */`)
### Step 10.2: `#include "file"`, `#include <file>`
### Step 10.3: `#define` (オブジェクト形式マクロ)
### Step 10.4: `#define` (関数形式マクロ)
### Step 10.5: 条件付きコンパイル (`#ifdef`/`#ifndef`/`#if`/`#else`/`#elif`/`#endif`)
### Step 10.6: `#`/`##` 演算子 (文字列化/トークン連結)
### Step 10.7: 事前定義マクロ (`__FILE__`, `__LINE__`, `__func__`)
### Step 10.8: `#error`, `#warning`, `#line`, `#pragma`

---

## Phase 11: 標準ライブラリ互換性 (4 steps)

### Step 11.1: printf呼び出し対応 (libc リンク)
### Step 11.2: 可変長引数 (`...`, `va_list`, `va_start`, `va_arg`)
### Step 11.3: 関数ポインタ (`int (*fp)(int, int)`)
### Step 11.4: コールバックパターン

---

## Phase 12: 高度なC機能 (12 steps)

### Step 12.1: 関数パラメータとしての配列 `int a[]` → `int *a`
### Step 12.2: 構造体の値渡しと値返し (ABI準拠)
### Step 12.3: 文字列初期化による配列 `char s[] = "hello";`
### Step 12.4: const修飾子
### Step 12.5: volatile修飾子
### Step 12.6: for文のスコープ内宣言の改善
### Step 12.7: 複雑な型宣言 (ポインタの配列、配列へのポインタ)
### Step 12.8: 同一行での複数変数宣言 `int a=1, b=2;`
### Step 12.9: 構造体ビットフィールド
### Step 12.10: 柔軟配列メンバ (flexible array member)
### Step 12.11: `_Alignof`, `_Alignas`
### Step 12.12: `_Generic`

---

## Phase 13: 最適化とポリッシュ (7 steps)

### Step 13.1: 定数畳み込み
### Step 13.2: 不要なpush/popの除去 (ピープホール最適化)
### Step 13.3: レジスタ割り付けの改善
### Step 13.4: エラーメッセージの改善 (GCC風、位置情報付き)
### Step 13.5: デバッグ情報 (`.file`/`.loc` ディレクティブ)
### Step 13.6: 複数ファイルのコンパイル (`-o`, `-c`, `-S`, `-E` オプション)
### Step 13.7: 総合テスト (FizzBuzz, リンクリスト, 簡易電卓, qsort)

---

## Phase 14: PostgreSQL互換性修正 (30 steps, 完了済み)

Steps 14.1–14.30: PostgreSQLコンパイルに必要な様々なバグ修正と機能追加。
詳細は `docs/step_14_*.md` を参照。

---

## Phase 15: 高度な宣言とType System拡張 (8 steps)

### Step 15.1: 同一行での複数変数宣言（完全対応）
- `int a, b, *c, d[10];` — カンマ区切りの宣言子を正しくパース
- ポインタ修飾子やarray修飾子を個別の宣言子ごとに処理
- PostgreSQLは `int i, j, k;` パターンを多用

### Step 15.2: long long型
- `long long` / `unsigned long long` の独立した型としてのサポート
- 現在は `long` と同一視されているが、型名として区別する必要がある
- PostgreSQLの `int64` / `uint64` 型に対応

### Step 15.3: 複雑な型宣言子パース強化
- 関数ポインタを返す関数: `int (*foo(void))(int)`
- ポインタの配列: `int *a[10]`
- 配列へのポインタ: `int (*a)[10]`
- 関数ポインタの配列: `int (*ops[10])(int, int)`

### Step 15.4: K&R スタイル関数宣言（レガシーサポート）
- 古い形式の関数宣言: `int foo(a, b) int a; int b; { ... }`
- PostgreSQLの古いコードや外部ライブラリで使用される可能性

### Step 15.5: 匿名構造体/ユニオンメンバ（C11）
- `struct { union { int a; float b; }; int c; };`
- 外側の構造体から直接メンバにアクセス可能

### Step 15.6: 関数プロトタイプにおける抽象宣言子
- `void foo(int (*)(int, int))` — パラメータ名なしの宣言
- `sizeof(int (*)[10])` — sizeof内での複雑な型

### Step 15.7: typeof / __typeof__ の完全実装
- `typeof(expr)` および `typeof(type)` の完全サポート
- 変数宣言で使用: `typeof(x) y = x;`
- PostgreSQLのマクロで使用

### Step 15.8: _Static_assert の実装
- `_Static_assert(expr, "message")` — コンパイル時アサーション
- PostgreSQLの StaticAssertStmt マクロに対応

---

## Phase 16: プリプロセッサ拡張 (8 steps)

### Step 16.1: 可変引数マクロ（__VA_ARGS__）
- `#define LOG(fmt, ...) printf(fmt, __VA_ARGS__)`
- PostgreSQLの `elog()` / `ereport()` マクロに必須

### Step 16.2: defined() 演算子
- `#if defined(FOO)` および `#if defined FOO`
- `#if !defined(BAR) && defined(BAZ)`

### Step 16.3: #if における複雑な定数式
- `#if SIZEOF_LONG == 8`
- `#if defined(A) && (B > 2 || C == 0)`
- 整数定数式の完全な評価

### Step 16.4: #undef ディレクティブ
- マクロの未定義化
- ヘッダーガードの管理に重要

### Step 16.5: 事前定義マクロの拡張
- `__STDC__`, `__STDC_VERSION__`, `__STDC_HOSTED__`
- `__GNUC__`, `__GNUC_MINOR__` (GCC互換)
- `__LP64__`, `__x86_64__`, `__linux__`
- コンパイラ識別マクロ

### Step 16.6: #pragma once と #pragma pack
- ヘッダーの重複インクルード防止
- 構造体パッキング制御

### Step 16.7: マクロ内改行とバックスラッシュ継続行
- `#define MACRO(x) \`  による複数行マクロ
- 既存のバックスラッシュ継続行サポートの強化

### Step 16.8: 条件付きインクルードと #include_next
- `#include_next <header.h>` (GCC拡張)
- ヘッダーサーチパスの完全対応

---

## Phase 17: 標準ライブラリヘッダスタブ (6 steps)

### Step 17.1: stddef.h / stdint.h / stdbool.h スタブ
- `NULL`, `size_t`, `ptrdiff_t`, `offsetof` マクロ
- `int8_t` ～ `uint64_t`、`intmax_t`、`uintmax_t`
- `true`, `false`, `bool`

### Step 17.2: stdio.h / stdlib.h スタブ
- `FILE *`, `stdin`/`stdout`/`stderr` 宣言
- `printf`, `fprintf`, `sprintf`, `snprintf` 宣言
- `malloc`, `calloc`, `realloc`, `free` 宣言
- `exit`, `abort`, `atexit` 宣言

### Step 17.3: string.h / strings.h スタブ
- `memcpy`, `memset`, `memmove`, `memcmp` 宣言
- `strlen`, `strcpy`, `strncpy`, `strcmp`, `strncmp` 宣言
- `strdup`, `strndup` 宣言

### Step 17.4: stdarg.h スタブ
- `va_list`, `va_start`, `va_arg`, `va_end`, `va_copy` マクロ定義
- 現在のビルトイン実装との連携

### Step 17.5: errno.h / limits.h / assert.h スタブ
- `errno` 宣言
- `INT_MAX`, `INT_MIN`, `LONG_MAX`, `CHAR_BIT` 等の定数
- `assert` マクロ

### Step 17.6: sys/ ヘッダ群スタブ (POSIX)
- `sys/types.h`: `pid_t`, `uid_t`, `gid_t`, `off_t`, `mode_t`
- `sys/stat.h`: `struct stat`, `stat()`, `fstat()`
- `unistd.h`: `read`, `write`, `close`, `fork`, `exec*`
- `fcntl.h`: `open`, `O_RDONLY` 等

---

## Phase 18: GCC拡張とビルトイン (8 steps)

### Step 18.1: __attribute__ のセマンティック対応
- `__attribute__((packed))` — パディング除去
- `__attribute__((aligned(N)))` — アライメント指定
- `__attribute__((noreturn))` — 戻り値なし関数
- `__attribute__((unused))` — 未使用警告抑制
- `__attribute__((format(printf, N, M)))` — 書式チェック（無視でOK）

### Step 18.2: 文式 (Statement Expressions)
- `({ stmt; stmt; expr; })` — ブロック内最後の式が値
- GNUマクロで多用: `#define MAX(a,b) ({ typeof(a) _a=(a); typeof(b) _b=(b); _a>_b?_a:_b; })`

### Step 18.3: __builtin 関数の拡張
- `__builtin_expect(expr, val)` — 既にサポート済み
- `__builtin_clz`, `__builtin_ctz` — ビット操作
- `__builtin_bswap16/32/64` — バイトスワップ
- `__builtin_popcount` — ポピュレーションカウント
- `__builtin_trap` — トラップ命令
- `__builtin_choose_expr` — コンパイル時条件分岐

### Step 18.4: インラインアセンブリ（スキップ/最小対応）
- `asm()` / `__asm__()` / `__asm__ volatile()` — パースしてスキップ
- PostgreSQLのスピンロック実装で使用されるが、代替実装も可能

### Step 18.5: 計算goto (Computed Goto)
- `void *label_ptr = &&label; goto *label_ptr;`
- PostgreSQLのインタプリタディスパッチで使用

### Step 18.6: __extension__ キーワードの完全対応
- GCC拡張使用時の警告抑制
- `__extension__ typedef unsigned long long uint64_t;`

### Step 18.7: _Thread_local / __thread
- スレッドローカルストレージ変数
- `.tbss` / `.tdata` セクション、`%fs` セグメントレジスタ

### Step 18.8: __builtin_types_compatible_p と型関連ビルトイン
- 型互換性チェック（既に部分実装済み）
- `__builtin_classify_type`

---

## Phase 19: 高度なコード生成 (6 steps)

### Step 19.1: 構造体の値渡しと値返し（ABI完全準拠）
- System V AMD64 ABI に従った分類: INTEGER, SSE, MEMORY
- 小さな構造体（≤16バイト）はレジスタ渡し
- 大きな構造体はスタック経由（hidden pointer）

### Step 19.2: 浮動小数点演算の完全対応
- `float` / `double` 四則演算
- XMMレジスタの活用
- float ↔ double ↔ int 変換
- 関数引数としてのfloat/double（XMMレジスタ渡し）

### Step 19.3: volatile変数のセマンティクス
- メモリアクセスの最適化防止
- PostgreSQL のシグナルハンドラ関連で使用

### Step 19.4: 可変長配列 (VLA)
- `int a[n];` — 実行時サイズの配列
- `alloca` 相当のスタック確保

### Step 19.5: 複合リテラルの完全対応
- `(struct Point){.x = 1, .y = 2}` — 一時変数として生成
- 関数引数として直接使用可能

### Step 19.6: ビットフィールドのABI準拠レイアウト
- ビットフィールドのパッキング規則
- unsigned/signed ビットフィールド
- ビットフィールド間のパディング

---

## Phase 20: PostgreSQL統合テスト (8 steps)

### Step 20.1: PostgreSQLビルドシステム連携
- Makefileベースのビルドプロセスとの連携
- `CC=our_compiler` での差し替えコンパイル

### Step 20.2: pg_config.h の処理
- PostgreSQLのconfigure生成ヘッダ
- プラットフォーム固有の定義

### Step 20.3: c.h (PostgreSQLの基盤ヘッダ) のコンパイル
- PostgreSQL全体で使用される基本型・マクロの集合
- ここが通れば多くのソースファイルがコンパイル可能に

### Step 20.4: nodes/ ディレクトリのコンパイル
- Node, List, Value 等の基本データ構造
- PostgreSQLのASTノード定義

### Step 20.5: utils/ の基本ユーティリティのコンパイル
- メモリ管理、文字列操作、エラー処理

### Step 20.6: parser/ のコンパイル
- SQL パーサー (bison 生成コード含む)

### Step 20.7: executor/ のコンパイル
- クエリ実行エンジン

### Step 20.8: 統合テスト — PostgreSQL起動テスト
- initdb + pg_ctl start が成功すること
- 基本的なSQLクエリが実行できること

---

## 合計: 136+ steps across 20 phases

## x86-64 コード生成の重要な規約

### System V AMD64 ABI
- 引数レジスタ: `%rdi`, `%rsi`, `%rdx`, `%rcx`, `%r8`, `%r9`
- 戻り値: `%rax`
- callee-saved: `%rbx`, `%rbp`, `%r12`-`%r15`
- caller-saved: `%rax`, `%rcx`, `%rdx`, `%rsi`, `%rdi`, `%r8`-`%r11`
- スタックは16バイトアライメント (call命令実行時)

### 関数プロローグ/エピローグ
```asm
func:
  push %rbp
  mov %rsp, %rbp
  sub $N, %rsp       # local variables (16-byte aligned)
  # ... body ...
  mov %rbp, %rsp
  pop %rbp
  ret
```

### スタックマシン方式のコード生成パターン
```
gen_expr(node):
  if node is Num:
    emit("mov ${}, %rax", node.val)
  if node is BinOp(+):
    gen_expr(node.lhs)
    emit("push %rax")
    gen_expr(node.rhs)
    emit("pop %rdi")
    emit("add %rdi, %rax")
```

## Critical Files
- `src/main.rs` — CLI entry point
- `src/token.rs` — Token definitions (new)
- `src/lexer.rs` — Tokenizer (new)
- `src/ast.rs` — AST nodes (new, Step 1.4)
- `src/parser.rs` — Recursive descent parser (new, Step 1.4)
- `src/codegen.rs` — x86-64 code generator (new, Step 1.4)
- `src/types.rs` — Type system (new, Phase 4)
- `src/sema.rs` — Semantic analysis (new, Phase 4)
- `src/error.rs` — Error reporting (new, Step 1.7)
- `src/preprocess.rs` — Preprocessor (new, Phase 10)
- `tests/test.sh` — Integration test runner (new, Step 1.1)

## Verification

各ステップ完了後:
1. `cargo build` でコンパイル成功を確認
2. `bash tests/test.sh` で全統合テストがパスすることを確認
3. `cargo test` でユニットテストがパスすることを確認

## 実装の進め方

1ステップずつ進める。各ステップで:
1. コードを実装
2. テストを追加・実行
3. 全テストパスを確認
4. 次のステップへ
