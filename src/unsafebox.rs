use std::fmt;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use crate::*;

/// This wraps the pointer to the allocation. Its main purpose is for implementing higher
/// level safe types of boxes. A `UnsafeBox` should eventually be given back to the pool they
/// belong to by `pool.dealloc()`, `pool.forget()` or `pool.take()`. A `UnsafeBox` does not
/// track which Pool they belong to, neither does it track the lifetime of its pool, therefore
/// it is the responsibility of the user to give them back to the correct pool or drop them
/// before the pool gets dropped. Safe abstractions should track the associated pool and add
/// lifetimes.
///
/// When a `UnsafeBox` goes out of scope while it is not explicitly given back to the pool its
/// contents will be properly destructed while the associated memory will leak within the `Pool`
/// from where it was allocated. This happens especially when panicking drops unsafe boxes.
///
/// # Safety
///
///  * A `UnsafeBox` must be dropped before its Pool becomes dropped, failing to do so is UB.
///
#[repr(transparent)]
pub struct UnsafeBox<T>(
    // This Option is always `Some()` in live objects, only `dealloc*()`, `forget()` and
    // `take()` which consume the box sets it to `None` to notify the `Drop` implementation
    // that the value is already destructed.
    Option<*mut Entry<T>>,
);

unsafe impl<T: Send> Send for UnsafeBox<T> {}
unsafe impl<T: Sync> Sync for UnsafeBox<T> {}

impl<T> UnsafeBox<T> {
    // Private ctor
    pub(crate) unsafe fn new(from: &mut Entry<T>) -> Self {
        Self(Some(from))
    }
}

impl<T> UnsafeBox<T> {
    #[track_caller]
    pub(crate) fn assert_initialized(&self) {
        assert!(self.0.is_some());
    }

    pub(crate) unsafe fn as_entry_mut(&mut self) -> &mut Entry<T> {
        debug_assert!(self.0.is_some());
        // Safety: Option is always `Some` when calling this, see above
        &mut *self.0.unwrap_unchecked()
    }

    pub(crate) unsafe fn as_entry(&self) -> &Entry<T> {
        debug_assert!(self.0.is_some());
        // Safety: Option is always `Some` when calling this, see above
        &*self.0.unwrap_unchecked()
    }

    pub(crate) unsafe fn take_entry(&mut self) -> &mut Entry<T> {
        debug_assert!(self.0.is_some());
        &mut *self.0.take().unwrap_unchecked()
    }

    pub(crate) unsafe fn manually_drop(&mut self) -> &mut Entry<T> {
        ManuallyDrop::drop(&mut self.as_entry_mut().data);
        &mut *self.0.take().unwrap_unchecked()
    }

    pub(crate) unsafe fn take(&mut self) -> T {
        debug_assert!(self.0.is_some());
        ManuallyDrop::take(&mut self.as_entry_mut().data)
    }
}

impl<T> Drop for UnsafeBox<T> {
    fn drop(&mut self) {
        if self.0.is_some() {
            // Safety: we just checked 'is_some()'
            unsafe {
                self.manually_drop();
            }
        }
    }
}

impl<T> Deref for UnsafeBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: Always contains a valid object when this function is callable, see above
        unsafe { &self.as_entry().data }
    }
}

impl<T> DerefMut for UnsafeBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: Always contains a valid object when this function is callable, see above
        unsafe { &mut self.as_entry_mut().data }
    }
}

impl<T> fmt::Debug for UnsafeBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_tuple("UnsafeBox")
            .field(&self.0.as_ref().map(|v| *v as *const Entry<T>))
            .finish()
    }
}
