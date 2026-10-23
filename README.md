# prpr - Packrat parser in Rust

[![crates.io](https://img.shields.io/crates/v/prpr.svg)](https://crates.io/crates/prpr)
[![docs.rs](https://img.shields.io/docsrs/prpr)](https://docs.rs/prpr)
[![CI](https://github.com/tanakh/prpr/actions/workflows/ci.yml/badge.svg)](https://github.com/tanakh/prpr/actions/workflows/ci.yml)
[![Book](https://img.shields.io/badge/book-online-blue.svg)](https://tanakh.github.io/prpr/)
![Rust 1.96+](https://img.shields.io/badge/rust-1.96%2B-orange.svg)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

This is a library that generates code for performing Packrat parsing on syntax defined using a [Parsing Expression Grammar (PEG)](https://en.wikipedia.org/wiki/Parsing_expression_grammar).

```rust
prpr::grammar! {
    pub expr: i64 = l:term rs:(("+" / "-") term)* {
        rs.into_iter().fold(l, |a, (op, b)| if op == "+" { a + b } else { a - b })
    }
    term: i64 = l:fact rs:(("*" / "/") fact)* {
        rs.into_iter().fold(l, |a, (op, b)| if op == "*" { a * b } else { a / b })
    }
    fact: i64 = number / "(" e:expr ")" { e }

    #[label = "number"]
    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

assert_eq!(expr("1 + 2 * (3 - 1)"), Ok(5));
assert_eq!(expr("1 +").unwrap_err().to_string(), "expected one of `(` or number, found end of input");
```

* Whitespace is skipped between the tokens of rules, but not inside `atom` rules, and tokens must be delimited (`123x` is not a number followed by `x`).
* Rules are only memoized with `#[memo]`.
* Errors report what was expected at the farthest position reached, and render like rustc's.
* Grammars can be written inline with `grammar!`, or in files, which `include_grammar!` includes or `prpr gen` (from `prpr-cli`) compiles into standalone Rust modules.
* Text (`&str`) and binary (`&[u8]`) input are supported.

## Documentation

* [The prpr Book](https://tanakh.github.io/prpr/) explains how to use prpr.
  Its source is in [book](https://github.com/tanakh/prpr/tree/master/book); build it with [mdBook](https://rust-lang.github.io/mdBook/): `mdbook build book`.
  Its examples are tested by `cargo test`.
* The [design note](https://github.com/tanakh/prpr/blob/master/docs/design.md) explains the design.
* The [benchmarks](https://github.com/tanakh/prpr/tree/master/benchmarks) compare prpr with other libraries.

## Crates

* `prpr`: the `grammar!`, `include_grammar!` and `define_proc_macro!` macros, and the runtime.
* `prpr-runtime`: the runtime alone, for generated files.
* `prpr-macros`: the `grammar!` and `include_grammar!` macros, used through `prpr`.
* `prpr-codegen`: the code generator, for `build.rs` scripts and tools.
* `prpr-cli`: the `prpr` command.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/tanakh/prpr/blob/master/LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](https://github.com/tanakh/prpr/blob/master/LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

As a special exception, the code that prpr generates, including the runtime that `prpr runtime` and `prpr_codegen::file::runtime_module` write and that `--inline-runtime` and `Runtime::Inline` include in generated modules, may be used, modified and distributed under terms of your choice, without the notices that these licenses require.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
