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
pub struct Rc<'a, T> {
    slot: Slot<RcInner<T>, Mutable>,
    pool: &'a Pool<RcInner<T>>,
}

impl<T> Rc<'_, T> {
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

impl<'a, T> Rc<'a, T> {
    /// Allocate a Rc from a Pool. The allocated Rc must not outlive the Pool it was created
    /// from.
    #[inline]
    pub fn new(t: T, pool: &'a Pool<RcInner<T>>) -> Self {
        Self {
            slot: pool.alloc(RcInner::new(t)).for_mutation(),
            pool,
        }
    }

    /// Creates a Weak reference from a Rc.
    #[must_use]
    pub fn downgrade(this: &Self) -> Weak<'a, T> {
        this.slot.get().inc_weak();
        unsafe {
            Weak::<'a, T> {
                slot: this.slot.copy(),
                pool: this.pool,
            }
        }
    }
}

impl<'a, T: Default> Rc<'a, T> {
    /// Allocate a default initialized Rc from a Pool. The allocated Rc must not outlive the
    /// Pool it was created from.
    #[inline]
    pub fn default(pool: &'a Pool<RcInner<T>>) -> Self {
        Rc::new(T::default(), pool)
    }
}

impl<T> Clone for Rc<'_, T> {
    #[must_use]
    fn clone(&self) -> Self {
        self.slot.get().inc_strong();
        unsafe {
            Self {
                slot: self.slot.copy(),
                pool: self.pool,
            }
        }
    }
}

impl<'a, T: Default> Pool<RcInner<T>> {
    /// Allocate a default initialized Rc from a Pool.
    // TODO: remove before v1.0
    #[inline]
    #[deprecated(since = "0.10.0", note = "please use `Rc:new()` instead")]
    pub fn default_rc(&'a mut self) -> Rc<'a, T> {
        #[allow(deprecated)]
        self.alloc_rc(T::default())
    }
}

impl<'a, T> Pool<RcInner<T>> {
    /// Allocate a Box from a Pool.
    // TODO: remove before v1.0
    #[inline]
    #[deprecated(since = "0.10.0", note = "please use `Rc:default()` instead")]
    pub fn alloc_rc(&'a self, t: T) -> Rc<'a, T> {
        Rc {
            slot: self.alloc(RcInner::new(t)).for_mutation(),
            pool: self,
        }
    }
}

impl<T> Drop for Rc<'_, T> {
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

impl<T> Deref for Rc<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> DerefMut for Rc<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T> Borrow<T> for Rc<'_, T> {
    #[inline]
    fn borrow(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> BorrowMut<T> for Rc<'_, T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T> AsRef<T> for Rc<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T> AsMut<T> for Rc<'_, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T: PartialEq> PartialEq for Rc<'_, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd> PartialOrd for Rc<'_, T> {
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

impl<T: Ord> Ord for Rc<'_, T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq> Eq for Rc<'_, T> {}

impl<T: Hash> Hash for Rc<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher> Hasher for Rc<'_, T> {
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

impl<T: fmt::Display> fmt::Display for Rc<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Rc<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Pointer for Rc<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// Weak references do not keep the object alive.
pub struct Weak<'a, T> {
    slot: Slot<RcInner<T>, Mutable>,
    pool: &'a Pool<RcInner<T>>,
}

impl<T> Weak<'_, T> {
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

impl<'a, T> Weak<'a, T> {
    /// Tries to create a Rc from a Weak reference. Fails when the strong count was zero.
    #[must_use]
    pub fn upgrade(&self) -> Option<Rc<'a, T>> {
        if self.strong_count() > 0 {
            self.slot.get().inc_strong();
            unsafe {
                Some(Rc::<'a, T> {
                    slot: self.slot.copy(),
                    pool: self.pool,
                })
            }
        } else {
            None
        }
    }
}

impl<T> Clone for Weak<'_, T> {
    fn clone(&self) -> Self {
        self.slot.get().inc_weak();
        unsafe {
            Self {
                slot: self.slot.copy(),
                pool: self.pool,
            }
        }
    }
}

impl<T> Drop for Weak<'_, T> {
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

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let pool = Pool::new();
        let _myrc = Rc::new("Rc", &pool);
    }
}
