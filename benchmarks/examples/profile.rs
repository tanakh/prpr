//! Parses a document repeatedly with one parser, for profiling:
//! `cargo run --release --example profile -- data/canada.json prpr 50`.
//!
//! The parsers are `prpr`, `peg`, `pest`, `nom` and `winnow` for JSON, and `prpr-graphql`,
//! `graphql-parser`, `async-graphql-parser`, `apollo-parser` and `cynic-parser` for GraphQL
//! schemas.

use prpr_benchmarks::*;
use std::hint::black_box;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, path, parser, rest @ ..] = args.as_slice() else {
        panic!("usage: profile <FILE> <PARSER> [ITERATIONS]");
    };
    let text = std::fs::read_to_string(path).unwrap();
    let parse: fn(&str) = match parser.as_str() {
        "prpr" => |s| drop(black_box(prpr_json::parse(s))),
        "peg" => |s| drop(black_box(peg_json::parse(s))),
        "pest" => |s| drop(black_box(pest_json::parse(s))),
        "nom" => |s| drop(black_box(nom_json::parse(s))),
        "winnow" => |s| drop(black_box(winnow_json::parse(s))),
        "prpr-graphql" => |s| drop(black_box(prpr_graphql::parse_schema(s))),
        "graphql-parser" => |s| drop(black_box(graphql_parser::parse_schema::<&str>(s).unwrap())),
        "async-graphql-parser" => {
            |s| drop(black_box(async_graphql_parser::parse_schema(s).unwrap()))
        }
        "apollo-parser" => |s| drop(black_box(apollo_parser::Parser::new(s).parse())),
        "cynic-parser" => |s| {
            drop(black_box(
                cynic_parser::parse_type_system_document(s).unwrap(),
            ))
        },
        _ => panic!("unknown parser `{parser}`"),
    };
    let iterations: usize = rest.first().map_or(50, |n| n.parse().unwrap());
    for _ in 0..iterations {
        parse(&text);
    }
}
