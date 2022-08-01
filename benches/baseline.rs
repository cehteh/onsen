// testing rust allocator vs onsen without doing any work on the data
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use onsen;

// just simple allocate and instantly drop test
fn rust_box_drop() {
    black_box(Box::new(0u64));
}

fn onsen_box_drop<const E: usize>(pool: &onsen::Pool<u64, E>) {
    black_box(pool.alloc_box(0u64));
}

// allocate many elements into a preallocated Vec and drop them all at the end
fn rust_box_many(howmany: usize) {
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(Box::new(0u64));
    }
}

fn onsen_box_many<'a, const E: usize>(howmany: usize, pool: &'a onsen::Pool<u64, E>) {
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(pool.alloc_box(0u64));
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut baseline = c.benchmark_group("baseline drop");

    baseline.bench_function("rust box drop", |b| b.iter(|| rust_box_drop()));

    baseline.bench_function("onsen box drop", {
        let pool = onsen::pool!(u64, PAGE);
        move |b| b.iter(|| onsen_box_drop(&pool))
    });

    drop(baseline);
    let mut baseline = c.benchmark_group("baseline keep");

    for size in [1000usize, 5000usize, 10000usize, 50000usize, 100000usize].iter() {
        baseline.throughput(Throughput::Elements(*size as u64));

        baseline.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                b.iter(|| {
                    rust_box_many(*s);
                })
            }
        });

        baseline.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            let pool = onsen::pool!(u64, PAGE);
            move |b, &s| {
                b.iter(|| {
                    onsen_box_many(*s, &pool);
                })
            }
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
