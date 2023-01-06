use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::*;

/// A fragile Box implementation which may leak memory within a Pool.  To free the memory of a
/// `BasicBox` it should eventually be given back to the pool it belongs to by
/// `BasicBox::drop()`, `BasicBox::forget()` or `BasicBox::take()`. A `BasicBox` do not track
/// which Pool they belong to. It is the responsibility of the user to give them back to the
/// correct pool. Unlike `UnsafeBox` a `BasicBox` has a lifetime bound to its pool, thus it
/// can not outlive the pool and no UB can happen when it becomes dropped.
///
/// When a `BasicBox` goes out of scope while it is not explicitly deallocated its contents
/// will be properly destructed while the associated memory will leak within the `Pool` from
/// where it was allocated. This happens especially when panicking drops unsafe boxes.
///
/// Sometimes can be used as advantage when using temporary pools where the memory reclamation
/// will happen when the `Pool` becomes destroyed.
#[repr(transparent)]
pub struct BasicBox<'a, T>(UnsafeBox<T>, PhantomData<&'a Pool<T>>);

unsafe impl<T: Send> Send for BasicBox<'_, T> {}
unsafe impl<T: Sync> Sync for BasicBox<'_, T> {}

impl<'a, T> BasicBox<'a, T> {
    /// Creates a new `BasicBox` from within the given pool.
    pub fn new(from: T, pool: &'a Pool<T>) -> Self {
        Self(pool.alloc(from), PhantomData)
    }

    /// Deallocates a `BasicBox`. A `BasicBox` that is not deallocated when it goes out of
    /// scope will leak within its pool.
    ///
    /// # Panics
    ///
    /// This `BasicBox` was not allocated from the given pool.
    pub fn drop(this: Self, pool: &'a Pool<T>) {
        pool.dealloc(this.0);
    }

    /// Deallocates a `BasicBox`. A `BasicBox` that is not deallocated when it goes out of
    /// scope will leak within its pool.
    ///
    /// # Safety
    ///
    /// This `BasicBox` must be allocated from the given pool.
    pub unsafe fn drop_unchecked(this: Self, pool: &'a Pool<T>) {
        pool.dealloc_unchecked(this.0);
    }

    /// Deallocates a `BasicBox` and returns its contained value.
    ///
    /// # Panics
    ///
    /// This `BasicBox` was not allocated from the given pool.
    pub fn take(this: Self, pool: &'a Pool<T>) -> T {
        pool.take(this.0)
    }

    /// Deallocates a `BasicBox` without calling its destructor. A `BasicBox` that is not
    /// deallocated when it goes out of scope will leak within its pool.
    ///
    /// # Panics
    ///
    /// This `BasicBox` was not allocated from the given pool.
    pub fn forget(this: Self, pool: &'a Pool<T>) {
        pool.forget(this.0);
    }
}

impl<'a, T: Default> BasicBox<'a, T> {
    /// Creates a new default initialized `BasicBox` from within the given pool.
    pub fn default(pool: &'a Pool<T>) -> Self {
        Self(pool.alloc(T::default()), PhantomData)
    }
}

impl<T> Deref for BasicBox<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for BasicBox<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Borrow<T> for BasicBox<'_, T> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> BorrowMut<T> for BasicBox<'_, T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> AsRef<T> for BasicBox<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for BasicBox<'_, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: PartialEq> PartialEq for BasicBox<'_, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd> PartialOrd for BasicBox<'_, T> {
    partial_ord_impl! {}
}

impl<T: Ord> Ord for BasicBox<'_, T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq> Eq for BasicBox<'_, T> {}

impl<T: Hash> Hash for BasicBox<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher> Hasher for BasicBox<'_, T> {
    hasher_impl! {}
}

impl<T: fmt::Display> fmt::Display for BasicBox<'_, T> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T> fmt::Debug for BasicBox<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_tuple("BasicBox").field(&self.0).finish()
    }
}

impl<T> fmt::Pointer for BasicBox<'_, T> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}
