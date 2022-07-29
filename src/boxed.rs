//! onsen::Box is WIP! many trait implementations of the std::boxed::Box are still missing.
use std::ops::Deref;
use std::ops::DerefMut;

use crate::*;

/// A Box for Pool allocated objects. This wraps Slots in a safe way. Dropping a Box will
/// ensure that the destructor is called and the memory is given back to the pool.
pub struct Box<'a, T, const E: usize> {
    slot: Slot<T>,
    pool: &'a mut Pool<T, E>,
}

impl<'a, T, const E: usize> Pool<T, E> {
    /// Allocate a Box from a Pool.
    pub fn alloc_box(&'a mut self, t: T) -> Box<'a, T, E> {
        Box {
            slot: self.alloc(t),
            pool: self,
        }
    }
}

impl<T, const E: usize> Drop for Box<'_, T, E> {
    fn drop(&mut self) {
        unsafe {
            self.pool.free_by_ref(&self.slot);
        }
    }
}

impl<T, const E: usize> Deref for Box<'_, T, E> {
    type Target = T;

    fn deref(&self) -> &<Self as Deref>::Target {
        self.slot.get()
    }
}

impl<T, const E: usize> DerefMut for Box<'_, T, E> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
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
