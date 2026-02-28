# Step 10.1: コメント

## 概要

C言語の2種類のコメントを実装する。

- **行コメント**: `// ...` — 行末まで無視
- **ブロックコメント**: `/* ... */` — 複数行にまたがれる

## 実装

レクサーのメインループで、空白スキップの直後にコメント検出を追加する。コメントは識別子やキーワードのパースよりも前にチェックする必要がある（`/` がスラッシュトークンとして消費される前に判定）。

### 行コメント (`//`)

```rust
if ch == '/' && self.peek_next() == Some('/') {
    while self.pos < self.input.len() && self.input[self.pos] != b'\n' {
        self.pos += 1;
    }
    continue;
}
```

改行文字に達するまでスキップ。改行自体は次のイテレーションで空白として処理される。

### ブロックコメント (`/* ... */`)

```rust
if ch == '/' && self.peek_next() == Some('*') {
    self.pos += 2;  // skip /*
    while self.pos + 1 < self.input.len() {
        if self.input[self.pos] == b'*' && self.input[self.pos + 1] == b'/' {
            self.pos += 2;  // skip */
            break;
        }
        self.pos += 1;
    }
    continue;
}
```

`*/` が見つかるまでスキップ。ネストには対応しない（C標準の仕様通り）。

### 判定順序

コメントの判定は以下の順序で行う：

1. 空白スキップ
2. **行コメント `//`** ← 新規
3. **ブロックコメント `/* */`** ← 新規
4. 識別子・キーワード
5. 文字リテラル
6. 文字列リテラル
7. 数値
8. 2文字トークン (`/=` など)
9. 1文字トークン (`/` など)

`/=` より先に `//` と `/*` を判定するため、トークン化の優先順位は正しく維持される。

## テストケース

```bash
# line comment
assert 42 'int main() { return 42; // comment\n}'
assert 10 'int main() { int a = 10; // set a\nreturn a; }'

# block comment
assert 3 'int main() { /* comment */ return 3; }'
assert 5 'int main() { int a = 5; /* set a */ return a; }'
```
