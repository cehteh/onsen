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

/// The pool type backing the `TSc`.
#[cfg(feature = "st_tbox")]
#[doc(hidden)]
pub type TScPool<T> = STPool<ScInner<T>>;

#[cfg(not(feature = "st_tbox"))]
#[doc(hidden)]
pub type TScPool<T> = TPool<ScInner<T>>;

/// For each type that shall be allocated with `TScs` there must be an associated global
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
/// let trc = TSc::new(123u8, MyTag);
/// ```
#[macro_export]
macro_rules! define_tsc_pool {
    ($TAG:ty:$T:ty) => {
        $crate::assoc_static!($TAG: $T, $crate::TScPool<$T> = $crate::TScPool::new());
    };
}

/// A reference counted smart pointer for Pool allocated objects. This wraps Slots in a safe
/// way. A `TSc` need a Pool holding `ScInner<T>`, not `T`.
pub struct TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    slot: Slot<ScInner<T>, Mutable>,
    tag: PhantomData<TAG>,
}

impl<T, TAG> TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    /// Associated function that returns the number of strong counters of this `TSc`.
    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        this.slot.get().get_strong()
    }
}

impl<T, TAG> TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    /// Allocate a `TSc` from a Pool.
    #[inline]
    pub fn new(t: T, _tag: TAG) -> Self {
        Self {
            slot: T::get_static().alloc(ScInner::new(t)).for_mutation(),
            tag: PhantomData,
        }
    }

    /// Allocate a `TSc` from a Pool with inferred or turbofish tag.
    #[inline]
    pub fn new_notag(t: T) -> Self {
        Self {
            slot: T::get_static().alloc(ScInner::new(t)).for_mutation(),
            tag: PhantomData,
        }
    }
}

impl<T, TAG> Default for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static + Default,
    TAG: 'static,
{
    /// Allocate a default initialized `TSc`
    #[inline]
    fn default() -> Self {
        TSc::new_notag(T::default())
    }
}

impl<T, TAG> Clone for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
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

impl<T, TAG> Drop for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn drop(&mut self) {
        let mslot = self.slot.get_mut();

        mslot.dec_strong();

        if mslot.get_strong() == 0 {
            unsafe {
                T::get_static().free_by_ref(&mut self.slot);
            }
        }
    }
}

impl<T, TAG> Deref for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, TAG> DerefMut for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T, TAG> Borrow<T> for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn borrow(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, TAG> BorrowMut<T> for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T, TAG> AsRef<T> for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.slot.get().data.assume_init_ref() }
    }
}

impl<T, TAG> AsMut<T> for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.slot.get_mut().data.assume_init_mut() }
    }
}

impl<T: PartialEq, TAG> PartialEq for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd, TAG> PartialOrd for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
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

impl<T: Ord, TAG> Ord for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: Eq, TAG> Eq for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
}

impl<T: Hash, TAG> Hash for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher, TAG> Hasher for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
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

impl<T: fmt::Display, TAG> fmt::Display for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug, TAG> fmt::Debug for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, TAG> fmt::Pointer for TSc<T, TAG>
where
    T: AssocStatic<TScPool<T>, TAG> + 'static,
    TAG: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use serial_test::serial;

    define_tsc_pool!((): &'static str);
    define_tsc_pool!((): u64);

    #[test]
    #[ignore]
    #[serial]
    fn smoke() {
        TBox::<&'static str, ()>::pool()
            .acquire()
            .expect("some other thread owns the pool");

        let _mybox = TSc::new("TBoxed", ());

        TBox::<&'static str, ()>::pool()
            .release()
            .expect("thread does not own the pool");
    }
}
