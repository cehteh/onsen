// testing rust allocator vs onsen without doing any work on the data
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// TODO: rename as baseline.rs again

use onsen;
use onsen::PoolApi;

// just simple allocate and instantly drop test
fn rust_box_drop() {
    black_box(Box::new(0u64));
}

fn onsen_bbox_drop(pool: &onsen::Pool<onsen::ThinPoolEntry<u64>>) {
    let a = black_box(onsen::BasicBox::new(0u64, pool));
    black_box(onsen::BasicBox::drop(a, pool));
}

fn onsen_bbox_drop_unchecked(pool: &onsen::Pool<onsen::ThinPoolEntry<u64>>) {
    let a = black_box(onsen::BasicBox::new(0u64, pool));
    unsafe {
        black_box(onsen::BasicBox::drop_unchecked(a, pool));
    }
}

fn onsen_rcpool_box_drop(pool: &onsen::RcPool<onsen::FatPoolEntry<u64>>) {
    black_box(onsen::Box::new(0u64, pool));
}

fn onsen_arcpool_box_drop(pool: &onsen::ArcPool<onsen::FatPoolEntry<u64>>) {
    black_box(onsen::Box::new(0u64, pool));
}

//TODO: add TcPool

/*
allocate many elements into a preallocated Vec and drop them all at the end
*/
fn rust_box_many(howmany: usize) {
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(Box::new(0u64));
    }
}

fn onsen_bbox_leak<'a>(howmany: usize, pool: &'a onsen::Pool<onsen::ThinPoolEntry<u64>>) {
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(onsen::BasicBox::new(0u64, pool));
    }
}

fn onsen_bbox_many<'a>(howmany: usize, pool: &'a onsen::Pool<onsen::ThinPoolEntry<u64>>) {
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(onsen::BasicBox::new(0u64, pool));
    }
    while let Some(element) = keep.pop() {
        onsen::BasicBox::drop(element, pool);
    }
}

fn onsen_bbox_many_unchecked<'a>(howmany: usize, pool: &'a onsen::Pool<onsen::ThinPoolEntry<u64>>) {
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(onsen::BasicBox::new(0u64, pool));
    }
    while let Some(element) = keep.pop() {
        unsafe {
            onsen::BasicBox::drop_unchecked(element, pool);
        }
    }
}

fn onsen_rcpool_box_many<'a>(howmany: usize, pool: &'a onsen::RcPool<onsen::FatPoolEntry<u64>>) {
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(onsen::Box::new(0u64, pool));
    }
}

fn onsen_arcpool_box_many<'a>(howmany: usize, pool: &'a onsen::ArcPool<onsen::FatPoolEntry<u64>>) {
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(onsen::Box::new(0u64, pool));
    }
}

// allocate many elements into a preallocated Vec and drop random elements on the way

#[inline(always)]
fn fast_prng(state: &mut u32) -> u32 {
    let rand = *state;
    *state = rand << 1 ^ ((rand >> 30) & 1) ^ ((rand >> 2) & 1);
    return rand;
}

fn rust_box_many_with_drop(howmany: usize, drop_percent: u32) {
    let mut state = 0xbabeface_u32;
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(Some(Box::new(0u64)));
        if fast_prng(&mut state) % 100 < drop_percent {
            let pos = fast_prng(&mut state) as usize % keep.len();
            if let Some(_) = keep[pos].take() {
                keep[pos] = None;
            }
        }
    }
}

fn onsen_bbox_many_with_drop<'a>(
    howmany: usize,
    drop_percent: u32,
    pool: &'a onsen::Pool<onsen::ThinPoolEntry<u64>>,
) {
    let mut state = 0xbabeface_u32;
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(Some(onsen::BasicBox::new(0u64, pool)));
        if fast_prng(&mut state) % 100 < drop_percent {
            let pos = fast_prng(&mut state) as usize % keep.len();
            if let Some(bbox) = keep[pos].take() {
                onsen::BasicBox::drop(bbox, pool);
            }
        }
    }
}

fn onsen_bbox_many_with_drop_unchecked<'a>(
    howmany: usize,
    drop_percent: u32,
    pool: &'a onsen::Pool<onsen::ThinPoolEntry<u64>>,
) {
    let mut state = 0xbabeface_u32;
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(Some(onsen::BasicBox::new(0u64, pool)));
        if fast_prng(&mut state) % 100 < drop_percent {
            let pos = fast_prng(&mut state) as usize % keep.len();
            if let Some(bbox) = keep[pos].take() {
                unsafe {
                    onsen::BasicBox::drop_unchecked(bbox, pool);
                }
            }
        }
    }
}

fn onsen_rcpool_box_many_with_drop<'a>(
    howmany: usize,
    drop_percent: u32,
    pool: &'a onsen::RcPool<onsen::FatPoolEntry<u64>>,
) {
    let mut state = 0xbabeface_u32;
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(Some(onsen::Box::new(0u64, pool)));
        if fast_prng(&mut state) % 100 < drop_percent {
            let pos = fast_prng(&mut state) as usize % keep.len();
            if let Some(_) = keep[pos].take() {
                keep[pos] = None;
            }
        }
    }
}

fn onsen_arcpool_box_many_with_drop<'a>(
    howmany: usize,
    drop_percent: u32,
    pool: &'a onsen::ArcPool<onsen::FatPoolEntry<u64>>,
) {
    let mut state = 0xbabeface_u32;
    let mut keep = Vec::with_capacity(howmany);
    for _ in 0..howmany {
        keep.push(Some(onsen::Box::new(0u64, pool)));
        if fast_prng(&mut state) % 100 < drop_percent {
            let pos = fast_prng(&mut state) as usize % keep.len();
            if let Some(_) = keep[pos].take() {
                keep[pos] = None;
            }
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut baseline = c.benchmark_group("baseline drop");

    baseline.bench_function("std::boxed::Box drop", |b| b.iter(|| rust_box_drop()));

    baseline.bench_function("onsen::BasicBox drop", {
        |b| {
            let pool = onsen::Pool::new();
            pool.with_min_entries(1000);
            b.iter(|| onsen_bbox_drop(&pool));
            drop(pool);
        }
    });

    baseline.bench_function("onsen::BasicBox drop unchecked", {
        |b| {
            let pool = onsen::Pool::new();
            pool.with_min_entries(1000);
            b.iter(|| onsen_bbox_drop_unchecked(&pool));
            drop(pool);
        }
    });

    baseline.bench_function("onsen::Box|RcPool drop", {
        |b| {
            let pool = onsen::RcPool::new();
            pool.with_min_entries(1000);
            b.iter(|| onsen_rcpool_box_drop(&pool));
            drop(pool);
        }
    });

    baseline.bench_function("onsen::Box|ArcPool drop", {
        |b| {
            let pool = onsen::ArcPool::new();
            pool.with_min_entries(1000);
            b.iter(|| onsen_arcpool_box_drop(&pool));
            drop(pool);
        }
    });

    drop(baseline);
    let mut baseline = c.benchmark_group("baseline keep");

    for size in [1000, 5000, 10000, 30000, 50000, 75000, 100000].iter() {
        baseline.throughput(Throughput::Elements(*size as u64));

        baseline.bench_with_input(BenchmarkId::new("std::boxed::box", size), &size, {
            |b, &s| {
                b.iter(|| {
                    rust_box_many(*s);
                })
            }
        });

        baseline.bench_with_input(BenchmarkId::new("onsen::BasicBox leak", size), &size, {
            move |b, &s| {
                let pool = onsen::Pool::new();
                pool.with_min_entries(1000);
                b.iter(|| onsen_bbox_leak(*s, &pool));
            }
        });

        baseline.bench_with_input(BenchmarkId::new("onsen::BasicBox", size), &size, {
            move |b, &s| {
                let pool = onsen::Pool::new();
                pool.with_min_entries(1000);
                b.iter(|| onsen_bbox_many(*s, &pool));
            }
        });

        baseline.bench_with_input(BenchmarkId::new("onsen::Box|RcPool", size), &size, {
            move |b, &s| {
                let pool = onsen::RcPool::new();
                pool.with_min_entries(1000);
                b.iter(|| onsen_rcpool_box_many(*s, &pool));
            }
        });

        baseline.bench_with_input(BenchmarkId::new("onsen::Box|ArcPool", size), &size, {
            move |b, &s| {
                let pool = onsen::ArcPool::new();
                pool.with_min_entries(1000);
                b.iter(|| onsen_arcpool_box_many(*s, &pool));
            }
        });
    }

    drop(baseline);
    let mut baseline = c.benchmark_group("baseline with 50 percent drop");

    for size in [1000, 5000, 10000, 30000, 50000, 75000, 100000].iter() {
        baseline.throughput(Throughput::Elements(*size as u64));

        baseline.bench_with_input(BenchmarkId::new("std::boxed::box", size), &size, {
            |b, &s| {
                b.iter(|| {
                    rust_box_many_with_drop(*s, 50);
                })
            }
        });

        baseline.bench_with_input(BenchmarkId::new("onsen::BasicBox", size), &size, {
            move |b, &s| {
                let pool = onsen::Pool::new();
                pool.with_min_entries(1000);
                b.iter(|| onsen_bbox_many_with_drop(*s, 50, &pool));
            }
        });

        baseline.bench_with_input(
            BenchmarkId::new("onsen::BasicBox unchecked", size),
            &size,
            {
                move |b, &s| {
                    let pool = onsen::Pool::new();
                    pool.with_min_entries(1000);
                    b.iter(|| onsen_bbox_many_with_drop_unchecked(*s, 50, &pool));
                }
            },
        );

        baseline.bench_with_input(BenchmarkId::new("onsen::Box|RcPool", size), &size, {
            move |b, &s| {
                let pool = onsen::RcPool::new();
                pool.with_min_entries(1000);
                b.iter(|| onsen_rcpool_box_many_with_drop(*s, 50, &pool));
            }
        });

        baseline.bench_with_input(BenchmarkId::new("onsen::Box|ArcPool", size), &size, {
            move |b, &s| {
                let pool = onsen::ArcPool::new();
                pool.with_min_entries(1000);
                b.iter(|| onsen_arcpool_box_many_with_drop(*s, 50, &pool));
            }
        });
    }

    drop(baseline);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
