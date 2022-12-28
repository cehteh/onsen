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

/// A reference counted smart pointer for pool allocated objects. This wraps `SimpleBox` in a
/// safe way. Rc's need a `RcPool<RcInner<T>>` as pool.
pub struct Rc<T> {
    slot: SimpleBox<RcInner<T>, Mutable>,
    pool: RcPool<RcInner<T>>,
}

impl<T> Rc<T> {
    /// Allocate a `Rc` from a `RcPool`.
    ///
    /// ```
    /// use onsen::*;
    ///
    /// let pool = RcPool::new();
    /// let myrc = Rc::new("ReferenceCounted", &pool);
    ///
    /// // allocate from the same pool
    /// let otherrc = Rc::new("ReferenceCounted", &myrc);
    /// ```
    #[inline]
    pub fn new(t: T, pool: impl AsRef<RcPool<RcInner<T>>>) -> Self {
        Self {
            slot: pool.as_ref().alloc(RcInner::new(t)).for_mutation(),
            pool: pool.as_ref().clone(),
        }
    }

    /// Creates a Weak reference from a Rc.
    #[must_use]
    pub fn downgrade(this: &Self) -> Weak<T> {
        this.slot.get().inc_weak();
        unsafe {
            Weak::<T> {
                slot: this.slot.copy(),
                pool: this.pool.clone(),
            }
        }
    }

    /// Associated function that returns the number of strong counters of this Rc.
    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        this.slot.get().strong_count.get()
    }

    /// Associated function that returns the number of weak counters of this Rc.
    #[must_use]
    pub fn weak_count(this: &Self) -> usize {
        this.slot.get().weak_count.get()
    }
}

impl<T: Default> Rc<T> {
    /// Allocate a default initialized `Rc` from a pool.
    #[inline]
    pub fn default(pool: impl AsRef<RcPool<RcInner<T>>>) -> Self {
        Rc::new(T::default(), pool)
    }
}

impl<T> Clone for Rc<T> {
    #[must_use]
    fn clone(&self) -> Self {
        self.slot.get().inc_strong();
        unsafe {
            Self {
                slot: self.slot.copy(),
                pool: self.pool.clone(),
            }
        }
    }
}

impl<T> Drop for Rc<T> {
    #[inline]
    fn drop(&mut self) {
        let mslot = self.slot.get_mut();

        mslot.dec_strong();

        if mslot.strong_count.get() == 0 {
            if mslot.weak_count.get() == 0 {
                // no references exist, can be freed completely
                unsafe {
                    self.pool.free_by_ref(&mut self.slot);
                }
            } else {
                // only weak references exist, drop in place
                unsafe {
                    mslot.data.assume_init_drop();
                }
            }
        }
    }
}

impl<T> Deref for Rc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> DerefMut for Rc<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T> Borrow<T> for Rc<T> {
    #[inline]
    fn borrow(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> BorrowMut<T> for Rc<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T> AsRef<T> for Rc<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> AsMut<T> for Rc<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T: PartialEq> PartialEq for Rc<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd> PartialOrd for Rc<T> {
    partial_ord_impl! {}
}

impl<T: Ord> Ord for Rc<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq> Eq for Rc<T> {}

impl<T: Hash> Hash for Rc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher> Hasher for Rc<T> {
    hasher_impl! {}
}

impl<T: fmt::Display> fmt::Display for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Pointer for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// `Weak` references do not keep the object alive.
pub struct Weak<T> {
    slot: SimpleBox<RcInner<T>, Mutable>,
    pool: RcPool<RcInner<T>>,
}

impl<T> Weak<T> {
    /// Associated function that returns the number of strong counters of this Weak.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.slot.get().strong_count.get()
    }

    /// Associated function that returns the number of weak counters of this Weak.
    #[must_use]
    pub fn weak_count(&self) -> usize {
        self.slot.get().weak_count.get()
    }
}

impl<T> Weak<T> {
    /// Tries to create a Rc from a Weak reference. Fails when the strong count was zero.
    #[must_use]
    pub fn upgrade(&self) -> Option<Rc<T>> {
        if self.strong_count() > 0 {
            self.slot.get().inc_strong();
            unsafe {
                Some(Rc::<T> {
                    slot: self.slot.copy(),
                    pool: self.pool.clone(),
                })
            }
        } else {
            None
        }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        self.slot.get().inc_weak();
        unsafe {
            Self {
                slot: self.slot.copy(),
                pool: self.pool.clone(),
            }
        }
    }
}

impl<T> Drop for Weak<T> {
    #[inline]
    fn drop(&mut self) {
        let mslot = self.slot.get_mut();
        mslot.dec_weak();

        if mslot.strong_count.get() == 0 {
            if mslot.weak_count.get() == 0 {
                // no references exist, can be freed completely
                unsafe {
                    self.pool.free_by_ref(&mut self.slot);
                }
            } else {
                // only weak references exist, drop in place
                unsafe {
                    mslot.data.assume_init_drop();
                }
            }
        }
    }
}

/// Data including reference counters
pub struct RcInner<T> {
    pub(crate) data: MaybeUninit<T>,
    strong_count: Cell<usize>,
    weak_count: Cell<usize>,
}

impl<T> RcInner<T> {
    #[inline]
    pub(crate) fn new(data: T) -> Self {
        Self {
            data: MaybeUninit::new(data),
            strong_count: Cell::new(1),
            weak_count: Cell::new(0),
        }
    }

    #[inline]
    pub(crate) fn get_strong(&self) -> usize {
        self.strong_count.get()
    }

    #[inline]
    pub(crate) fn get_weak(&self) -> usize {
        self.weak_count.get()
    }

    #[inline]
    pub(crate) fn inc_strong(&self) {
        self.strong_count.set(self.strong_count.get() + 1);
    }

    #[inline]
    pub(crate) fn dec_strong(&self) {
        self.strong_count.set(self.strong_count.get() - 1);
    }

    #[inline]
    pub(crate) fn inc_weak(&self) {
        self.weak_count.set(self.weak_count.get() + 1);
    }

    #[inline]
    pub(crate) fn dec_weak(&self) {
        self.weak_count.set(self.weak_count.get() - 1);
    }
}

/// Get a reference to the pool this `Rc` was constructed from.
impl<T> AsRef<RcPool<RcInner<T>>> for Rc<T> {
    fn as_ref(&self) -> &RcPool<RcInner<T>> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let pool = RcPool::new();
        let _myrc = Rc::new("Rc", &pool);
    }
}
