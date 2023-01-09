use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::*;

/// A Box for pool allocated objects. This wraps `UnsafeBox` in a safe way. Dropping a Box will
/// ensure that the destructor is called and the memory is given back to the pool.
pub struct Box<T, P: SharedPoolApi<T>> {
    slot: UnsafeBox<T>,
    pool: P,
}

impl<T, P: SharedPoolApi<T>> AsSharedPool<T, P> for Box<T, P> {
    #[inline]
    fn as_shared_pool(&self) -> &P {
        &self.pool
    }
}

impl<T, P: SharedPoolApi<T>> Box<T, P> {
    /// Creates a new `Box` containing the supplied value. The `Box` can be created from
    /// anything that can act as a pool. These are shared pools themselves as well as any
    /// other Box.
    #[inline]
    pub fn new(value: T, aspool: &impl AsSharedPool<T, P>) -> Self {
        let pool = aspool.as_shared_pool();
        Self {
            slot: pool.alloc(value),
            pool: pool.clone(),
        }
    }

    /// Associated function that frees the memory of a Box without calling the destructor of
    /// its value.
    #[inline]
    pub fn forget(mut this: Self) {
        std::mem::forget(unsafe { this.slot.take() });
    }

    /// Associated function that frees the memory of a Box and returns the value it was holding.
    #[inline]
    #[must_use]
    pub fn into_inner(mut this: Self) -> T {
        unsafe { this.slot.take() }
    }
}

impl<T: Default, P: SharedPoolApi<T>> Box<T, P> {
    /// Allocate a default initialized `Box` from a pool.
    #[inline]
    #[must_use]
    pub fn default(pool: &P) -> Self {
        Self {
            slot: pool.alloc(T::default()),
            pool: pool.clone(),
        }
    }
}

impl<T, P: SharedPoolApi<T>> Drop for Box<T, P> {
    #[inline]
    fn drop(&mut self) {
        // Safety: Boxes always refer the pool they where created from
        unsafe {
            self.pool
                .with_lock(|pool| pool.fast_free_entry_unchecked(self.slot.manually_drop()));
        }
    }
}

impl<T, P: SharedPoolApi<T>> Deref for Box<T, P> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        &self.slot
    }
}

impl<T, P: SharedPoolApi<T>> DerefMut for Box<T, P> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.slot
    }
}

impl<T, P: SharedPoolApi<T>> Borrow<T> for Box<T, P> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.slot
    }
}

impl<T, P: SharedPoolApi<T>> BorrowMut<T> for Box<T, P> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.slot
    }
}

impl<T, P: SharedPoolApi<T>> AsRef<T> for Box<T, P> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.slot
    }
}

impl<T, P: SharedPoolApi<T>> AsMut<T> for Box<T, P> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.slot
    }
}

impl<T: PartialEq, P: SharedPoolApi<T>> PartialEq for Box<T, P> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd, P: SharedPoolApi<T>> PartialOrd for Box<T, P> {
    partial_ord_impl! {}
}

impl<T: Ord, P: SharedPoolApi<T>> Ord for Box<T, P> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq, P: SharedPoolApi<T>> Eq for Box<T, P> {}

impl<T: Hash, P: SharedPoolApi<T>> Hash for Box<T, P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher, P: SharedPoolApi<T>> Hasher for Box<T, P> {
    hasher_impl! {}
}

impl<T: fmt::Display, P: SharedPoolApi<T>> fmt::Display for Box<T, P> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug, P: SharedPoolApi<T>> fmt::Debug for Box<T, P> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, P: SharedPoolApi<T>> fmt::Pointer for Box<T, P> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}
