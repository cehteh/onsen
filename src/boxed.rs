//! onsen::Box is WIP! many trait implementations of the std::boxed::Box are still missing.
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::*;

/// A Box for Pool allocated objects. This wraps Slots in a safe way. Dropping a Box will
/// ensure that the destructor is called and the memory is given back to the pool.
pub struct Box<'a, T, const E: usize> {
    slot: Slot<T>,
    pool: &'a mut Pool<T, E>,
}

impl<T, const E: usize> Box<'_, T, E> {
    /// Associated function that frees the memory of a Box without calling the destructor of
    /// its value.
    #[inline]
    pub fn forget(b: Self) {
        unsafe { b.pool.forget_by_ref(&b.slot) }
    }

    /// Associated function that frees the memory of a Box and returns the value it was holding.
    #[inline]
    pub fn take(b: Self) -> T {
        unsafe { b.pool.take_by_ref(&b.slot) }
    }
}

impl<'a, T: Default, const E: usize> Pool<T, E> {
    /// Allocate a default initialized Box from a Pool.
    #[inline]
    pub fn default_box(&'a mut self) -> Box<'a, T, E> {
        self.alloc_box(T::default())
    }
}

impl<'a, T, const E: usize> Pool<T, E> {
    /// Allocate a Box from a Pool.
    #[inline]
    pub fn alloc_box(&'a mut self, t: T) -> Box<'a, T, E> {
        Box {
            slot: self.alloc(t),
            pool: self,
        }
    }
}

impl<T, const E: usize> Drop for Box<'_, T, E> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.pool.free_by_ref(&self.slot);
        }
    }
}

impl<T, const E: usize> Deref for Box<'_, T, E> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        self.slot.get()
    }
}

impl<T, const E: usize> DerefMut for Box<'_, T, E> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        self.slot.get_mut()
    }
}

impl<T, const E: usize> Borrow<T> for Box<'_, T, E> {
    #[inline]
    fn borrow(&self) -> &T {
        self.slot.get()
    }
}

impl<T, const E: usize> BorrowMut<T> for Box<'_, T, E> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        self.slot.get_mut()
    }
}

impl<T, const E: usize> AsRef<T> for Box<'_, T, E> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.slot.get()
    }
}

impl<T, const E: usize> AsMut<T> for Box<'_, T, E> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.slot.get_mut()
    }
}

impl<T: PartialEq, const E: usize> PartialEq for Box<'_, T, E> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
    #[inline]
    fn ne(&self, other: &Self) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}

impl<T: PartialOrd, const E: usize> PartialOrd for Box<'_, T, E> {
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

impl<T: Ord, const E: usize> Ord for Box<'_, T, E> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq, const E: usize> Eq for Box<'_, T, E> {}

impl<T: Hash, const E: usize> Hash for Box<'_, T, E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher, const E: usize> Hasher for Box<'_, T, E> {
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i)
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i)
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i)
    }
    fn write_u64(&mut self, i: u64) {
        (**self).write_u64(i)
    }
    fn write_u128(&mut self, i: u128) {
        (**self).write_u128(i)
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i)
    }
    fn write_i8(&mut self, i: i8) {
        (**self).write_i8(i)
    }
    fn write_i16(&mut self, i: i16) {
        (**self).write_i16(i)
    }
    fn write_i32(&mut self, i: i32) {
        (**self).write_i32(i)
    }
    fn write_i64(&mut self, i: i64) {
        (**self).write_i64(i)
    }
    fn write_i128(&mut self, i: i128) {
        (**self).write_i128(i)
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i)
    }
    // fn write_length_prefix(&mut self, len: usize) {
    //     (**self).write_length_prefix(len)
    // }
    // fn write_str(&mut self, s: &str) {
    //     (**self).write_str(s)
    // }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let mut pool: Pool<&str, 128> = Pool::new();
        let _mybox = pool.alloc_box("Boxed");
    }

    #[test]
    fn deref() {
        let mut pool: Pool<&str, 128> = Pool::new();
        let mybox = pool.alloc_box("Boxed");
        assert_eq!(*mybox, "Boxed");
    }

    #[test]
    fn deref_mut() {
        let mut pool: Pool<&str, 128> = Pool::new();
        let mut mybox = pool.alloc_box("Boxed");
        *mybox = "Changed";
        assert_eq!(*mybox, "Changed");
    }
}
