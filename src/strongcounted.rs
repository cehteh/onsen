use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::*;

/// A reference counted smart pointer for pool allocated objects. This wraps Slots in a safe
/// way. Sc's need a `RcPool<ScInner<T>>` as backing pool.  Sc's do not have a Weak
/// counterpart. When no Weak functionality is required this can give a space advantage
/// for small objects and be slightly faster.
pub struct Sc<T> {
    slot: Slot<ScInner<T>, Mutable>,
    pool: RcPool<ScInner<T>>,
}

impl<T> Sc<T> {
    /// Allocate a `Sc` from a `RcPool`.
    #[inline]
    pub fn new(t: T, pool: impl AsRef<RcPool<ScInner<T>>>) -> Self {
        Self {
            slot: pool.as_ref().alloc(ScInner::new(t)).for_mutation(),
            pool: pool.as_ref().clone(),
        }
    }

    /// Associated function that returns the number of strong counters of this Sc.
    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        this.slot.get().strong_count.get()
    }
}

impl<T: Default> Sc<T> {
    /// Allocate a default initialized `Sc` from a pool.
    #[inline]
    pub fn default(pool: impl AsRef<RcPool<ScInner<T>>>) -> Self {
        Sc::new(T::default(), pool)
    }
}

impl<T> Clone for Sc<T> {
    #[must_use]
    fn clone(&self) -> Self {
        unsafe {
            self.slot.get().inc_strong();
            Self {
                slot: self.slot.copy(),
                pool: self.pool.clone(),
            }
        }
    }
}

impl<T> Drop for Sc<T> {
    #[inline]
    fn drop(&mut self) {
        let mslot = self.slot.get_mut();

        mslot.dec_strong();

        if mslot.strong_count.get() == 0 {
            unsafe {
                self.pool.free_by_ref(&mut self.slot);
            }
        }
    }
}

impl<T> Deref for Sc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> DerefMut for Sc<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T> Borrow<T> for Sc<T> {
    #[inline]
    fn borrow(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> BorrowMut<T> for Sc<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T> AsRef<T> for Sc<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> AsMut<T> for Sc<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T: PartialEq> PartialEq for Sc<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd> PartialOrd for Sc<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        PartialOrd::lt(&**self, &**other)
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        PartialOrd::le(&**self, &**other)
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        PartialOrd::ge(&**self, &**other)
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        PartialOrd::gt(&**self, &**other)
    }
}

impl<T: Ord> Ord for Sc<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq> Eq for Sc<T> {}

impl<T: Hash> Hash for Sc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher> Hasher for Sc<T> {
    hasher_impl! {}
}

impl<T: fmt::Display> fmt::Display for Sc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Sc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Pointer for Sc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// Data including reference counter
pub struct ScInner<T> {
    pub(crate) data: MaybeUninit<T>,
    strong_count: Cell<usize>,
}

impl<T> ScInner<T> {
    #[inline]
    pub(crate) fn new(data: T) -> Self {
        Self {
            data: MaybeUninit::new(data),
            strong_count: Cell::new(1),
        }
    }

    #[inline]
    pub(crate) fn get_strong(&self) -> usize {
        self.strong_count.get()
    }

    #[inline]
    pub(crate) fn inc_strong(&self) {
        self.strong_count.set(self.strong_count.get() + 1);
    }

    #[inline]
    pub(crate) fn dec_strong(&self) {
        self.strong_count.set(self.strong_count.get() - 1);
    }
}

/// Get a reference to the pool this `Sc` was constructed from.
impl<T> AsRef<RcPool<ScInner<T>>> for Sc<T> {
    fn as_ref(&self) -> &RcPool<ScInner<T>> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let pool = RcPool::new();
        let _mysc = Sc::new("Sc", &pool);
    }
}
