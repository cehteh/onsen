#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

// A Pool can hold at most $2**NUM_BLOCKS * E - E$ entries. Common architectures (x86_64 and
// arm64) use at most 48bits for the virtual address space. An Entry is at least 8 bytes wide.
// Thus NUM_BLOCKS=45 would hold for all theoretically possible number of allocations even in
// the case that $E=1$, in practice memory should run out *much* earlier. Actually the last
// allocation is guaranteed to fail because it would dedicate almost the complete address
// space to a single Pool.  A value of 44 covering one third of the available address space
// already exceeds most computers memory capacity (by many magnitudes). This explanations is
// for the worst case scenario where $E=1$, usually one want E to be considerably larger.
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

mod pool;
pub use pool::*;

mod slot;
pub use slot::*;

mod entry;
use entry::*;

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn alloc_drop() {
        let mut pool: Pool<&str, 128> = Pool::new();

        let memory = pool.alloc("Hello Memory");
        unsafe {
            pool.drop(memory);
        }
    }

    #[test]
    fn alloc_access() {
        let mut pool: Pool<&str, 128> = Pool::new();

        let mut memory = pool.alloc("Hello Memory");

        assert_eq!(memory.get(), &"Hello Memory");
        assert_eq!(memory.get_mut(), &"Hello Memory");
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
    }
}
