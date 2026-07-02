//! Point-code micro-benchmarks: parse (structured + decimal), components, and
//! `to_string`, for ITU and ANSI.
//!
//! Run with `cargo bench`. Numbers feed the README "Performance" table.
//!
//! mtp3 is a types/SAP crate, not a wire codec — the hot path a consumer (an STP
//! routing table, config load) hits is point-code parsing and formatting, so
//! that is what these measure. All fixtures come from the public API.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use mtp3::{PointCode, Variant};

fn bench_pointcode(c: &mut Criterion) {
    let itu = PointCode::from_components([2, 1, 3], Variant::Itu).unwrap();
    let ansi = PointCode::from_components([1, 1, 5], Variant::Ansi).unwrap();
    let itu_str = itu.to_string();
    let ansi_str = ansi.to_string();
    let itu_decimal = itu.value().to_string();
    let ansi_decimal = ansi.value().to_string();

    let mut g = c.benchmark_group("pointcode");
    g.throughput(Throughput::Elements(1));

    // parse — structured `a-b-c` form.
    g.bench_function("itu/parse_structured", |b| {
        b.iter(|| PointCode::parse(black_box(&itu_str), Variant::Itu).unwrap())
    });
    g.bench_function("ansi/parse_structured", |b| {
        b.iter(|| PointCode::parse(black_box(&ansi_str), Variant::Ansi).unwrap())
    });

    // parse — plain decimal form.
    g.bench_function("itu/parse_decimal", |b| {
        b.iter(|| PointCode::parse(black_box(&itu_decimal), Variant::Itu).unwrap())
    });
    g.bench_function("ansi/parse_decimal", |b| {
        b.iter(|| PointCode::parse(black_box(&ansi_decimal), Variant::Ansi).unwrap())
    });

    // components — decompose the value into structured parts.
    g.bench_function("itu/components", |b| b.iter(|| black_box(itu).components()));
    g.bench_function("ansi/components", |b| {
        b.iter(|| black_box(ansi).components())
    });

    // to_string — render back to `a-b-c`.
    g.bench_function("itu/to_string", |b| b.iter(|| black_box(itu).to_string()));
    g.bench_function("ansi/to_string", |b| b.iter(|| black_box(ansi).to_string()));

    g.finish();
}

criterion_group!(benches, bench_pointcode);
criterion_main!(benches);
