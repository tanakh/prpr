# Installation

prpr requires Rust 1.96 or later and works with every edition.

Add it to the dependencies in `Cargo.toml`:

```toml
[dependencies]
prpr = "0.1"
```

> **Note:** prpr is not on crates.io yet.
> Until it is, use a git dependency instead:
> `prpr = { git = "https://github.com/tanakh/prpr" }`.

The `prpr` crate provides the `grammar!` macro, which generates a parser from a grammar written in Rust code, and the runtime that generated parsers use.
That is all you need to follow the rest of Getting started.

To generate parsers from grammar files instead, install the `prpr` command from the `prpr-cli` crate:

```sh
cargo install prpr-cli
# until it is published:
cargo install --git https://github.com/tanakh/prpr prpr-cli
```

Generated files then depend on the `prpr-runtime` crate, or on nothing at all if the runtime is included in them.
