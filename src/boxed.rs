use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::*;

// PLANNED: assoc_static pool

/// A Box for Pool allocated objects. This wraps Slots in a safe way. Dropping a Box will
/// ensure that the destructor is called and the memory is given back to the pool.
pub struct Box<'a, T> {
    slot: Slot<T, Mutable>,
    pool: &'a Pool<T>,
}

impl<T> Box<'_, T> {
    /// Associated function that frees the memory of a Box without calling the destructor of
    /// its value.
    #[inline]
    pub fn forget(mut b: Self) {
        unsafe { b.pool.forget_by_ref(&mut b.slot) }
    }

    /// Associated function that frees the memory of a Box and returns the value it was holding.
    #[inline]
    #[must_use]
    pub fn take(mut b: Self) -> T {
        unsafe { b.pool.take_by_ref(&mut b.slot) }
    }
}

impl<'a, T: Default> Pool<T> {
    /// Allocate a default initialized Box from a Pool.
    #[inline]
    pub fn default_box(&'a self) -> Box<'a, T> {
        self.alloc_box(T::default())
    }
}

impl<'a, T> Pool<T> {
    /// Allocate a Box from a Pool.
    #[inline]
    pub fn alloc_box(&'a self, t: T) -> Box<'a, T> {
        Box {
            slot: self.alloc(t).for_mutation(),
            pool: self,
        }
    }
}

impl<T> Drop for Box<'_, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.pool.free_by_ref(&mut self.slot);
        }
    }
}

impl<T> Deref for Box<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        self.slot.get()
    }
}

impl<T> DerefMut for Box<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        self.slot.get_mut()
    }
}

impl<T> Borrow<T> for Box<'_, T> {
    #[inline]
    fn borrow(&self) -> &T {
        self.slot.get()
    }
}

impl<T> BorrowMut<T> for Box<'_, T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        self.slot.get_mut()
    }
}

impl<T> AsRef<T> for Box<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.slot.get()
    }
}

impl<T> AsMut<T> for Box<'_, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.slot.get_mut()
    }
}

impl<T: PartialEq> PartialEq for Box<'_, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd> PartialOrd for Box<'_, T> {
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

impl<T: Ord> Ord for Box<'_, T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq> Eq for Box<'_, T> {}

impl<T: Hash> Hash for Box<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher> Hasher for Box<'_, T> {
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

impl<T: fmt::Display> fmt::Display for Box<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Box<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Pointer for Box<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let pool: Pool<&str> = Pool::new();
        let _mybox = pool.alloc_box("Boxed");
    }

    #[test]
    fn deref() {
        let pool: Pool<&str> = Pool::new();
        let mybox = pool.alloc_box("Boxed");
        assert_eq!(*mybox, "Boxed");
    }

    #[test]
    fn deref_mut() {
        let pool: Pool<&str> = Pool::new();
        let mut mybox = pool.alloc_box("Boxed");
        *mybox = "Changed";
        assert_eq!(*mybox, "Changed");
    }

    #[test]
    fn eq() {
        let pool: Pool<&str> = Pool::new();
        let box1 = pool.alloc_box("Boxed");
        let box2 = pool.alloc_box("Boxed");
        let box3 = pool.alloc_box("Boxed again");
        assert_eq!(box1, box2);
        assert_ne!(box1, box3);
    }
}
