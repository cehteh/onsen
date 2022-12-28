use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::*;

/// A Box for pool allocated objects. This wraps `SimpleBox` in a safe way. Dropping a Box will
/// ensure that the destructor is called and the memory is given back to the pool. Uses a `RcPool<T>`
/// to keep the backing pool alive as long any `Box<T>` is still in use.
pub struct Box<T> {
    slot: SimpleBox<T, Mutable>,
    pool: RcPool<T>,
}

impl<T> Box<T> {
    /// Allocate a Box from a `RcPool`.
    ///
    /// ```
    /// use onsen::*;
    ///
    /// let pool: RcPool<&str> = RcPool::new();
    /// let mybox = Box::new("Boxed", &pool);
    ///
    /// // allocate from the same pool
    /// let otherbox = Box::new("Boxed", &mybox);
    /// ```
    #[inline]
    pub fn new(t: T, pool: impl AsRef<RcPool<T>>) -> Self {
        Self {
            slot: pool.as_ref().alloc(t).for_mutation(),
            pool: pool.as_ref().clone(),
        }
    }

    /// Associated function that frees the memory of a Box without calling the destructor of
    /// its value.
    #[inline]
    pub fn forget(mut this: Self) {
        unsafe { this.pool.forget_by_ref(&mut this.slot) }
    }

    /// Associated function that frees the memory of a Box and returns the value it was holding.
    #[inline]
    #[must_use]
    pub fn take(mut this: Self) -> T {
        unsafe { this.pool.take_by_ref(&mut this.slot) }
    }
}

impl<T: Default> Box<T> {
    /// Allocate a default initialized `Box` from a pool.
    #[inline]
    #[must_use]
    pub fn default(pool: impl AsRef<RcPool<T>>) -> Self {
        Box::new(T::default(), pool)
    }
}

impl<T> Drop for Box<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.pool.free_by_ref(&mut self.slot);
        }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        self.slot.get()
    }
}

impl<T> DerefMut for Box<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        self.slot.get_mut()
    }
}

impl<T> Borrow<T> for Box<T> {
    #[inline]
    fn borrow(&self) -> &T {
        self.slot.get()
    }
}

impl<T> BorrowMut<T> for Box<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        self.slot.get_mut()
    }
}

impl<T> AsRef<T> for Box<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.slot.get()
    }
}

impl<T> AsMut<T> for Box<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.slot.get_mut()
    }
}

impl<T: PartialEq> PartialEq for Box<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd> PartialOrd for Box<T> {
    partial_ord_impl! {}
}

impl<T: Ord> Ord for Box<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq> Eq for Box<T> {}

impl<T: Hash> Hash for Box<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher> Hasher for Box<T> {
    hasher_impl! {}
}

impl<T: fmt::Display> fmt::Display for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Pointer for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// Get a reference to the pool this `Box` was constructed from.
impl<T> AsRef<RcPool<T>> for Box<T> {
    fn as_ref(&self) -> &RcPool<T> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let pool: RcPool<&str> = RcPool::new();
        let _mybox = Box::new("Boxed", &pool);
    }
}
