//! Parses JSON documents into the same syntax tree with each library.
//!
//! The documents are read from `$PRPR_BENCH_DATA` (default: `data/`): `canada.json`,
//! `citm_catalog.json` and `twitter.json` from <https://github.com/serde-rs/json-benchmark>.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use prpr_benchmarks::*;
use std::hint::black_box;

const FILES: &[&str] = &["canada", "citm_catalog", "twitter"];

fn bench(c: &mut Criterion) {
    let dir = std::env::var("PRPR_BENCH_DATA").unwrap_or_else(|_| "data".into());
    for name in FILES {
        let path = format!("{dir}/{name}.json");
        let Ok(text) = std::fs::read_to_string(&path) else {
            eprintln!("skipping {path}: not found");
            continue;
        };
        let mut group = c.benchmark_group(*name);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function("prpr", |b| b.iter(|| prpr_json::parse(black_box(&text))));
        group.bench_function("peg", |b| b.iter(|| peg_json::parse(black_box(&text))));
        group.bench_function("pest", |b| b.iter(|| pest_json::parse(black_box(&text))));
        group.bench_function("nom", |b| b.iter(|| nom_json::parse(black_box(&text))));
        group.bench_function("winnow", |b| {
            b.iter(|| winnow_json::parse(black_box(&text)))
        });
        group.bench_function("serde_json", |b| {
            b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(&text)).unwrap())
        });
        group.finish();
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
