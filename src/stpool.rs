#![cfg(feature = "stpool")]
use std::cell::RefCell;
use threadcell::{Guard as STPoolGuard, ThreadCell};

use crate::*;

/// A single thread, interior mutable memory Pool holding objects of type T that can
/// cooperatively moved between threads.
///
/// Cooperatively means that threads have to acquire the pool before using it and release it
/// when done with it. Unlike mutexes this is meant to be some long time acquisition.
/// In single threaded applications the `release()` call is optional.
///
/// # Panics
///
/// Accessing a pool that it not acquired will panic.
///
/// # Example
/// ```rust,ignore
/// use onsen::*;
///
/// struct MyTag;
/// define_tbox_pool!(MyTag: u8);
///
/// // acquire the pool before doing work
/// TBox::<u8, MyTag>::pool().acquire().expect("some other thread owns the pool");
///
/// // Do some work
/// {
///     // Drops the box instantly
///     let _ = TBox::new(123u8, MyTag);
/// }
///
/// // Release the pool when done.
/// //
/// // Important: If any access to the pool follows this, including
/// //            dropping boxes, the thread will panic!
/// TBox::<u8, MyTag>::pool().release().expect("did not own the pool");
/// ```
pub struct STPool<T: Sized>(ThreadCell<RefCell<PoolInner<T>>>);

impl<T> STPool<T> {
    /// Creates a new `STPool` for objects of type T.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(ThreadCell::new_disowned(RefCell::new(PoolInner::new())))
    }

    /// Acquire the ownership of the `STPool` by the current thread.  Must be called before
    /// any operation on the pool is done. The pool will stay acquired until it is
    /// released. Returns `Ok(())` when the pool was successful acquired and
    /// `Err(PoolOwnershipError)` when the current thread could not acquire the pool.
    pub fn acquire(&self) -> Result<(), PoolOwnershipError> {
        if self.0.try_acquire() {
            Ok(())
        } else {
            Err(PoolOwnershipError)
        }
    }

    /// Releases the threads ownership of the `STPool` so that some other thread can use it.
    /// When a thread exits it should release the pool, otherwise other threads can't pick it
    /// up.  Returns `Ok(())` when the pool was successful released and
    /// `Err(PoolOwnershipError)` when the current thread did not own the pool.
    pub fn release(&self) -> Result<(), PoolOwnershipError> {
        if self.0.try_release() {
            Ok(())
        } else {
            Err(PoolOwnershipError)
        }
    }

    /// Acquire the ownership of the `STPool` with a `STPoolGuard`. The pool will stay
    /// acquired until this guard becomes dropped.  Returns `Ok(STPoolGuard<..>` when the pool
    /// was successful acquired and `Err(PoolOwnershipError)` when the current thread did not
    /// own the pool. The returned guard itself is opaque, it just needs to stay around as
    /// long as needed.
    #[must_use = "the returned guard holds the threads ownership of the pool"]
    pub fn acquire_guard(&self) -> Result<STPoolGuard<RefCell<PoolInner<T>>>, PoolOwnershipError> {
        let guard = STPoolGuard::new(&self.0);
        if guard.inner().try_acquire() {
            Ok(guard)
        } else {
            Err(PoolOwnershipError)
        }
    }

    /// Recovers the `STPool` when its owning thread has ended without releasing it (eg. after
    /// a panic).
    ///
    /// # Safety
    ///
    /// It is UB to `force_release()` a pool from a thread that is still using it.
    pub unsafe fn force_release(&self) {
        self.0.steal().release();
    }
}

impl<T> PoolApi<T> for STPool<T> {}

impl<T> PoolLock<T> for &STPool<T> {
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(self, f: F) -> R {
        f(&mut self.0.get().borrow_mut())
    }
}

impl<T> Default for STPool<T> {
    fn default() -> Self {
        Self::new()
    }
}
