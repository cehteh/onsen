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
/// way. Rc's need a Pool holding `RcInner<T>`, not `T`.
pub struct Rc<'a, T, const E: usize> {
    slot: Slot<RcInner<T>>,
    pool: &'a Pool<RcInner<T>, E>,
}

impl<T, const E: usize> Rc<'_, T, E> {
    /// Associated function that returns the number of strong counters of this Rc.
    pub fn strong_count(this: &Self) -> usize {
        this.slot.get().strong_count.get()
    }

    /// Associated function that returns the number of weak counters of this Rc.
    pub fn weak_count(this: &Self) -> usize {
        this.slot.get().weak_count.get()
    }
}

impl<'a, T, const E: usize> Rc<'a, T, E> {
    /// Creates a Weak reference from a Rc.
    pub fn downgrade(this: &Self) -> Weak<'a, T, E> {
        this.slot.get().inc_weak();
        Weak::<'a, T, E> {
            slot: unsafe { this.slot.copy() },
            pool: this.pool,
        }
    }
}

impl<T, const E: usize> Clone for Rc<'_, T, E> {
    fn clone(&self) -> Self {
        self.slot.get().inc_strong();
        Self {
            slot: unsafe { self.slot.copy() },
            pool: self.pool,
        }
    }
}

impl<'a, T: Default, const E: usize> Pool<RcInner<T>, E> {
    /// Allocate a default initialized Rc from a Pool.
    #[inline]
    pub fn default_rc(&'a mut self) -> Rc<'a, T, E> {
        self.alloc_rc(T::default())
    }
}

impl<'a, T, const E: usize> Pool<RcInner<T>, E> {
    /// Allocate a Box from a Pool.
    #[inline]
    pub fn alloc_rc(&'a self, t: T) -> Rc<'a, T, E> {
        Rc {
            slot: self.alloc(RcInner::new(t)),
            pool: self,
        }
    }
}

impl<T, const E: usize> Drop for Rc<'_, T, E> {
    #[inline]
    fn drop(&mut self) {
        self.slot.get().dec_strong();

        if self.slot.get_mut().strong_count.get() == 0 {
            if self.slot.get_mut().weak_count.get() == 0 {
                // no references exist, can be freed completely
                unsafe {
                    self.pool.free_by_ref(&self.slot);
                }
            } else {
                // only weak references exist, drop in place
                unsafe {
                    self.slot.get_mut().data.assume_init_drop();
                }
            }
        }
    }
}

impl<T, const E: usize> Deref for Rc<'_, T, E> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, const E: usize> DerefMut for Rc<'_, T, E> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T, const E: usize> Borrow<T> for Rc<'_, T, E> {
    #[inline]
    fn borrow(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, const E: usize> BorrowMut<T> for Rc<'_, T, E> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T, const E: usize> AsRef<T> for Rc<'_, T, E> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, const E: usize> AsMut<T> for Rc<'_, T, E> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T: PartialEq, const E: usize> PartialEq for Rc<'_, T, E> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd, const E: usize> PartialOrd for Rc<'_, T, E> {
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

impl<T: Ord, const E: usize> Ord for Rc<'_, T, E> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq, const E: usize> Eq for Rc<'_, T, E> {}

impl<T: Hash, const E: usize> Hash for Rc<'_, T, E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher, const E: usize> Hasher for Rc<'_, T, E> {
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

impl<T: fmt::Display, const E: usize> fmt::Display for Rc<'_, T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug, const E: usize> fmt::Debug for Rc<'_, T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, const E: usize> fmt::Pointer for Rc<'_, T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// Weak references do not keep the object alive.
pub struct Weak<'a, T, const E: usize> {
    slot: Slot<RcInner<T>>,
    pool: &'a Pool<RcInner<T>, E>,
}

impl<T, const E: usize> Weak<'_, T, E> {
    /// Associated function that returns the number of strong counters of this Weak.
    pub fn strong_count(&self) -> usize {
        self.slot.get().strong_count.get()
    }

    /// Associated function that returns the number of weak counters of this Weak.
    pub fn weak_count(&self) -> usize {
        self.slot.get().weak_count.get()
    }
}

impl<'a, T, const E: usize> Weak<'a, T, E> {
    /// Tries to create a Rc from a Weak reference. Fails when the strong count was zero.
    pub fn upgrade(&self) -> Option<Rc<'a, T, E>> {
        if self.strong_count() > 0 {
            self.slot.get().inc_strong();
            Some(Rc::<'a, T, E> {
                slot: unsafe { self.slot.copy() },
                pool: self.pool,
            })
        } else {
            None
        }
    }
}

impl<T, const E: usize> Clone for Weak<'_, T, E> {
    fn clone(&self) -> Self {
        self.slot.get().inc_weak();
        Self {
            slot: unsafe { self.slot.copy() },
            pool: self.pool,
        }
    }
}

impl<T, const E: usize> Drop for Weak<'_, T, E> {
    #[inline]
    fn drop(&mut self) {
        self.slot.get().dec_weak();

        if self.slot.get_mut().strong_count.get() == 0 {
            if self.slot.get_mut().weak_count.get() == 0 {
                // no references exist, can be freed completely
                unsafe {
                    self.pool.free_by_ref(&self.slot);
                }
            } else {
                // only weak references exist, drop in place
                unsafe {
                    self.slot.get_mut().data.assume_init_drop();
                }
            }
        }
    }
}

// TODO: better way to hide this from the api
#[allow(missing_docs)]
pub struct RcInner<T> {
    data: MaybeUninit<T>,
    strong_count: Cell<usize>,
    weak_count: Cell<usize>,
}

impl<T> RcInner<T> {
    #[inline]
    fn new(data: T) -> Self {
        Self {
            data: MaybeUninit::new(data),
            strong_count: Cell::new(1),
            weak_count: Cell::new(0),
        }
    }

    #[inline]
    fn inc_strong(&self) {
        self.strong_count
            .set(self.strong_count.get().wrapping_add(1));
    }

    #[inline]
    fn dec_strong(&self) {
        self.strong_count
            .set(self.strong_count.get().wrapping_sub(1));
    }

    #[inline]
    fn inc_weak(&self) {
        self.weak_count.set(self.weak_count.get().wrapping_add(1));
    }

    #[inline]
    fn dec_weak(&self) {
        self.weak_count.set(self.weak_count.get().wrapping_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let pool: Pool<_, 128> = Pool::new();
        let _myrc = pool.alloc_rc("Rc");
    }

    #[test]
    fn macro_test() {
        let pool = pool!(RcInner<&str>, PAGE);
        let myrc = pool.alloc_rc("Rc");
        assert_eq!(*myrc, "Rc");
    }

    #[test]
    fn clone() {
        let pool = pool!(RcInner<&str>, PAGE);
        let myrc1 = pool.alloc_rc("Rc");
        let myrc2 = myrc1.clone();
        let myrc3 = Rc::clone(&myrc2);

        assert_eq!(*myrc1, "Rc");
        assert_eq!(myrc1, myrc2);
        assert_eq!(myrc2, myrc3);
        assert_eq!(Rc::strong_count(&myrc3), 3);
    }

    #[test]
    fn deref_mut() {
        let pool = pool!(RcInner<&str>, PAGE);
        let mut myrc = pool.alloc_rc("Rc");
        *myrc = "Changed";
        assert_eq!(*myrc, "Changed");
    }

    #[test]
    fn weak() {
        let pool = pool!(RcInner<&str>, PAGE);
        let myrc = pool.alloc_rc("Rc");
        let weak = Rc::downgrade(&myrc);
        assert_eq!(weak.strong_count(), 1);
        assert_eq!(weak.weak_count(), 1);
        let strong = weak.upgrade().unwrap();
        assert_eq!(Rc::strong_count(&strong), 2);
        assert_eq!(myrc, strong);
        assert_eq!(*strong, "Rc");
    }
}
