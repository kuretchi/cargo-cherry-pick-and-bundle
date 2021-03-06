[日本語](./README.ja.md)

*This repository has been archived.*
*I recommend [cargo-simple-bundler](https://github.com/kuretchi/cargo-simple-bundler).*

# cargo-cherry-pick-and-bundle

Packs only necessary modules of the crate into a single file semi-automatically.
Intended to use for online judges which accept only single file submissions.

## Install

```sh
cargo install --git https://github.com/kuretchi/cargo-cherry-pick-and-bundle
```

## Usage

In a package root directory:

```sh
cargo cherry-pick-and-bundle >output.rs
```

The command reads and parses source files recursively from the root module file,
and each time it encounters `mod` or `use`, it will ask you whether it is necessary or not.
Finally, the command will create a single inline module block that contains
only necessary parts, perform the following processing, and then print it to stdout.

* Removing modules with the `#[cfg(test)]` attribute
* Removing documentation comments
* Replacing the keyword `crate` in paths with `super`s

## Related tools

* [shino16/cargo-auto-bundle](https://github.com/shino16/cargo-auto-bundle)
* [qryxip/cargo-equip](https://github.com/qryxip/cargo-equip)
* [hatoo/cargo-snippet](https://github.com/hatoo/cargo-snippet)

## License

[MIT License](./LICENSE-MIT) or [Apache License 2.0](./LICENSE-APACHE)
