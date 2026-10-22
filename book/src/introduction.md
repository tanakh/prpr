# Introduction

prpr is a parser generator for Rust.
You describe a language with a [Parsing Expression Grammar](https://en.wikipedia.org/wiki/Parsing_expression_grammar) (PEG), add Rust code that builds values from what is matched, and prpr turns the grammar into Rust functions that parse text or bytes.

```rust
prpr::grammar! {
    pub list: Vec<i64> = "[" xs:(number, ",")* "]" { xs }

    #[label = "number"]
    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

fn main() {
    assert_eq!(list("[1, 2, 3]"), Ok(vec![1, 2, 3]));
    assert_eq!(
        list("[1, 2,]").unwrap_err().to_string(),
        "expected number, found `]`"
    );
}
```

The grammar above defines two rules.
`list` matches a `[`, numbers separated by commas, and a `]`, and its value is the vector of the numbers.
`number` matches digits and parses them.
`grammar!` generates a function for each rule, such as `list`, which parses a string and returns the value or an error.

## What prpr offers

* **Grammars in Rust or in files.**
  A grammar can be written inline with `grammar!`, or in a file, which `include_grammar!` includes or the `prpr` command compiles into a Rust module.
  Either way, mistakes in a grammar are reported at the right place in the grammar, and with `grammar!`, so are type errors in the Rust code inside it.
* **No separate lexer.**
  Whitespace is skipped between tokens automatically, `atom` rules describe tokens, and tokens must be delimited, so `ifx` is not the keyword `if` followed by `x`.
* **Useful error messages.**
  Errors say what was expected where the parser got the farthest, labels name things the way users think of them, and errors render like the messages of rustc.
* **Memoization where it pays.**
  prpr is a Packrat parser generator: rules marked with `#[memo]` remember their results, which keeps backtracking linear.
  Other rules do not pay for it.
* **Zero-copy values.**
  Values may borrow from the input, e.g. `&'i str`.
* **Text and binary input.**
  Grammars parse `&str` or `&[u8]`.
* **Composable grammars.**
  A grammar can use the rules of another grammar.
* **Speed.**
  On JSON documents, prpr is faster than the other parser generators and parser combinator libraries it was compared with; see the [benchmarks](https://github.com/tanakh/prpr/tree/master/benchmarks).

## How to read this book

[Getting started](getting-started/installation.md) walks through two complete parsers: a calculator and a JSON parser.
The [Guide](guide/rules.md) then explains each part of prpr in depth: rules, expressions, values, whitespace, errors, memoization, binary input and composition.
[Generating code](codegen/api.md) describes the generated code and the ways to generate it: from grammar files, in build scripts, and in proc-macros.
The [Cookbook](cookbook/keywords.md) shows how to handle common tasks, such as keywords, operator precedence, comments and syntax trees with spans.
The [Reference](reference/syntax.md) summarizes the syntax of grammars and the errors prpr reports.
The examples in this book are tested, so they work as they are written.
