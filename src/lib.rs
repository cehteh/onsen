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

mod block;
use block::*;

mod pool;
pub use pool::*;

mod slot;
pub use slot::*;

mod entry;
use entry::*;

mod boxed;
pub use boxed::*;

mod refcounted;
pub use refcounted::*;

mod strongcounted;
pub use strongcounted::*;

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
        let pool: Pool<&str> = Pool::new();

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
        let pool: Pool<&str> = Pool::new();

        let mut memory = pool.alloc("Hello Memory");

        assert_eq!(memory.get_mut(), &"Hello Memory");
        assert_eq!(&*memory.pin(), &"Hello Memory");
    }

    #[test]
    fn alloc_uninit() {
        let pool: Pool<&str> = Pool::new();

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
