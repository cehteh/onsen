// testing rust allocator vs onsen without doing any work on the data
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

use onsen;
use onsen::PoolApi;

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

/// Owned data (copypaste, workaround the lack of GAT's, eventually needs macro or wait for GAT's)
pub struct SmallOwnedData(Data<3>, usize);
pub struct MedOwnedData(Data<64>, usize);
pub struct BigOwnedData(Data<1000>, usize);

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
pub struct SmallBoxedData(Box<Data<3>>);
pub struct MedBoxedData(Box<Data<64>>);
pub struct BigBoxedData(Box<Data<1000>>);

impl DataHandle for SmallBoxedData {
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

impl DataHandle for MedBoxedData {
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

impl DataHandle for BigBoxedData {
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
impl DataHandle for onsen::Box<'_, Data<3>> {
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

impl DataHandle for onsen::Box<'_, Data<64>> {
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

impl DataHandle for onsen::Box<'_, Data<1000>> {
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

// data in a onsen tbox
#[cfg(feature = "tbox")]
struct Bench;
#[cfg(feature = "tbox")]
onsen::define_tbox_pool!(Bench: Data<3>);
#[cfg(feature = "tbox")]
onsen::define_tbox_pool!(Bench: Data<64>);
#[cfg(feature = "tbox")]
onsen::define_tbox_pool!(Bench: Data<1000>);

#[cfg(feature = "tbox")]
impl DataHandle for onsen::TBox<Data<3>, Bench> {
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

#[cfg(feature = "tbox")]
impl DataHandle for onsen::TBox<Data<64>, Bench> {
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

#[cfg(feature = "tbox")]
impl DataHandle for onsen::TBox<Data<1000>, Bench> {
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
#[repr(C)]
pub struct SmallOwnedWorker;
#[repr(C)]
pub struct MedOwnedWorker;
#[repr(C)]
pub struct BigOwnedWorker;

impl Worker<'_> for SmallOwnedWorker {
    type Data = SmallOwnedData;
    fn new() -> Self {
        SmallOwnedWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(SmallOwnedData(Data::new(primary), primary as usize))
    }
}

impl Worker<'_> for MedOwnedWorker {
    type Data = MedOwnedData;
    fn new() -> Self {
        MedOwnedWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(MedOwnedData(Data::new(primary), primary as usize))
    }
}

impl Worker<'_> for BigOwnedWorker {
    type Data = BigOwnedData;
    fn new() -> Self {
        BigOwnedWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(BigOwnedData(Data::new(primary), primary as usize))
    }
}

// Now implement the workers for rust boxes
#[repr(C)]
pub struct SmallBoxWorker;
#[repr(C)]
pub struct MedBoxWorker;
#[repr(C)]
pub struct BigBoxWorker;

impl Worker<'_> for SmallBoxWorker {
    type Data = SmallBoxedData;
    fn new() -> Self {
        SmallBoxWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(SmallBoxedData(Box::new(Data::new(primary))))
    }
}

impl Worker<'_> for MedBoxWorker {
    type Data = MedBoxedData;
    fn new() -> Self {
        MedBoxWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(MedBoxedData(Box::new(Data::new(primary))))
    }
}

impl Worker<'_> for BigBoxWorker {
    type Data = BigBoxedData;
    fn new() -> Self {
        BigBoxWorker
    }

    fn new_element(&self, primary: u32) -> Option<Self::Data> {
        Some(BigBoxedData(Box::new(Data::new(primary))))
    }
}

// // Now implement the workers for onsen boxes
pub struct SmallOnsenWorker {
    pool: onsen::Pool<Data<3>>,
}

pub struct MedOnsenWorker {
    pool: onsen::Pool<Data<64>>,
}

pub struct BigOnsenWorker {
    pool: onsen::Pool<Data<1000>>,
}

impl<'a> Worker<'a> for SmallOnsenWorker {
    type Data = onsen::Box<'a, Data<3>>;
    fn new() -> Self {
        let pool = onsen::Pool::new();
        pool.with_min_entries(1000);
        SmallOnsenWorker { pool }
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(onsen::Box::new(Data::new(primary), &self.pool))
    }
}

impl<'a> Worker<'a> for MedOnsenWorker {
    type Data = onsen::Box<'a, Data<64>>;
    fn new() -> Self {
        let pool = onsen::Pool::new();
        pool.with_min_entries(1000);
        MedOnsenWorker { pool }
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(onsen::Box::new(Data::new(primary), &self.pool))
    }
}

impl<'a> Worker<'a> for BigOnsenWorker {
    type Data = onsen::Box<'a, Data<1000>>;
    fn new() -> Self {
        let pool = onsen::Pool::new();
        pool.with_min_entries(1000);
        BigOnsenWorker { pool }
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(onsen::Box::new(Data::new(primary), &self.pool))
    }
}

// Now implement the workers for onsen tbox
pub struct SmallOnsenTBoxWorker;

pub struct MedOnsenTBoxWorker;

pub struct BigOnsenTBoxWorker;

#[cfg(feature = "tbox")]
impl<'a> Worker<'a> for SmallOnsenTBoxWorker {
    type Data = onsen::TBox<Data<3>, Bench>;
    fn new() -> Self {
        SmallOnsenTBoxWorker
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(onsen::TBox::new(Data::new(primary), Bench))
    }
}

#[cfg(feature = "tbox")]
impl<'a> Worker<'a> for MedOnsenTBoxWorker {
    type Data = onsen::TBox<Data<64>, Bench>;
    fn new() -> Self {
        MedOnsenTBoxWorker
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(onsen::TBox::new(Data::new(primary), Bench))
    }
}

#[cfg(feature = "tbox")]
impl<'a> Worker<'a> for BigOnsenTBoxWorker {
    type Data = onsen::TBox<Data<1000>, Bench>;
    fn new() -> Self {
        BigOnsenTBoxWorker
    }

    fn new_element(&'a self, primary: u32) -> Option<Self::Data> {
        Some(onsen::TBox::new(Data::new(primary), Bench))
    }
}

//

#[inline(always)]
fn fast_prng(state: &mut u32) -> u32 {
    let rand = *state;
    *state = rand << 1 ^ ((rand >> 30) & 1) ^ ((rand >> 2) & 1);
    rand
}

fn criterion_benchmark(c: &mut Criterion) {
    // Keep benchmarks
    let mut simulated_work = c.benchmark_group("simulated keep, small data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = SmallOwnedWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = SmallBoxWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = SmallOnsenWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        #[cfg(feature = "tbox")]
        simulated_work.bench_with_input(BenchmarkId::new("onsen tbox", size), &size, {
            |b, &s| {
                let worker = SmallOnsenTBoxWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });
    }

    drop(simulated_work);
    let mut simulated_work = c.benchmark_group("simulated keep, medium data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(60));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = MedOwnedWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = MedBoxWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = MedOnsenWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        #[cfg(feature = "tbox")]
        simulated_work.bench_with_input(BenchmarkId::new("onsen tbox", size), &size, {
            |b, &s| {
                let worker = MedOnsenTBoxWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });
    }

    drop(simulated_work);
    let mut simulated_work = c.benchmark_group("simulated keep, big data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(90));

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = BigBoxWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = BigOnsenWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });

        #[cfg(feature = "tbox")]
        simulated_work.bench_with_input(BenchmarkId::new("onsen tbox", size), &size, {
            |b, &s| {
                let worker = BigOnsenTBoxWorker::new();
                b.iter(|| {
                    worker.run_keep(*s);
                })
            }
        });
    }

    drop(simulated_work);

    // Drop benchmarks
    let mut simulated_work = c.benchmark_group("simulated drop, small data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(30));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = SmallOwnedWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = SmallBoxWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = SmallOnsenWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        #[cfg(feature = "tbox")]
        simulated_work.bench_with_input(BenchmarkId::new("onsen tbox", size), &size, {
            |b, &s| {
                let worker = SmallOnsenTBoxWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });
    }

    drop(simulated_work);
    let mut simulated_work = c.benchmark_group("simulated drop, medium data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(60));

        simulated_work.bench_with_input(BenchmarkId::new("owned", size), &size, {
            |b, &s| {
                let worker = MedOwnedWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = MedBoxWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = MedOnsenWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        #[cfg(feature = "tbox")]
        simulated_work.bench_with_input(BenchmarkId::new("onsen tbox", size), &size, {
            |b, &s| {
                let worker = MedOnsenTBoxWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });
    }

    drop(simulated_work);
    let mut simulated_work = c.benchmark_group("simulated drop, big data");

    for size in [100, 500, 1000, 3000, 5000, 7500, 10000].iter() {
        simulated_work.throughput(Throughput::Elements(*size as u64));
        simulated_work.measurement_time(Duration::from_secs(90));

        simulated_work.bench_with_input(BenchmarkId::new("rust box", size), &size, {
            |b, &s| {
                let worker = BigBoxWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        simulated_work.bench_with_input(BenchmarkId::new("onsen box", size), &size, {
            |b, &s| {
                let worker = BigOnsenWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });

        #[cfg(feature = "tbox")]
        simulated_work.bench_with_input(BenchmarkId::new("onsen tbox", size), &size, {
            |b, &s| {
                let worker = BigOnsenTBoxWorker::new();
                b.iter(|| {
                    worker.run_drop(*s);
                })
            }
        });
    }

    drop(simulated_work);
}

criterion_group!(benches2, criterion_benchmark);
criterion_main!(benches2);
