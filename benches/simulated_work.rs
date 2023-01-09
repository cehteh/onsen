// testing rust allocator vs onsen without doing any work on the data
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

use onsen;
use onsen::PoolApi;

const SMALL: usize = 3;
const MEDIUM: usize = 64;
const BIG: usize = 1000;

// The data we work on contains a primary value used for sorting and some payload which becomes mutated
pub struct Data<const N: usize> {
    primary: u32,
    payload: [u32; N],
}

impl<const N: usize> Data<N> {
    fn new(primary: u32) -> Self {
        Data {
            primary,
            payload: [primary; N],
        }
    }
}

trait DataHandle {
    fn primary(&self) -> &u32;
    fn primary_mut(&mut self) -> &mut u32;
    fn payload(&mut self) -> &mut [u32];
}

pub struct OwnedData<const N: usize>(Data<N>, usize);

impl<const N: usize> DataHandle for OwnedData<N> {
    fn primary(&self) -> &u32 {
        &self.0.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.0.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.0.payload
    }
}

// data in a rust box
pub struct BoxedData<const N: usize>(Box<Data<N>>);

impl<const N: usize> DataHandle for BoxedData<N> {
    fn primary(&self) -> &u32 {
        &self.0.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.0.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.0.payload
    }
}

// data in a onsen box
pub struct OnsenBoxedData<const N: usize>(onsen::Box<Data<N>, onsen::RcPool<Data<N>>>);

impl<const N: usize> DataHandle for OnsenBoxedData<N> {
    fn primary(&self) -> &u32 {
        &self.0.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.0.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.0.payload
    }
}

// data in a onsen basic box
pub struct OnsenBasicBoxedData<'a, const N: usize>(onsen::BasicBox<'a, Data<N>>);

impl<'a, const N: usize> DataHandle for OnsenBasicBoxedData<'a, N> {
    fn primary(&self) -> &u32 {
        &self.0.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.0.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.0.payload
    }
}

// Worker contains context and work we simulate
trait Worker<'a> {
    type Data: DataHandle;
    fn new() -> Self;
    fn new_element(&'a self, primary: u32) -> Option<Self::Data>;

    // a work simulator that does not free elements
    fn run_keep(&'a self, howmuch: usize) {
        let mut state = 0xbabeface_u32;
        let mut workspace: Vec<Option<Self::Data>> = Vec::with_capacity(howmuch);
        for _ in 0..howmuch {
            match fast_prng(&mut state) % 100 {
                _ if workspace.len() < 50 => {
                    // warmup for the first 50 entries
                    workspace.push(self.new_element(fast_prng(&mut state)));
                }
                0..=69 => {
                    //push new entry
                    workspace.push(self.new_element(fast_prng(&mut state)));
                }
                70..=74 => {
                    // mutate payload & primary within first 10%
                    let pos = fast_prng(&mut state) as usize % (workspace.len() / 10);
                    if let Some(value) = workspace[pos].as_mut() {
                        *value.primary_mut() = fast_prng(&mut state);
                        for n in 0..value.payload().len() {
                            value.payload()[n] = fast_prng(&mut state);
                        }
                    }
                }
                75..=79 => {
                    // sort the first 5% of the vec values increasing, none at the end
                    let pos = fast_prng(&mut state) as usize % (workspace.len() / 20);
                    workspace[0..pos].sort_unstable_by(|a, b| {
                        match (a, b) {
                            (Some(a), Some(b)) => a.primary().partial_cmp(&b.primary()).unwrap(),
                            _ => std::cmp::Ordering::Greater, // Contains a 'None'
                        }
                    });
                }
                80..=99 => {
                    // swap 2 entries, first from the first 10%, second from anywhere
                    let pos1 = fast_prng(&mut state) as usize % (workspace.len() / 10);
                    let pos2 = fast_prng(&mut state) as usize % (workspace.len());
                    workspace.swap(pos1, pos2);
                }
                _ => unreachable!(),
            }
        }
    }

    // a work simulator that sometimes frees elements
    fn run_drop(&'a self, howmuch: usize) {
        let mut state = 0xbabeface_u32;
        let mut workspace: Vec<Option<Self::Data>> = Vec::with_capacity(howmuch);
        for _ in 0..howmuch {
            match fast_prng(&mut state) % 100 {
                _ if workspace.len() < 50 => {
                    // warmup for the first 50 entries
                    workspace.push(self.new_element(fast_prng(&mut state)));
                }
                0..=39 => {
                    //push new entry
                    workspace.push(self.new_element(fast_prng(&mut state)));
                }
                40..=69 => {
                    // mutate payload & primary within first 10%
                    let pos = fast_prng(&mut state) as usize % (workspace.len() / 10);
                    if let Some(value) = workspace[pos].as_mut() {
                        *value.primary_mut() = fast_prng(&mut state);
                        for n in 0..value.payload().len() {
                            value.payload()[n] = fast_prng(&mut state);
                        }
                    }
                }
                70..=74 => {
                    // sort the first 5% of the vec values increasing, none at the end
                    let pos = fast_prng(&mut state) as usize % (workspace.len() / 20);
                    workspace[0..pos].sort_unstable_by(|a, b| {
                        match (a, b) {
                            (Some(a), Some(b)) => a.primary().partial_cmp(&b.primary()).unwrap(),
                            _ => std::cmp::Ordering::Greater, // Contains a 'None'
                        }
                    });
                }
                75..=89 => {
                    // swap 2 entries, first from the first 10%, second from anywhere
                    let pos1 = fast_prng(&mut state) as usize % (workspace.len() / 10);
                    let pos2 = fast_prng(&mut state) as usize % (workspace.len());
                    workspace.swap(pos1, pos2);
                }
                90..=99 => {
                    // drop random entry in first 20%
                    let pos = fast_prng(&mut state) as usize % (workspace.len() / 5);
                    workspace[pos] = None;
                }
                _ => unreachable!(),
            }
        }
    }
}

// Worker for owned data
#[repr(C)]
pub struct OwnedWorker<const N: usize>;

impl<const N: usize> Worker<'_> for OwnedWorker<N> {
    type Data = OwnedData<N>;
    fn new() -> Self {
        Self
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(OwnedData(Data::new(primary), primary as usize))
    }
}

// Worker for rust boxes
#[repr(C)]
pub struct BoxWorker<const N: usize>;

impl<const N: usize> Worker<'_> for BoxWorker<N> {
    type Data = BoxedData<N>;
    fn new() -> Self {
        Self
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(BoxedData(Box::new(Data::new(primary))))
    }
}

// Worker for onsen boxes
pub struct OnsenBoxWorker<const N: usize> {
    pool: onsen::RcPool<Data<N>>,
}

impl<'a, const N: usize> Worker<'a> for OnsenBoxWorker<N> {
    type Data = OnsenBoxedData<N>;
    fn new() -> Self {
        let pool = onsen::RcPool::new();
        pool.with_min_entries(1000);
        OnsenBoxWorker { pool }
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(OnsenBoxedData(onsen::Box::new(
            Data::new(primary),
            &self.pool,
        )))
    }
}

// Worker for leaking onsen basic boxes
pub struct OnsenBasicBoxLeakWorker<const N: usize> {
    pool: onsen::Pool<Data<N>>,
}

impl<'a, const N: usize> Worker<'a> for OnsenBasicBoxLeakWorker<N> {
    type Data = OnsenBasicBoxedData<'a, N>;
    fn new() -> Self {
        let pool = onsen::Pool::new();
        pool.with_min_entries(1000);
        OnsenBasicBoxLeakWorker { pool }
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(OnsenBasicBoxedData(onsen::BasicBox::new(
            Data::new(primary),
            &self.pool,
        )))
    }
}

#[inline(always)]
fn fast_prng(state: &mut u32) -> u32 {
    let rand = *state;
    *state = rand << 1 ^ ((rand >> 30) & 1) ^ ((rand >> 2) & 1);
    rand
}

fn criterion_benchmark(c: &mut Criterion) {
    let test_range = [
        1000, 10000, 25000, 50000, 75000, 100000, 125000, 150000, 175000, 200000,
    ];
    // Keep benchmarks
    let mut simulated_work = c.benchmark_group("simulated keep, small data");

    for size in test_range.iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = OwnedWorker::<SMALL>::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("std::boxed::Box", size), &size, {
            |b, &s| {
                let worker = BoxWorker::<SMALL>::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen::Box", size), &size, {
            |b, &s| {
                let worker = OnsenBoxWorker::<SMALL>::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(
            BenchmarkId::new("leaking onsen::BasicBox", size),
            &size,
            {
                |b, &s| {
                    let worker = OnsenBasicBoxLeakWorker::<SMALL>::new();
                    b.iter(|| {
                        worker.run_keep(*s);
                    })
                }
            },
        );
    }

    drop(simulated_work);

    let mut simulated_work = c.benchmark_group("simulated keep, medium data");

    for size in test_range.iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = OwnedWorker::<MEDIUM>::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("std::boxed::Box", size), &size, {
            |b, &s| {
                let worker = BoxWorker::<MEDIUM>::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen::Box", size), &size, {
            |b, &s| {
                let worker = OnsenBoxWorker::<MEDIUM>::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(
            BenchmarkId::new("leaking onsen::BasicBox", size),
            &size,
            {
                |b, &s| {
                    let worker = OnsenBasicBoxLeakWorker::<MEDIUM>::new();
                    b.iter(|| {
                        worker.run_keep(*s);
                    })
                }
            },
        );
    }

    drop(simulated_work);

    // Keep benchmarks
    let mut simulated_work = c.benchmark_group("simulated keep, big data");

    for size in test_range.iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("std::boxed::Box", size), &size, {
            |b, &s| {
                let worker = BoxWorker::<BIG>::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen::Box", size), &size, {
            |b, &s| {
                let worker = OnsenBoxWorker::<BIG>::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(
            BenchmarkId::new("leaking onsen::BasicBox", size),
            &size,
            {
                |b, &s| {
                    let worker = OnsenBasicBoxLeakWorker::<BIG>::new();
                    b.iter(|| {
                        worker.run_keep(*s);
                    })
                }
            },
        );
    }

    drop(simulated_work);

    // Drop benchmarks
    let mut simulated_work = c.benchmark_group("simulated drop, small data");

    for size in test_range.iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = OwnedWorker::<SMALL>::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("std::boxed::Box", size), &size, {
            |b, &s| {
                let worker = BoxWorker::<SMALL>::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen::Box", size), &size, {
            |b, &s| {
                let worker = OnsenBoxWorker::<SMALL>::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(
            BenchmarkId::new("leaking onsen::BasicBox", size),
            &size,
            {
                |b, &s| {
                    let worker = OnsenBasicBoxLeakWorker::<SMALL>::new();
                    b.iter(|| {
                        worker.run_drop(*s);
                    })
                }
            },
        );
    }
    drop(simulated_work);

    let mut simulated_work = c.benchmark_group("simulated drop, medium data");

    for size in test_range.iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = OwnedWorker::<MEDIUM>::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("std::boxed::Box", size), &size, {
            |b, &s| {
                let worker = BoxWorker::<MEDIUM>::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen::Box", size), &size, {
            |b, &s| {
                let worker = OnsenBoxWorker::<MEDIUM>::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(
            BenchmarkId::new("leaking onsen::BasicBox", size),
            &size,
            {
                |b, &s| {
                    let worker = OnsenBasicBoxLeakWorker::<MEDIUM>::new();
                    b.iter(|| {
                        worker.run_drop(*s);
                    })
                }
            },
        );
    }
    drop(simulated_work);

    let mut simulated_work = c.benchmark_group("simulated drop, big data");

    for size in test_range.iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("std::boxed::Box", size), &size, {
            |b, &s| {
                let worker = BoxWorker::<BIG>::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen::Box", size), &size, {
            |b, &s| {
                let worker = OnsenBoxWorker::<BIG>::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(
            BenchmarkId::new("leaking onsen::BasicBox", size),
            &size,
            {
                |b, &s| {
                    let worker = OnsenBasicBoxLeakWorker::<BIG>::new();
                    b.iter(|| {
                        worker.run_drop(*s);
                    })
                }
            },
        );
    }
    drop(simulated_work);
}

criterion_group!(benches2, criterion_benchmark);
criterion_main!(benches2);
