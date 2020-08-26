[English](./README.md)

# cargo-cherry-pick-and-bundle

クレートの必要なモジュールだけを一つのファイルにまとめる作業を半自動化します．
単一ファイルでの提出のみ受け付けるオンラインジャッジのために使用されることを想定しています．

## インストール

```
cargo install --git https://github.com/kuretchi/cargo-cherry-pick-and-bundle
```

## 使い方

パッケージのルートディレクトリ下で：

```
cargo cherry-pick-and-bundle >output.rs
```

ルートモジュールのファイルから再帰的にソースファイルを読み込み解析し，`mod` や `use` を見つけるたびに，それが必要かどうか質問します．
最終的に，必要な部分のみを一つのインラインモジュールにまとめ，以下の処理がされた状態で標準出力に出力します．

* `#[cfg(test)]` 属性が付いたモジュールの削除
* ドキュメンテーションコメントの削除
* パス中の `crate` を `super` で置き換える

## ライセンス

[MIT License](./LICENSE-MIT) or [Apache License 2.0](./LICENSE-APACHE)
