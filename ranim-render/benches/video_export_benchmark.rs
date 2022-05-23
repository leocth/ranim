use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ranim_render::{output, args::{Args, Quality}};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("output high", |b| b.iter(|| output(black_box(Args {
        preview: false,
        quality: Quality::High,
        output_file: PathBuf::from("media/output-benchmark.mp4"),
        single_frame: false,
        no_output: true, 
    }))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);