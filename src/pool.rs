use std::cell::RefCell;
use std::fmt;

use crate::*;

/// A single threaded, interior mutable memory Pool holding objects of type T.  Onsen Pools
/// obtain memory blocks from the global allocator. As long the Pool exists these blocks are
/// not given back to the allocator even when all entries are free. Only destruction of the
/// pool frees the associated blocks.
pub struct Pool<T: Sized>(RefCell<PoolInner<T>>);

impl<T> Pool<T> {
    /// Creates a new Pool for objects of type T.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(RefCell::new(PoolInner::new()))
    }
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for Pool<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_tuple("Pool").field(&self.0).finish()
    }
}

impl<T> Drop for Pool<T> {
    #[inline]
    fn drop(&mut self) {
        debug_assert!(
            self.with_lock(|pool| pool.is_all_free()),
            "Dropped pool with active allocations"
        );
    }
}

impl<T> PrivPoolApi<T> for Pool<T> {
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(&self, f: F) -> R {
        f(&mut self.0.borrow_mut())
    }
}

impl<T> PoolApi<T> for Pool<T> {}

#[cfg(test)]
mod pool_tests {
    use crate::*;

    #[test]
    fn smoke() {
        let _pool: Pool<String> = Pool::new();
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    #[cfg_attr(miri, ignore)]
    fn leak_unsafe_box() {
        let pool: Pool<String> = Pool::new();
        let bbox = pool.alloc("I am a Zombie".to_string());
        std::mem::forget(bbox)
    }
}
