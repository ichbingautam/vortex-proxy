#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vortex_core::load_balancer::ewma::PeakEwma;

fn bench_peak_ewma_observe(c: &mut Criterion) {
    let ewma = PeakEwma::new(500.0, 0.5);

    // Simulate hot path: recording latencies
    c.bench_function("ewma_observe_latency", |b| {
        b.iter(|| {
            // We use black_box to prevent the compiler from optimizing the loop away
            ewma.observe_latency(black_box(15.5));
        })
    });
}

fn bench_peak_ewma_score(c: &mut Criterion) {
    let ewma = PeakEwma::new(500.0, 0.5);
    // prime it with some data
    ewma.observe_latency(20.0);
    let _guard = ewma.increment_active();

    // Simulate hot path: load balancer fetching the routing score
    c.bench_function("ewma_calculate_score", |b| {
        b.iter(|| {
            let score = ewma.calculate_score();
            black_box(score);
        })
    });
}

criterion_group!(benches, bench_peak_ewma_observe, bench_peak_ewma_score);
criterion_main!(benches);
