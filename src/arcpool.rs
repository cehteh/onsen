use crate::*;
use erasable::*;
use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::Mutex;

#[cfg(feature = "parking_lot")]
use parking_lot::Mutex;

/// A single thread, interior mutable memory Pool backed by a reference count.  This allows
/// objects to hold references back to the pool to keep it alive without carrying a lifetime.
pub struct ArcPool<T: PoolEntry>(Arc<Mutex<PoolInner<T>>>);

impl<T: PoolEntry> ArcPool<T> {
    /// Creates a new `ArcPool` for objects of type T.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(PoolInner::new())))
    }
}

impl<T: PoolEntry> Clone for ArcPool<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: PoolEntry> Default for ArcPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PoolEntry> PrivPoolApi for ArcPool<T> {
    type Entry = T;

    #[cfg(not(feature = "parking_lot"))]
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<Self::Entry>) -> R>(&self, f: F) -> R {
        f(&mut self.0.lock().expect("Poisoned Mutex"))
    }

    #[cfg(feature = "parking_lot")]
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<Self::Entry>) -> R>(&self, f: F) -> R {
        f(&mut self.0.lock())
    }
}

impl<T: PoolEntry> PoolApi for ArcPool<T> {}

impl<T: PoolEntry> SharedPoolApi for ArcPool<T> {}

unsafe impl<T: PoolEntry> ErasablePtr for ArcPool<T> {
    fn erase(this: Self) -> ErasedPtr {
        ErasablePtr::erase(this.0)
    }

    unsafe fn unerase(this: ErasedPtr) -> Self {
        Self(ErasablePtr::unerase(this))
    }
}

impl<T: PoolEntry> CloneSharedPool for ArcPool<T> {
    type Pool = Self;
    #[inline]
    fn clone_shared_pool(&self) -> Self {
        self.clone()
    }
}
