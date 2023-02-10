use crate::*;
use erasable::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A single thread, interior mutable memory Pool backed by a reference count.  This allows
/// objects to hold references back to the pool to keep it alive without carrying a lifetime.
pub struct RcPool<T: PoolEntry>(Rc<RefCell<PoolInner<T>>>);

impl<T: PoolEntry> RcPool<T> {
    /// Creates a new `RcPool` for objects of type T.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(PoolInner::new())))
    }
}

impl<T: PoolEntry> Clone for RcPool<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: PoolEntry> Default for RcPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PoolEntry> PrivPoolApi for RcPool<T> {
    type Entry = T;

    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<Self::Entry>) -> R>(&self, f: F) -> R {
        f(&mut self.0.borrow_mut())
    }
}

impl<T: PoolEntry> PoolApi for RcPool<T> {}

impl<T: PoolEntry> SharedPoolApi for RcPool<T> {}

unsafe impl<T: PoolEntry> ErasablePtr for RcPool<T> {
    fn erase(this: Self) -> ErasedPtr {
        ErasablePtr::erase(this.0)
    }

    unsafe fn unerase(this: ErasedPtr) -> Self {
        Self(ErasablePtr::unerase(this))
    }
}

impl<T: PoolEntry> CloneSharedPool for RcPool<T> {
    type Pool = Self;
    #[inline]
    fn clone_shared_pool(&self) -> Self {
        self.clone()
    }
}
