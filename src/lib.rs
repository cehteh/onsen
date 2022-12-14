#![doc = include_str!("../README.md")]
// uncomment for discovering new lints:
// #![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![warn(clippy::cargo_common_metadata)]
#![warn(clippy::doc_markdown)]
#![warn(clippy::missing_panics_doc)]
#![warn(clippy::must_use_candidate)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

/// Maximum number of blocks in a Pool
pub(crate) const NUM_BLOCKS: usize = 44;

#[cfg(feature = "tbox")]
#[doc(hidden)]
pub use assoc_static::*;

mod block;
use block::*;

mod pool;
pub use pool::*;

mod rcpool;
pub use rcpool::*;

mod tpool;
pub use tpool::*;

mod stpool;
#[cfg(feature = "stpool")]
pub use stpool::*;

mod slot;
pub use slot::*;

mod entry;
pub use entry::*;

mod boxed;
pub use boxed::*;

mod refcounted;
pub use refcounted::*;

mod strongcounted;
pub use strongcounted::*;

mod tboxed;
#[cfg(feature = "tbox")]
pub use tboxed::*;

mod trefcounted;
#[cfg(feature = "tbox")]
pub use trefcounted::*;

mod tstrongcounted;
#[cfg(feature = "tbox")]
pub use tstrongcounted::*;

/// The error returned when a `STPool` can not be acquired or released.
#[derive(Debug, Copy, Clone)]
pub struct PoolOwnershipError;

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn alloc_free() {
        let pool: Pool<&str> = Pool::new();

        let memory = pool.alloc("Hello Memory");
        unsafe {
            pool.free(memory);
        }
    }

    #[test]
    fn pool_leak() {
        let pool: Pool<&str> = Pool::new();

        let _memory = pool.alloc("Hello Memory");

        pool.leak();
    }

    #[test]
    fn alloc_access() {
        let pool: Pool<&str> = Pool::new();

        let mut memory = pool.alloc("Hello Memory").for_mutation();

        assert_eq!(memory.get(), &"Hello Memory");
        assert_eq!(memory.get_mut(), &"Hello Memory");

        unsafe {
            pool.free(memory);
        }
    }

    #[test]
    fn alloc_more() {
        let mut slots = Vec::new();
        let pool: Pool<&str> = Pool::new();

        for _i in 0..1000 {
            slots.push(pool.alloc("Hello Memory"));
        }

        unsafe {
            slots.drain(..).for_each(|slot| pool.free(slot));
        }
    }

    #[test]
    fn alloc_uninit() {
        let pool: Pool<&str> = Pool::new();

        let mut memory = pool.alloc_uninit();

        let memory = unsafe {
            memory.get_uninit().write("Hello Init");
            memory.assume_init()
        };

        assert_eq!(memory.get(), &"Hello Init");

        unsafe {
            pool.free(memory);
        }
    }
}
