use std::cell::RefCell;
use threadcell::ThreadCell;

use crate::*;

/// A single thread, interior mutable memory Pool holding objects of type T.
pub struct STPool<T: Sized>(ThreadCell<RefCell<PoolInner<T>>>);

impl<T> STPool<T> {
    /// Creates a new `STPool` for objects of type T.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(ThreadCell::new_disowned(RefCell::new(PoolInner::new())))
    }

    /// Releases the threads ownership of the `STPool` so that some other thread can use it.
    /// When a thread exits it should release the pool, otherwise other threads can't pick it
    /// up.  Returns *true* when the pool was successful released and *false* when the current
    /// thread does not own the pool.
    ///
    #[cfg_attr(
        feature = "st_tbox",
        doc = r##"
    use onsen::*;

    struct MyTag;
    define_tbox_pool!(MyTag: u8);
    {
        // Drops the box instantly
        let _ = TBox::new(123u8, MyTag);
    }

    // This is how the pool gets eventually released.
    //
    // Important: If any access to the pool follows this, including
    //             dropping boxes, the pool will be re-acquired!
    assert!(TBox::<u8, MyTag>::get_pool().release())
    "##
    )]
    pub fn release(&self) -> bool {
        if self.0.is_owned() {
            self.0.release();
            true
        } else {
            false
        }
    }

    /// Recovers the `STPool` when its owning thread has ended without releasing it (eg. after
    /// a panic).
    ///
    /// # Safety
    ///
    /// It is UB to steal a pool from a thread that is still using it.
    pub unsafe fn steal(&self) {
        self.0.steal();
    }
}

impl<T> PoolApi<T> for STPool<T> {}

impl<T> PoolLock<T> for &STPool<T> {
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(self, f: F) -> R {
        f(&mut self.0.acquire_get().borrow_mut())
    }
}

impl<T> Default for STPool<T> {
    fn default() -> Self {
        Self::new()
    }
}
