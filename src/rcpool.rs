use crate::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A single thread, interior mutable memory Pool backed by a reference count.  This allows
/// objects to hold references back to the pool to keep it alive without carrying a lifetime.
pub struct RcPool<T: Sized>(Rc<RefCell<PoolInner<T>>>);

impl<T> RcPool<T> {
    /// Creates a new `RcPool` for objects of type T.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(PoolInner::new())))
    }
}

impl<T> Clone for RcPool<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for RcPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PrivPoolApi<T> for RcPool<T> {
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(&self, f: F) -> R {
        f(&mut self.0.borrow_mut())
    }
}

impl<T> PoolApi<T> for RcPool<T> {}

impl<T> SharedPoolApi<T> for RcPool<T> {}

impl<T> AsSharedPool<T, RcPool<T>> for RcPool<T> {
    #[inline]
    fn as_shared_pool(&self) -> &Self {
        self
    }
}

#[cfg(test)]
mod pool_tests {
    use crate::*;

    #[test]
    fn smoke() {
        let _pool: RcPool<String> = RcPool::new();
    }
}
