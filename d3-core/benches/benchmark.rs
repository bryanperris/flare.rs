use criterion::{black_box, criterion_group, criterion_main, Criterion};
use d3_core::math::vector::Vector;
use d3_core::math::vector_utils;

fn benchmark_magnitude(c: &mut Criterion) {
    let vector = Vector { x: 3.0, y: 4.0, z: 5.0 };

    c.bench_function("magnitude", |b| {
        b.iter(|| Vector::magnitude(&black_box(vector)))
    });
}

criterion_group!(benches, benchmark_magnitude);
criterion_main!(benches);