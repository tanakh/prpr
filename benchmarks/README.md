# Benchmarks

JSON parsers written with prpr, [rust-peg](https://crates.io/crates/peg), [pest](https://crates.io/crates/pest), [nom](https://crates.io/crates/nom) and [winnow](https://crates.io/crates/winnow), all building the same syntax tree, compared with [serde_json](https://crates.io/crates/serde_json).

The parsers are written in the idiomatic style of each library.
String literals are recognized by the parsers and decoded by a shared function, so that the comparison is about parsing.

## Running

The documents come from [json-benchmark](https://github.com/serde-rs/json-benchmark):

```sh
mkdir -p data
for f in canada citm_catalog twitter; do
    curl -sSfL -o data/$f.json https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/$f.json
done
cargo test --release   # checks that all parsers agree
cargo bench
```

`PRPR_BENCH_DATA` overrides the directory of the documents.

## Results

Mean time to parse each document, measured with criterion on 2026-10-21 (rustc 1.99.0, x86-64 Linux on WSL2).

| document | prpr | peg | pest | nom | winnow | serde_json |
|---|---:|---:|---:|---:|---:|---:|
| canada.json (2.2 MB) | 8.01 ms | 8.86 ms | 35.67 ms | 12.75 ms | 14.73 ms | 4.64 ms |
| citm_catalog.json (1.7 MB) | 2.37 ms | 2.95 ms | 28.63 ms | 4.69 ms | 6.05 ms | 2.69 ms |
| twitter.json (0.6 MB) | 1.42 ms | 1.62 ms | 14.06 ms | 2.24 ms | 3.00 ms | 1.29 ms |

serde_json builds its own `Value`, whose objects are `BTreeMap`s, so it does somewhat different work.
