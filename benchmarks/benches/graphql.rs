//! Parses a GraphQL schema with prpr and with the GraphQL parsers of other crates, each building
//! its own syntax tree.
//!
//! The schema is read from `$PRPR_BENCH_DATA` (default: `data/`): `github.graphql`, the public
//! schema of GitHub's GraphQL API from <https://docs.github.com/public/fpt/schema.docs.graphql>.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use prpr_benchmarks::prpr_graphql;
use std::hint::black_box;

fn bench(c: &mut Criterion) {
    let dir = std::env::var("PRPR_BENCH_DATA").unwrap_or_else(|_| "data".into());
    let path = format!("{dir}/github.graphql");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("skipping {path}: not found");
        return;
    };
    let mut group = c.benchmark_group("github");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("prpr", |b| {
        b.iter(|| prpr_graphql::parse_schema(black_box(&text)))
    });
    group.bench_function("graphql-parser", |b| {
        b.iter(|| graphql_parser::parse_schema::<&str>(black_box(&text)).unwrap())
    });
    group.bench_function("async-graphql-parser", |b| {
        b.iter(|| async_graphql_parser::parse_schema(black_box(&text)).unwrap())
    });
    group.bench_function("apollo-parser", |b| {
        b.iter(|| apollo_parser::Parser::new(black_box(&text)).parse())
    });
    group.bench_function("cynic-parser", |b| {
        b.iter(|| cynic_parser::parse_type_system_document(black_box(&text)).unwrap())
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
