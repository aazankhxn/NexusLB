use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nexuslb_network::{BufferPool, DEFAULT_BUFFER_SIZE};

fn bench_buffer_pool(c: &mut Criterion) {
    let pool = BufferPool::new(1024, DEFAULT_BUFFER_SIZE);

    let mut group = c.benchmark_group("memory_allocation_vs_pooling");

    group.bench_function("buffer_pool_acquire_and_drop", |b| {
        b.iter(|| {
            let buf = pool.acquire();
            black_box(buf.as_slice());
        })
    });

    group.bench_function("standard_heap_alloc_and_drop", |b| {
        b.iter(|| {
            let buf = vec![0u8; DEFAULT_BUFFER_SIZE];
            black_box(buf.as_slice());
        })
    });

    group.finish();
}

criterion_group!(benches, bench_buffer_pool);
criterion_main!(benches);
