use crate::*;
use erasable::*;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;

use std::ops::Deref;
use std::ops::DerefMut;

// PLANNED: FatBox

/// A Box for pool allocated objects. This wraps `UnsafeBox` in a safe way. Dropping a Box will
/// ensure that the destructor is called and the memory is given back to the pool.
pub struct Box<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>>(UnsafeBox<P::Entry>);

unsafe impl<T: Send, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Send for Box<T, P> {}
unsafe impl<T: Sync, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Sync for Box<T, P> {}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Box<T, P> {
    /// Creates a new `Box` containing the supplied value. The `Box` can be created from
    /// anything that can act as a pool. These are shared pools themselves as well as any
    /// other Box.
    #[inline]
    pub fn new<C: CloneSharedPool<Pool = P>>(value: T, aspool: &C) -> Self {
        let pool = aspool.clone_shared_pool();
        Self(pool.alloc(FatPoolEntry {
            slot: value,
            pool: ErasablePtr::erase(pool.clone()),
        }))
    }

    /// Associated function that frees the memory of a Box without calling the destructor of
    /// its value.
    #[inline]
    pub fn forget(mut this: Self) {
        std::mem::forget(unsafe { this.0.take() });
    }

    /// Associated function that frees the memory of a Box and returns the value it was holding.
    #[inline]
    #[must_use]
    pub fn into_inner(mut this: Self) -> T {
        unsafe { this.0.take().slot }
    }
}

impl<T: Default, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Box<T, P> {
    /// Allocate a default initialized `Box` from a pool.
    #[inline]
    #[must_use]
    pub fn default<C: CloneSharedPool<Pool = P>>(aspool: &C) -> Self {
        let pool = aspool.clone_shared_pool();
        Self(pool.alloc(FatPoolEntry {
            slot: T::default(),
            pool: ErasablePtr::erase(pool.clone()),
        }))
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Drop for Box<T, P> {
    #[inline]
    fn drop(&mut self) {
        // Safety: Boxes always refer the pool they where created from
        unsafe {
            <P as ErasablePtr>::unerase(self.0.pool)
                .with_lock(|pool| pool.fast_free_entry_unchecked(self.0.manually_drop()));
        }
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Deref for Box<T, P> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        &self.0.slot
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> DerefMut for Box<T, P> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.0.slot
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Borrow<T> for Box<T, P> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.0.slot
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> BorrowMut<T> for Box<T, P> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0.slot
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> AsRef<T> for Box<T, P> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0.slot
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> AsMut<T> for Box<T, P> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0.slot
    }
}

impl<T: PartialEq, P: SharedPoolApi<Entry = FatPoolEntry<T>>> PartialEq for Box<T, P> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd, P: SharedPoolApi<Entry = FatPoolEntry<T>>> PartialOrd for Box<T, P> {
    partial_ord_impl! {}
}

impl<T: Ord, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Ord for Box<T, P> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Eq for Box<T, P> {}

impl<T: Hash, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Hash for Box<T, P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher, P: SharedPoolApi<Entry = FatPoolEntry<T>>> Hasher for Box<T, P> {
    hasher_impl! {}
}

impl<T: fmt::Display, P: SharedPoolApi<Entry = FatPoolEntry<T>>> fmt::Display for Box<T, P> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug, P: SharedPoolApi<Entry = FatPoolEntry<T>>> fmt::Debug for Box<T, P> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> fmt::Pointer for Box<T, P> {
    #[mutants::skip] /* we just pretend it works */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

impl<T, P: SharedPoolApi<Entry = FatPoolEntry<T>>> CloneSharedPool for Box<T, P> {
    type Pool = P;
    #[inline]
    fn clone_shared_pool(&self) -> P {
        unsafe { self.0.pool.with(|pool: &P| pool.clone()) }
    }
}
