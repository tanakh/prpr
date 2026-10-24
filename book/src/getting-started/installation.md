# Installation

prpr requires Rust 1.96 or later and works with every edition.

Add it to the dependencies in `Cargo.toml`:

```toml
[dependencies]
prpr = "0.1"
```

The `prpr` crate provides the `grammar!` macro, which generates a parser from a grammar written in Rust code, `include_grammar!`, which does the same from a grammar file, and the runtime that generated parsers use.
That is all you need to follow the rest of Getting started.

To generate Rust modules from grammar files ahead of time instead, install the `prpr` command from the `prpr-cli` crate (or use the `prpr-codegen` crate in a [build script](../codegen/build-scripts.md)):

```sh
cargo install prpr-cli
```

Generated files then depend on the `prpr-runtime` crate, or on nothing at all if the runtime is included in them.
