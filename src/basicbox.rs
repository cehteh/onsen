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
/// will happen when the `Pool` becomes dropped.
#[repr(transparent)]
pub struct BasicBox<'a, T, P: PoolApi<Entry = ThinPoolEntry<T>>>(
    UnsafeBox<P::Entry>,
    PhantomData<&'a P>,
);

unsafe impl<T: Send, P: PoolApi<Entry = ThinPoolEntry<T>>> Send for BasicBox<'_, T, P> {}
unsafe impl<T: Sync, P: PoolApi<Entry = ThinPoolEntry<T>>> Sync for BasicBox<'_, T, P> {}

impl<'a, T, P: PoolApi<Entry = ThinPoolEntry<T>>> BasicBox<'a, T, P> {
    /// Creates a new `BasicBox` from within the given pool.
    pub fn new(from: T, pool: &P) -> Self {
        Self(pool.alloc(ThinPoolEntry(from)), PhantomData)
    }

    /// Deallocates a `BasicBox`. A `BasicBox` that is not deallocated when it goes out of
    /// scope will leak within its pool.
    ///
    /// # Panics
    ///
    /// This `BasicBox` was not allocated from the given pool.
    pub fn drop(this: Self, pool: &'a P) {
        pool.dealloc(this.0);
    }

    /// Deallocates a `BasicBox`. A `BasicBox` that is not deallocated when it goes out of
    /// scope will leak within its pool.
    ///
    /// # Safety
    ///
    /// This `BasicBox` must be allocated from the given pool.
    pub unsafe fn drop_unchecked(this: Self, pool: &'a P) {
        pool.dealloc_unchecked(this.0);
    }

    /// Deallocates a `BasicBox` and returns its contained value.
    ///
    /// # Panics
    ///
    /// This `BasicBox` was not allocated from the given pool.
    pub fn into_inner(this: Self, pool: &'a P) -> T {
        pool.take(this.0).0
    }

    /// Deallocates a `BasicBox` without calling its destructor. A `BasicBox` that is not
    /// deallocated when it goes out of scope will leak within its pool.
    ///
    /// # Panics
    ///
    /// This `BasicBox` was not allocated from the given pool.
    pub fn forget(this: Self, pool: &'a P) {
        pool.forget(this.0);
    }
}

impl<'a, T: Default, P: PoolApi<Entry = ThinPoolEntry<T>>> BasicBox<'a, T, P> {
    /// Creates a new default initialized `BasicBox` from within the given pool.
    pub fn default(pool: &'a P) -> Self {
        Self(pool.alloc(ThinPoolEntry(T::default())), PhantomData)
    }
}

impl<T, P: PoolApi<Entry = ThinPoolEntry<T>>> Deref for BasicBox<'_, T, P> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0 .0
    }
}

impl<T, P: PoolApi<Entry = ThinPoolEntry<T>>> DerefMut for BasicBox<'_, T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0 .0
    }
}

impl<T, P: PoolApi<Entry = ThinPoolEntry<T>>> Borrow<T> for BasicBox<'_, T, P> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.0 .0
    }
}

impl<T, P: PoolApi<Entry = ThinPoolEntry<T>>> BorrowMut<T> for BasicBox<'_, T, P> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0 .0
    }
}

impl<T, P: PoolApi<Entry = ThinPoolEntry<T>>> AsRef<T> for BasicBox<'_, T, P> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0 .0
    }
}

impl<T, P: PoolApi<Entry = ThinPoolEntry<T>>> AsMut<T> for BasicBox<'_, T, P> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0 .0
    }
}

impl<T: PartialEq, P: PoolApi<Entry = ThinPoolEntry<T>>> PartialEq for BasicBox<'_, T, P> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd, P: PoolApi<Entry = ThinPoolEntry<T>>> PartialOrd for BasicBox<'_, T, P> {
    partial_ord_impl! {}
}

impl<T: Ord, P: PoolApi<Entry = ThinPoolEntry<T>>> Ord for BasicBox<'_, T, P> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq, P: PoolApi<Entry = ThinPoolEntry<T>>> Eq for BasicBox<'_, T, P> {}

impl<T: Hash, P: PoolApi<Entry = ThinPoolEntry<T>>> Hash for BasicBox<'_, T, P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher, P: PoolApi<Entry = ThinPoolEntry<T>>> Hasher for BasicBox<'_, T, P> {
    hasher_impl! {}
}

impl<T: fmt::Display, P: PoolApi<Entry = ThinPoolEntry<T>>> fmt::Display for BasicBox<'_, T, P> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T, P: PoolApi<Entry = ThinPoolEntry<T>>> fmt::Debug for BasicBox<'_, T, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_tuple("BasicBox").field(&self.0).finish()
    }
}

impl<T, P: PoolApi<Entry = ThinPoolEntry<T>>> fmt::Pointer for BasicBox<'_, T, P> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}
