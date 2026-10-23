# Benchmarks

Parsers written with prpr, compared with other libraries on JSON and GraphQL.

## JSON

JSON parsers written with prpr, [rust-peg](https://crates.io/crates/peg), [pest](https://crates.io/crates/pest), [nom](https://crates.io/crates/nom) and [winnow](https://crates.io/crates/winnow), all building the same syntax tree, compared with [serde_json](https://crates.io/crates/serde_json).

The parsers are written in the idiomatic style of each library.
String literals are recognized by the parsers and decoded by a shared function, so that the comparison is about parsing.

### Running

The documents come from [json-benchmark](https://github.com/serde-rs/json-benchmark):

```sh
mkdir -p data
for f in canada citm_catalog twitter; do
    curl -sSfL -o data/$f.json https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/$f.json
done
cargo test --release   # checks that all parsers agree
cargo bench --bench json
```

`PRPR_BENCH_DATA` overrides the directory of the documents.

### Results

Mean time to parse each document, measured with criterion on 2026-10-21 (rustc 1.99.0, x86-64 Linux on WSL2).

| document | prpr | peg | pest | nom | winnow | serde_json |
|---|---:|---:|---:|---:|---:|---:|
| canada.json (2.2 MB) | 8.01 ms | 8.86 ms | 35.67 ms | 12.75 ms | 14.73 ms | 4.64 ms |
| citm_catalog.json (1.7 MB) | 2.37 ms | 2.95 ms | 28.63 ms | 4.69 ms | 6.05 ms | 2.69 ms |
| twitter.json (0.6 MB) | 1.42 ms | 1.62 ms | 14.06 ms | 2.24 ms | 3.00 ms | 1.29 ms |

serde_json builds its own `Value`, whose objects are `BTreeMap`s, so it does somewhat different work.

## GraphQL

A GraphQL parser written with prpr, [graphql.peg](src/graphql.peg), compared with the GraphQL parsers of other crates, which are written in different ways:

* [graphql-parser](https://crates.io/crates/graphql-parser): with the parser combinators of [combine](https://crates.io/crates/combine).
* [async-graphql-parser](https://crates.io/crates/async-graphql-parser): with [pest](https://crates.io/crates/pest), another PEG parser generator.
* [apollo-parser](https://crates.io/crates/apollo-parser): by hand.
* [cynic-parser](https://crates.io/crates/cynic-parser): with [LALRPOP](https://crates.io/crates/lalrpop), an LR(1) parser generator, and the lexer generator [logos](https://crates.io/crates/logos).

The grammar follows the [GraphQL specification](https://spec.graphql.org/October2021/) (October 2021), and reports errors such as these:

```text
error: expected one of `)` or name, found `{`
 --> query.graphql:1:20
  |
1 | query { user(id: 1 { name } }
  |                    ^

error: expected a string that ends on the same line, found `"`
 --> query.graphql:1:8
  |
1 | { a(s: "unterminated) }
  |        ^
```

Each parser builds its own syntax tree, so they do different work:

* prpr's tree, in [graphql.rs](src/graphql.rs), borrows names from the input, decodes strings, and has the spans of definitions, fields, selections and directives.
* graphql-parser's tree, with `&str` names, borrows names, decodes strings, and has the lines and columns of definitions, fields, selections and directives.
* async-graphql-parser's tree allocates names, decodes strings, and has the lines and columns of its nodes.
* apollo-parser builds a lossless syntax tree, with whitespace and comments.
* cynic-parser stores the document in arenas, with spans, and keeps strings as written, to be decoded when they are read.

[tests/graphql.rs](tests/graphql.rs) checks that the parsers agree on the definitions of the documents, their kinds, names, numbers of members and descriptions, as far as each parser supports the specification:
graphql-parser supports neither descriptions of schemas, schema extensions nor directives of variables, and async-graphql-parser supports neither descriptions of schemas nor directives of variables after default values, and leaves `\"""` escaped in block strings.

### Running

The document is the public schema of [GitHub's GraphQL API](https://docs.github.com/en/graphql):

```sh
mkdir -p data
curl -sSfL -o data/github.graphql https://docs.github.com/public/fpt/schema.docs.graphql
cargo test --release   # checks that all parsers agree
cargo bench --bench graphql
```

### Results

Mean time to parse the schema, measured with criterion on 2026-10-23 (rustc 1.99.0, x86-64 Linux on WSL2).

| document | prpr | graphql-parser | async-graphql-parser | apollo-parser | cynic-parser |
|---|---:|---:|---:|---:|---:|
| github.graphql (1.6 MB) | 5.49 ms | 10.95 ms | 57.27 ms | 13.35 ms | 8.14 ms |

## Profiling

`examples/profile.rs` parses a document repeatedly with one parser, for profilers such as `perf`:

```sh
cargo run --release --example profile -- data/canada.json prpr 50
cargo run --release --example profile -- data/github.graphql prpr-graphql 50
```
