# cargo-cherry-pick-and-bundle

クレートの必要な部分だけをモジュール単位で取り出し，コピー&ペーストできる状態の一つのモジュールにまとめる作業を半自動化します．

## インストール

```
$ cargo install --git https://github.com/kuretchi/cargo-cherry-pick-and-bundle
```

## 使い方

```
$ cargo cherry-pick-and-bundle >output.rs
```

ソースコード中の `mod` や `use` について，それぞれそれが必要かどうか質問されます．すべての質問に答えると，以下の処理がされた状態で，必要と答えた部分のみが一つの `mod` にまとめられ，標準出力に出力されます．

* `#[cfg(test)]` 属性が付いたモジュールの削除
* ドキュメンテーションコメントの削除
* パス中の `crate` を等価な個数の `super` で置き換える
