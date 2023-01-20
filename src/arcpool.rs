use crate::*;
use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::Mutex;

#[cfg(feature = "parking_lot")]
use parking_lot::Mutex;

/// A multithreaded, interior mutable memory Pool backed by a reference count.  This allows
/// objects to hold references back to the pool to keep it alive without carrying a lifetime.
pub struct ArcPool<T: Sized>(Arc<Mutex<PoolInner<T>>>);

impl<T> ArcPool<T> {
    /// Creates a new `ArcPool` for objects of type T.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(PoolInner::new())))
    }
}

impl<T> Clone for ArcPool<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for ArcPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PrivPoolApi<T> for ArcPool<T> {
    #[cfg(not(feature = "parking_lot"))]
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(&self, f: F) -> R {
        f(&mut self.0.lock().expect("Poisoned Mutex"))
    }

    #[cfg(feature = "parking_lot")]
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(&self, f: F) -> R {
        f(&mut self.0.lock())
    }
}

impl<T> PoolApi<T> for ArcPool<T> {}

impl<T> SharedPoolApi<T> for ArcPool<T> {}

impl<T> AsSharedPool<T, ArcPool<T>> for ArcPool<T> {
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
        let _pool: ArcPool<String> = ArcPool::new();
    }
}

#[test]
fn size() {
    assert_eq!(
        std::mem::size_of::<UnsafeBox<usize>>(),
        std::mem::size_of::<[usize; 1]>()
    );
}
