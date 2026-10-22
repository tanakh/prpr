# prpr - Packrat parser in Rust

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
* Grammars can be written inline with `grammar!`, or in files compiled with `prpr gen` (from `prpr-cli`) into standalone Rust modules.
* Text (`&str`) and binary (`&[u8]`) input are supported.

See the [design note](docs/design.md) for the grammar language and the design, and [benchmarks](benchmarks/README.md) for how it compares with other libraries.

## Crates

* `prpr`: the `grammar!` macro and the runtime.
* `prpr-runtime`: the runtime alone, for generated files.
* `prpr-codegen`: the code generator, for `build.rs` scripts and tools.
* `prpr-cli`: the `prpr` command.

## License

MIT
