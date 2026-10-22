//! Parses a document repeatedly with one parser, for profiling:
//! `cargo run --release --example profile -- data/canada.json prpr 50`.

use prpr_benchmarks::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, path, parser, rest @ ..] = args.as_slice() else {
        panic!("usage: profile <FILE> <prpr|peg|pest|nom|winnow> [ITERATIONS]");
    };
    let text = std::fs::read_to_string(path).unwrap();
    let parse = match parser.as_str() {
        "prpr" => prpr_json::parse,
        "peg" => peg_json::parse,
        "pest" => pest_json::parse,
        "nom" => nom_json::parse,
        "winnow" => winnow_json::parse,
        _ => panic!("unknown parser `{parser}`"),
    };
    let iterations: usize = rest.first().map_or(50, |n| n.parse().unwrap());
    for _ in 0..iterations {
        std::hint::black_box(parse(&text));
    }
}
