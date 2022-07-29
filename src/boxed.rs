//! onsen::Box is WIP! many trait implementations of the std::boxed::Box are still missing.
use std::borrow::Borrow;
use std::borrow::BorrowMut;
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
