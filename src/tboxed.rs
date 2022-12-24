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

/// The pool type backing the `TBox`. Note that `STPool` has the `release()` and `steal()`
/// methods which `TPool`does not have.
#[cfg(feature = "st_tbox")]
#[doc(hidden)]
pub type TBoxPool<T> = STPool<T>;

#[cfg(not(feature = "st_tbox"))]
#[doc(hidden)]
pub type TBoxPool<T> = TPool<T>;

/// For each type that shall be allocated with `TBoxes` there must be an associated global
/// memory pool. This is defined with this macro.
///
/// ```rust,ignore
/// use onsen::*;
///
/// // ZST tag
/// struct MyTag;
///
/// // define a pool holding u8 values
/// define_tbox_pool!(MyTag: u8);
///
/// /// allocated a tbox from the u8 pool tagged by 'MyTag'
/// let tbox = TBox::new(123u8, MyTag);
/// ```
#[macro_export]
macro_rules! define_tbox_pool {
    ($TAG:ty:$T:ty) => {
        $crate::assoc_static!($TAG: $T, $crate::TBoxPool<$T> = $crate::TBoxPool::new());
    };
}

/// A `TBox` for Pool allocated objects. This wraps Slots in a safe way. Dropping a `TBox`
/// will ensure that the destructor is called and the memory is given back to the pool. `TBoxes`
/// use a TAG to discriminate. This can be any user defined type, preferably a ZST made only
/// for this purpose. See the `assoc_static` documentation for details.
pub struct TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
    TAG: 'static,
{
    slot: Slot<T, Mutable>,
    tag: PhantomData<TAG>,
}

impl<T, TAG> TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
    TAG: 'static,
{
    /// Allocate a `TBox` from a static pool.
    #[inline]
    pub fn new(t: T, _tag: TAG) -> Self {
        Self {
            slot: T::get_static().alloc(t).for_mutation(),
            tag: PhantomData,
        }
    }

    /// Allocate a `TBox` from a static Pool with inferred or turbofish tag.
    #[inline]
    pub fn new_notag(t: T) -> Self {
        Self {
            slot: T::get_static().alloc(t).for_mutation(),
            tag: PhantomData,
        }
    }

    /// Associated function that frees the memory of a `TBox` without calling the destructor
    /// of its value.
    #[inline]
    pub fn forget(mut b: Self) {
        unsafe { T::get_static().forget_by_ref(&mut b.slot) }
    }

    /// Associated function that frees the memory of a `TBox` and returns the value it was holding.
    #[inline]
    #[must_use]
    pub fn take(mut b: Self) -> T {
        unsafe { T::get_static().take_by_ref(&mut b.slot) }
    }

    /// Get a reference to the associated pool of this `TBox` type
    #[inline]
    #[must_use]
    pub fn pool() -> &'static TBoxPool<T> {
        T::get_static()
    }
}

impl<T, TAG> Default for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static + Default,
    TAG: 'static,
{
    /// Allocate a default initialized `TBox`
    #[inline]
    fn default() -> Self {
        TBox::new_notag(T::default())
    }
}

impl<T, TAG> Drop for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
    TAG: 'static,
{
    #[inline]
    fn drop(&mut self) {
        unsafe {
            T::get_static().free_by_ref(&mut self.slot);
        }
    }
}

impl<T, TAG: 'static> Deref for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &<Self as Deref>::Target {
        self.slot.get()
    }
}

impl<T, TAG: 'static> DerefMut for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        self.slot.get_mut()
    }
}

impl<T, TAG: 'static> Borrow<T> for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    #[inline]
    fn borrow(&self) -> &T {
        self.slot.get()
    }
}

impl<T, TAG: 'static> BorrowMut<T> for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        self.slot.get_mut()
    }
}

impl<T, TAG: 'static> AsRef<T> for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self.slot.get()
    }
}

impl<T, TAG: 'static> AsMut<T> for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.slot.get_mut()
    }
}

impl<T: PartialEq, TAG: 'static> PartialEq for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd, TAG: 'static> PartialOrd for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
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

impl<T: Ord, TAG: 'static> Ord for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: Eq, TAG: 'static> Eq for TBox<T, TAG> where T: AssocStatic<TBoxPool<T>, TAG> + 'static {}

impl<T: Hash, TAG: 'static> Hash for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: Hasher, TAG: 'static> Hasher for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    hasher_impl! {}
}

impl<T: fmt::Display, TAG: 'static> fmt::Display for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug, TAG: 'static> fmt::Debug for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, TAG: 'static> fmt::Pointer for TBox<T, TAG>
where
    T: AssocStatic<TBoxPool<T>, TAG> + 'static,
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

    define_tbox_pool!((): &'static str);
    define_tbox_pool!((): u64);

    #[test]
    #[serial]
    fn smoke() {
        TBox::<&'static str, ()>::pool().acquire().unwrap();

        {
            let _mybox = TBox::new("TBoxed", ());
        }

        TBox::<&'static str, ()>::pool().release().unwrap();
    }

    #[test]
    #[serial]
    #[ignore]
    fn alloc_many() {
        TBox::<&'static str, ()>::pool().acquire().unwrap();
        {
            const HOWMANY: usize = 100000000;
            let mut vec = Vec::with_capacity(HOWMANY);
            for i in 0..HOWMANY {
                vec.push(TBox::new(i as u64, ()));
            }
        }
        TBox::<&'static str, ()>::pool().release().unwrap();
    }
}
