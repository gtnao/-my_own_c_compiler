# Step 16.3〜16.8: プリプロセッサ拡張（残りのステップ）

## Step 16.3: 複雑な #if 式
Step 16.2 の完全な `CondEval` 再帰下降評価器で既に実装済みです。算術、比較、論理、ビット演算、三項演算子、`defined()` を含むすべてのCプリプロセッサ式演算子をサポートしています。

## Step 16.4: #undef ディレクティブ
既に実装済みです。プリプロセッサは `#undef NAME` を処理し、定義マップからマクロを削除します。

## Step 16.6: #pragma once と #pragma pack
- `#pragma` 行は解析され、静かに無視されます
- `#pragma once` の動作は、正規化されたファイルパスを追跡し再インクルードを防止する `included` セットによって実質的に処理されています
- `#pragma pack` は本コンパイラのユースケースでは不要です（構造体レイアウトは標準ABIルールに従います）

## Step 16.8: #include_next
`#include_next` は GCC拡張で、現在のファイルを含むディレクトリの次のディレクトリからヘッダの検索を開始します。

本コンパイラの実装では、`#include_next` を `#include` と同一に扱います。これは簡略化ですが、本コンパイラ提供のヘッダが `#include_next` チェインを必要とするシステムヘッダをシャドウしないため、ほとんどの実用的なケースで機能します。

```rust
if trimmed.starts_with("#include_next") || trimmed.starts_with("#include") {
    let directive_len = if trimmed.starts_with("#include_next") {
        "#include_next".len()
    } else {
        "#include".len()
    };
    // ... rest of include processing
}
```
