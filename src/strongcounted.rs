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

/// A reference counted smart pointer for Pool allocated objects. This wraps Slots in a safe
/// way. Sc's need a Pool holding `ScInner<T>`, not `T`.  Sc's do not have a Weak
/// counterpart. When the Weak functionality is not required this can give a space advantage
/// on small objects.
pub struct Sc<'a, T> {
    slot: Slot<ScInner<T>>,
    pool: &'a Pool<ScInner<T>>,
}

impl<T> Sc<'_, T> {
    /// Associated function that returns the number of strong counters of this Sc.
    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        unsafe { this.slot.get_unchecked().strong_count.get() }
    }
}

impl<T> Clone for Sc<'_, T> {
    #[must_use]
    fn clone(&self) -> Self {
        unsafe {
            self.slot.get_unchecked().inc_strong();
            Self {
                slot: self.slot.copy(),
                pool: self.pool,
            }
        }
    }
}

impl<'a, T: Default> Pool<ScInner<T>> {
    /// Allocate a default initialized Sc from a Pool.
    #[inline]
    pub fn default_sc(&'a mut self) -> Sc<'a, T> {
        self.alloc_sc(T::default())
    }
}

impl<'a, T> Pool<ScInner<T>> {
    /// Allocate a Box from a Pool.
    #[inline]
    pub fn alloc_sc(&'a self, t: T) -> Sc<'a, T> {
        Sc {
            slot: self.alloc(ScInner::new(t)),
            pool: self,
        }
    }
}

impl<T> Drop for Sc<'_, T> {
    #[inline]
    fn drop(&mut self) {
        let mslot = unsafe { self.slot.get_mut_unchecked() };

        mslot.dec_strong();

        if mslot.strong_count.get() == 0 {
            unsafe {
                self.pool.free_by_ref(&mut self.slot);
            }
        }
    }
}

impl<T> Deref for Sc<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        unsafe { self.slot.get_unchecked().data.assume_init_ref() }
    }
}

impl<T> DerefMut for Sc<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { self.slot.get_mut_unchecked().data.assume_init_mut() }
    }
}

impl<T> Borrow<T> for Sc<'_, T> {
    #[inline]
    fn borrow(&self) -> &T {
        unsafe { self.slot.get_unchecked().data.assume_init_ref() }
    }
}

impl<T> BorrowMut<T> for Sc<'_, T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut_unchecked().data.assume_init_mut() }
    }
}

impl<T> AsRef<T> for Sc<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.slot.get_unchecked().data.assume_init_ref() }
    }
}

impl<T> AsMut<T> for Sc<'_, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut_unchecked().data.assume_init_mut() }
    }
}

impl<T: PartialEq> PartialEq for Sc<'_, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd> PartialOrd for Sc<'_, T> {
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

impl<T: Ord> Ord for Sc<'_, T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq> Eq for Sc<'_, T> {}

impl<T: Hash> Hash for Sc<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher> Hasher for Sc<'_, T> {
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

impl<T: fmt::Display> fmt::Display for Sc<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Sc<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Pointer for Sc<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

// TODO: better way to hide this from the api
#[allow(missing_docs)]
pub struct ScInner<T> {
    data: MaybeUninit<T>,
    strong_count: Cell<usize>,
}

impl<T> ScInner<T> {
    #[inline]
    fn new(data: T) -> Self {
        Self {
            data: MaybeUninit::new(data),
            strong_count: Cell::new(1),
        }
    }

    #[inline]
    fn inc_strong(&self) {
        self.strong_count.set(self.strong_count.get() + 1);
    }

    #[inline]
    fn dec_strong(&self) {
        self.strong_count.set(self.strong_count.get() - 1);
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let pool = Pool::new();
        let _mysc = pool.alloc_sc("Sc");
    }

    #[test]
    fn macro_test() {
        let pool = Pool::new();
        let mysc = pool.alloc_sc("Sc");
        assert_eq!(*mysc, "Sc");
    }

    #[test]
    fn clone() {
        let pool = Pool::new();
        let mysc1 = pool.alloc_sc("Sc");
        let mysc2 = mysc1.clone();
        let mysc3 = Sc::clone(&mysc2);

        assert_eq!(*mysc1, "Sc");
        assert_eq!(mysc1, mysc2);
        assert_eq!(mysc2, mysc3);
        assert_eq!(Sc::strong_count(&mysc3), 3);
    }

    #[test]
    fn deref_mut() {
        let pool = Pool::new();
        let mut mysc = pool.alloc_sc("Sc");
        *mysc = "Changed";
        assert_eq!(*mysc, "Changed");
    }
}
