#![cfg(feature = "tbox")]
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::*;

/// The pool type backing the `TRc`.
#[cfg(feature = "st_tbox")]
#[doc(hidden)]
pub type TRcPool<T> = STPool<RcInner<T>>;

#[cfg(not(feature = "st_tbox"))]
#[doc(hidden)]
pub type TRcPool<T> = TPool<RcInner<T>>;

/// For each type that shall be allocated with `TRcs` there must be an associated global
/// memory pool. This is defined with this macro.
///
/// ```rust,ignore
/// use onsen::*;
///
/// // ZST tag
/// struct MyTag;
///
/// // define a pool holding u8 values
/// define_trc_pool!(MyTag: u8);
///
/// /// allocated a trc from the u8 pool tagged by 'MyTag'
/// let trc = TRc::new(123u8, MyTag);
/// ```
#[macro_export]
macro_rules! define_trc_pool {
    ($TAG:ty:$T:ty) => {
        $crate::assoc_static!($TAG: $T, $crate::TRcPool<$T> = $crate::TRcPool::new());
    };
}

/// A reference counted smart pointer for Pool allocated objects. This wraps Slots in a safe
/// way. A `TRc` need a Pool holding `RcInner<T>`, not `T`.
pub struct TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    slot: Slot<RcInner<T>, Mutable>,
    tag: PhantomData<TAG>,
}

impl<T, TAG> TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    /// Associated function that returns the number of strong counters of this `TRc`.
    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        this.slot.get().get_strong()
    }

    /// Associated function that returns the number of weak counters of this `TRc`.
    #[must_use]
    pub fn weak_count(this: &Self) -> usize {
        this.slot.get().get_weak()
    }
}

impl<T, TAG> TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    /// Allocate a `TRc` from a Pool.
    #[inline]
    pub fn new(t: T, _tag: TAG) -> Self {
        Self {
            slot: T::get_static().alloc(RcInner::new(t)).for_mutation(),
            tag: PhantomData,
        }
    }

    /// Allocate a `TRc` from a Pool with inferred or turbofish tag.
    #[inline]
    pub fn new_notag(t: T) -> Self {
        Self {
            slot: T::get_static().alloc(RcInner::new(t)).for_mutation(),
            tag: PhantomData,
        }
    }

    /// Creates a `TWeak` reference from a `TRc`.
    #[must_use]
    pub fn downgrade(this: &Self) -> TWeak<T, TAG> {
        this.slot.get().inc_weak();
        unsafe {
            TWeak::<T, TAG> {
                slot: this.slot.copy(),
                tag: PhantomData,
            }
        }
    }
}

impl<T, TAG> Default for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static + Default,
    TAG: 'static,
{
    /// Allocate a default initialized `TRc`
    #[inline]
    fn default() -> Self {
        TRc::new_notag(T::default())
    }
}

impl<T, TAG> Clone for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[must_use]
    fn clone(&self) -> Self {
        self.slot.get().inc_strong();
        unsafe {
            Self {
                slot: self.slot.copy(),
                tag: PhantomData,
            }
        }
    }
}

impl<T, TAG> Drop for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn drop(&mut self) {
        let mslot = self.slot.get_mut();

        mslot.dec_strong();

        if mslot.get_strong() == 0 {
            if mslot.get_weak() == 0 {
                // no references exist, can be freed completely
                unsafe {
                    T::get_static().free_by_ref(&mut self.slot);
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

impl<T, TAG> Deref for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, TAG> DerefMut for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T, TAG> Borrow<T> for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn borrow(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, TAG> BorrowMut<T> for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T, TAG> AsRef<T> for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, TAG> AsMut<T> for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T: PartialEq, TAG> PartialEq for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd, TAG> PartialOrd for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
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

impl<T: Ord, TAG> Ord for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: Eq, TAG> Eq for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
}

impl<T: Hash, TAG> Hash for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher, TAG> Hasher for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
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

impl<T: fmt::Display, TAG> fmt::Display for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug, TAG> fmt::Debug for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, TAG> fmt::Pointer for TRc<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// `TWeak` references do not keep the object alive.
pub struct TWeak<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    slot: Slot<RcInner<T>, Mutable>,
    tag: PhantomData<TAG>,
}

impl<T, TAG> TWeak<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    /// Associated function that returns the number of strong counters of this `TWeak`.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.slot.get().get_strong()
    }

    /// Associated function that returns the number of weak counters of this `TWeak`.
    #[must_use]
    pub fn weak_count(&self) -> usize {
        self.slot.get().get_weak()
    }
}

impl<T, TAG> TWeak<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    /// Tries to create a `TRc` from a `TWeak` reference. Fails when the strong count was zero.
    #[must_use]
    pub fn upgrade(&self) -> Option<TRc<T, TAG>> {
        if self.strong_count() > 0 {
            self.slot.get().inc_strong();
            unsafe {
                Some(TRc::<T, TAG> {
                    slot: self.slot.copy(),
                    tag: PhantomData,
                })
            }
        } else {
            None
        }
    }
}

impl<T, TAG> Clone for TWeak<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn clone(&self) -> Self {
        self.slot.get().inc_weak();
        unsafe {
            Self {
                slot: self.slot.copy(),
                tag: PhantomData,
            }
        }
    }
}

impl<T, TAG> Drop for TWeak<T, TAG>
where
    T: AssocStatic<TRcPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn drop(&mut self) {
        let mslot = self.slot.get_mut();
        mslot.dec_weak();

        if mslot.get_strong() == 0 {
            if mslot.get_weak() == 0 {
                // no references exist, can be freed completely
                unsafe {
                    T::get_static().free_by_ref(&mut self.slot);
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

#[cfg(test)]
mod tests {
    use crate::*;
    use serial_test::serial;

    define_trc_pool!((): &'static str);
    define_trc_pool!((): u64);

    #[test]
    #[ignore]
    #[serial]
    fn smoke() {
        TBox::<&'static str, ()>::get_pool()
            .acquire()
            .expect("some other thread owns the pool");

        let _mybox = TRc::new("TBoxed", ());

        TBox::<&'static str, ()>::get_pool()
            .release()
            .expect("thread does not own the pool");
    }
}
