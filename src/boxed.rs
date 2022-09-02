use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::*;

/// A Box for pool allocated objects. This wraps Slots in a safe way. Dropping a Box will
/// ensure that the destructor is called and the memory is given back to the pool. Uses a `RcPool<T>`
/// to keep the backing pool alive as long any `Box<T>` is still in use.
pub struct Box<T> {
    slot: Slot<T, Mutable>,
    pool: RcPool<T>,
}

impl<T> Box<T> {
    /// Allocate a Box from a RcPool.
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
    /// Allocate a default initialized Box from a Pool.
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
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes);
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i);
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i);
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i);
    }
    fn write_u64(&mut self, i: u64) {
        (**self).write_u64(i);
    }
    fn write_u128(&mut self, i: u128) {
        (**self).write_u128(i);
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i);
    }
    fn write_i8(&mut self, i: i8) {
        (**self).write_i8(i);
    }
    fn write_i16(&mut self, i: i16) {
        (**self).write_i16(i);
    }
    fn write_i32(&mut self, i: i32) {
        (**self).write_i32(i);
    }
    fn write_i64(&mut self, i: i64) {
        (**self).write_i64(i);
    }
    fn write_i128(&mut self, i: i128) {
        (**self).write_i128(i);
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i);
    }
    // fn write_length_prefix(&mut self, len: usize) {
    //     (**self).write_length_prefix(len)
    // }
    // fn write_str(&mut self, s: &str) {
    //     (**self).write_str(s)
    // }
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
