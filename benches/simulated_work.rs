// testing rust allocator vs onsen without doing any work on the data
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

use onsen;

// The data we work on contains a primary value used for sorting and some payload which becomes mutated
struct Data<const N: usize> {
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

// Worker contains context and work we simulate
trait Worker<'a> {
    type Data: DataHandle;
    fn new() -> Self;
    fn new_element(&'a self, primary: u32) -> Option<Self::Data>;

    fn run(&'a self, howmuch: usize) {
        let mut state = 0xbabeface_u32;
        let mut workspace: Vec<Option<Self::Data>> = Vec::with_capacity(howmuch);
        for _ in 0..howmuch {
            match fast_prng(&mut state) % 100 {
                _ if workspace.len() < 50 => {
                    // warmup for the first 50 entries
                    workspace.push(self.new_element(fast_prng(&mut state)));
                }
                0..=59 => {
                    //push new entry
                    workspace.push(self.new_element(fast_prng(&mut state)));
                }
                60..=69 => {
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
                    // sort the first 10% of the vec values increasing, none at the end
                    let pos = fast_prng(&mut state) as usize % (workspace.len() / 10);
                    workspace[0..pos].sort_unstable_by(|a, b| {
                        match (a, b) {
                            (Some(a), Some(b)) => a.primary().partial_cmp(&b.primary()).unwrap(),
                            _ => std::cmp::Ordering::Greater, // Contains a 'None'
                        }
                    });
                }
                75..=79 => {
                    // sort the whole vec values increasing, none at the end
                    workspace.sort_unstable_by(|a, b| {
                        match (a, b) {
                            (Some(a), Some(b)) => a.primary().partial_cmp(&b.primary()).unwrap(),
                            _ => std::cmp::Ordering::Greater, // Contains a 'None'
                        }
                    });
                }
                80..=89 => {
                    // swap 2 entries, first from the first 10%, second from the first 25%
                    let pos1 = fast_prng(&mut state) as usize % (workspace.len() / 10);
                    let pos2 = fast_prng(&mut state) as usize % (workspace.len() / 4);
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

/// Owned data (copypaste, workaround the lack of GAT's, eventually needs macro or wait for GAT's)
struct SmallOwnedData(Data<3>);
struct MedOwnedData(Data<64>);
struct BigOwnedData(Data<1000>);

impl DataHandle for SmallOwnedData {
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

impl DataHandle for MedOwnedData {
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

impl DataHandle for BigOwnedData {
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
impl DataHandle for Box<Data<3>> {
    fn primary(&self) -> &u32 {
        &self.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.payload
    }
}

impl DataHandle for Box<Data<64>> {
    fn primary(&self) -> &u32 {
        &self.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.payload
    }
}

impl DataHandle for Box<Data<1000>> {
    fn primary(&self) -> &u32 {
        &self.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.payload
    }
}

// data in a onsen box
impl DataHandle for onsen::Box<'_, Data<3>, 1024> {
    fn primary(&self) -> &u32 {
        &self.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.payload
    }
}

impl DataHandle for onsen::Box<'_, Data<64>, 1024> {
    fn primary(&self) -> &u32 {
        &self.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.payload
    }
}

impl DataHandle for onsen::Box<'_, Data<1000>, 1024> {
    fn primary(&self) -> &u32 {
        &self.primary
    }

    fn primary_mut(&mut self) -> &mut u32 {
        &mut self.primary
    }

    fn payload(&mut self) -> &mut [u32] {
        &mut self.payload
    }
}

// Now implement the workers for owned
struct SmallOwnedWorker;
struct MedOwnedWorker;
struct BigOwnedWorker;

impl Worker<'_> for SmallOwnedWorker {
    type Data = SmallOwnedData;
    fn new() -> Self {
        SmallOwnedWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(SmallOwnedData(Data::new(primary)))
    }
}

impl Worker<'_> for MedOwnedWorker {
    type Data = MedOwnedData;
    fn new() -> Self {
        MedOwnedWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(MedOwnedData(Data::new(primary)))
    }
}

impl Worker<'_> for BigOwnedWorker {
    type Data = BigOwnedData;
    fn new() -> Self {
        BigOwnedWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(BigOwnedData(Data::new(primary)))
    }
}

// Now implement the workers for rust boxes
struct SmallBoxWorker;
struct MedBoxWorker;
struct BigBoxWorker;

impl Worker<'_> for SmallBoxWorker {
    type Data = Box<Data<3>>;
    fn new() -> Self {
        SmallBoxWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(Box::new(Data::new(primary)))
    }
}

impl Worker<'_> for MedBoxWorker {
    type Data = Box<Data<64>>;
    fn new() -> Self {
        MedBoxWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(Box::new(Data::new(primary)))
    }
}

impl Worker<'_> for BigBoxWorker {
    type Data = Box<Data<1000>>;
    fn new() -> Self {
        BigBoxWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(Box::new(Data::new(primary)))
    }
}

// // Now implement the workers for onsen boxes
struct SmallOnsenWorker {
    pool: onsen::Pool<Data<3>, 1024>,
}

struct MedOnsenWorker {
    pool: onsen::Pool<Data<64>, 1024>,
}

struct BigOnsenWorker {
    pool: onsen::Pool<Data<1000>, 1024>,
}

impl<'a> Worker<'a> for SmallOnsenWorker {
    type Data = onsen::Box<'a, Data<3>, 1024>;
    fn new() -> Self {
        SmallOnsenWorker {
            pool: onsen::Pool::new(),
        }
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(self.pool.alloc_box(Data::new(primary)))
    }
}

impl<'a> Worker<'a> for MedOnsenWorker {
    type Data = onsen::Box<'a, Data<64>, 1024>;
    fn new() -> Self {
        MedOnsenWorker {
            pool: onsen::Pool::new(),
        }
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(self.pool.alloc_box(Data::new(primary)))
    }
}

impl<'a> Worker<'a> for BigOnsenWorker {
    type Data = onsen::Box<'a, Data<1000>, 1024>;
    fn new() -> Self {
        BigOnsenWorker {
            pool: onsen::Pool::new(),
        }
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(self.pool.alloc_box(Data::new(primary)))
    }
}

#[inline(always)]
fn fast_prng(state: &mut u32) -> u32 {
    let rand = *state;
    *state = rand << 1 ^ ((rand >> 30) & 1) ^ ((rand >> 2) & 1);
    rand
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut simulated_work = c.benchmark_group("simulated work, small data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = SmallOwnedWorker::new();
                b.iter(|| {
                    worker.run(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = SmallBoxWorker::new();
                b.iter(|| {
                    worker.run(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = SmallOnsenWorker::new();
                b.iter(|| {
                    worker.run(*s);
                })
            }
        });
    }

    drop(simulated_work);
    let mut simulated_work = c.benchmark_group("simulated work, medium data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(60));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = MedOwnedWorker::new();
                b.iter(|| {
                    worker.run(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = MedBoxWorker::new();
                b.iter(|| {
                    worker.run(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = MedOnsenWorker::new();
                b.iter(|| {
                    worker.run(*s);
                })
            }
        });
    }

    drop(simulated_work);
    let mut simulated_work = c.benchmark_group("simulated work, big data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(90));

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = BigBoxWorker::new();
                b.iter(|| {
                    worker.run(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = BigOnsenWorker::new();
                b.iter(|| {
                    worker.run(*s);
                })
            }
        });
    }

    drop(simulated_work);
}

criterion_group!(benches2, criterion_benchmark);
criterion_main!(benches2);
