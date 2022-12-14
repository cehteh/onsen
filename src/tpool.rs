use crate::*;

#[cfg(not(feature = "parking_lot"))]
use std::sync::Mutex;

#[cfg(feature = "parking_lot")]
use parking_lot::Mutex;

/// A threadsafe, interior mutable memory Pool holding objects of type T.  The whole pool is
/// protected by a single lock, thread safety is not meant to scale here. When scalability
/// over many threads is needed then onsen is not the right tool.
pub struct TPool<T: Sized>(Mutex<PoolInner<T>>);

impl<T> TPool<T> {
    /// Creates a new `TPool` for objects of type T.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(Mutex::new(PoolInner::new()))
    }

    /// Mock the `STPool` API's to make `TPool` a drop in replacement.
    #[inline(always)]
    pub fn acquire(&self) -> Result<(), PoolOwnershipError> {
        Ok(())
    }

    /// Mock the `STPool` API's to make `TPool` a drop in replacement.
    #[inline(always)]
    pub fn acquire_guard(&self) -> Result<(), PoolOwnershipError> {
        Ok(())
    }

    /// Mock the `STPool` API's to make `TPool` a drop in replacement.
    #[inline(always)]
    pub fn release(&self) -> Result<(), PoolOwnershipError> {
        Ok(())
    }

    /// Mock the `STPool` API's to make `TPool` a drop in replacement.
    #[inline(always)]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn force_release(&self) {}
}

impl<T> PoolApi<T> for TPool<T> {}

impl<T> PoolLock<T> for &TPool<T> {
    #[inline]
    #[cfg(not(feature = "parking_lot"))]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(self, f: F) -> R {
        f(&mut self.0.lock().expect("Failed to lock Mutex"))
    }

    #[inline]
    #[cfg(feature = "parking_lot")]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(self, f: F) -> R {
        f(&mut self.0.lock())
    }
}

impl<T> Default for TPool<T> {
    fn default() -> Self {
        Self::new()
    }
}
