#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

// A Pool can hold at most $2**NUM_BLOCKS * E - E$ entries. Common architectures (x86_64 and
// arm64) use at most 48bits for the virtual address space. An Entry is at least 8 bytes wide.
// Thus NUM_BLOCKS=45 would hold for all theoretically possible number of allocations even in
// the case that $E=1$, in practice memory should run out *much* earlier.  A value of 44
// covering one third of the available address space already exceeds most computers memory
// capacity (by many magnitudes). This explanations is for the worst case scenario where
// $E=1$, usually one want E to be considerably larger.
/// Maximum number of blocks in a Pool
pub(crate) const NUM_BLOCKS: usize = 44;

/// Valid, initialized slot from which a mutable pin was taken
pub(crate) const PINNED: usize = 1;
/// Valid, initialized slot from which a mutable reference was taken
pub(crate) const REFERENCED: usize = 2;
/// Initialized slot
pub(crate) const INITIALIZED: usize = 3;
/// Uninitialized slot
pub(crate) const UNINITIALIZED: usize = 4;

// best guess values, eventually these should be configured properly
const CACHELINE_SIZE: usize = 128;
const PAGE_SIZE: usize = 4096;
const HUGEPAGE_SIZE: usize = 2097152; // 2MB
const ONEGIGABYTE_SIZE: usize = 1073741824;

mod pool;
pub use pool::*;

mod slot;
pub use slot::*;

mod entry;
use entry::*;

mod boxed;
pub use boxed::*;

/// Helper trait for configuring the optimal block size E. Note that Objects must be smaller
/// than the granularity picked here. Ideally much smaller (by a factor greater than 10).
pub trait OptimalBlockSize {
    /// Make E big enough to fill a single cacheline. Good for small T and highly dynamic
    /// memory usage where that also may see very little memory usage.
    const CACHELINE: usize;
    /// Make E big enough to fill a memory page. Possibly the best for almost any use case as
    /// long the stored objects are reasonable small (smaller than ~500 Bytes).
    const PAGE: usize;
    /// Make E big enough to fill a memory hugepage. Better suited for bigger objects (more
    /// than ~500 Bytes) or when a lot allocations are expected.
    const HUGEPAGE: usize;
    /// Make E big enough to fill 1GB of memory. Good suited for big objects and when a
    /// lot allocations are expected.
    const ONEGIGABYTE: usize;
}

impl<T: Sized> OptimalBlockSize for T {
    const CACHELINE: usize = CACHELINE_SIZE / std::mem::size_of::<Entry<T>>();
    const PAGE: usize = PAGE_SIZE / std::mem::size_of::<Entry<T>>();
    const HUGEPAGE: usize = HUGEPAGE_SIZE / std::mem::size_of::<Entry<T>>();
    const ONEGIGABYTE: usize = ONEGIGABYTE_SIZE / std::mem::size_of::<Entry<T>>();
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn optimal_size() {
        let _pool: Pool<&str, { <&str>::PAGE }> = Pool::new();
        let _pool = pool!(&str, PAGE);
    }

    #[test]
    fn alloc_free() {
        let mut pool: Pool<&str, 128> = Pool::new();

        let memory = pool.alloc("Hello Memory");
        unsafe {
            pool.free(memory);
        }
    }

    #[test]
    fn pool_leak() {
        let mut pool: Pool<&str, 128> = Pool::new();

        let _memory = pool.alloc("Hello Memory");

        pool.leak();
    }

    #[test]
    fn alloc_access() {
        let mut pool: Pool<&str, 128> = Pool::new();

        let mut memory = pool.alloc("Hello Memory");

        assert_eq!(memory.get(), &"Hello Memory");
        assert_eq!(memory.get_mut(), &"Hello Memory");

        unsafe {
            pool.free(memory);
        }
    }

    #[test]
    fn alloc_more() {
        let mut slots = Vec::new();
        let mut pool: Pool<&str, 128> = Pool::new();

        for _i in 0..1000 {
            slots.push(pool.alloc("Hello Memory"));
        }

        unsafe {
            slots.drain(..).for_each(|slot| pool.free(slot));
        }
    }

    #[test]
    #[should_panic]
    fn alloc_pincheck() {
        let mut pool: Pool<&str, 128> = Pool::new();

        let mut memory = pool.alloc("Hello Memory");

        assert_eq!(memory.get_mut(), &"Hello Memory");
        assert_eq!(&*memory.pin(), &"Hello Memory");
    }

    #[test]
    fn alloc_uninit() {
        let mut pool: Pool<&str, 128> = Pool::new();

        let mut memory = pool.alloc_uninit();

        unsafe {
            memory.get_uninit().write("Hello Init");
            memory.assume_init();
        }

        assert_eq!(memory.get(), &"Hello Init");

        unsafe {
            pool.free(memory);
        }
    }
}
