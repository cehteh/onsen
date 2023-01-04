use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use crate::*;

/// Handle to an allocated object. This wraps the pointer to the allocation and provides an
/// API for accessing the content. To free memory slots should eventually be given back to the
/// pool they belong to by `pool.dealloc()`, `pool.forget()` or `pool.take()`. `BasicBox` do
/// not track which Pool they belong to. It is the responsibility of the user to give them
/// back to the correct pool. In debug mode it asserted that a slot belongs to the pool when
/// it is given back. Safe abstractions should track the associated pool.
///
/// When a `BasicBox` goes out of scope while it is not explicitly deallocated its contents
/// will be properly destructed but the associated memory will leak within the `Pool` from
/// where it was allocated. This happens especially on panics when raw `BasicBox` and not
/// higher level abstractions are used, thus one should make sure that this can't be the case
/// or happens rarely.
///
/// Sometimes can be used as advantage when using temporary pools where the memory reclamation
/// will happen when the `Pool` becomes destroyed. Benchmarks show that deallocation is approx
/// half of the cost on alloc/dealloc pair (compared to real work you do with the data this
/// should be very cheap nevertheless).
#[repr(transparent)]
pub struct BasicBox<'a, T>(
    // This Option is always `Some()` in live objects, only `dealloc*()`, `forget()` and
    // `take()` which consume the box sets it to `None` to notify the `Drop` implementation that the value is
    // already destructed.
    Option<&'a mut Entry<T>>,
);

unsafe impl<T: Send> Send for BasicBox<'_, T> {}
unsafe impl<T: Sync> Sync for BasicBox<'_, T> {}

impl<'a, T> BasicBox<'a, T> {
    // Private ctor
    pub(crate) fn new(from: &'a mut Entry<T>) -> Self {
        Self(Some(from))
    }
}

impl<'a, T> BasicBox<'a, T> {
    #[track_caller]
    pub(crate) fn assert_initialized(&self) {
        assert!(self.0.is_some());
    }

    pub(crate) unsafe fn as_entry_mut(&mut self) -> &mut Entry<T> {
        debug_assert!(self.0.is_some());
        // Safety: Option is always `Some` when calling this, see above
        self.0.as_mut().unwrap_unchecked()
    }

    pub(crate) unsafe fn as_entry(&self) -> &Entry<T> {
        debug_assert!(self.0.is_some());
        // Safety: Option is always `Some` when calling this, see above
        self.0.as_ref().unwrap_unchecked()
    }

    pub(crate) unsafe fn take_entry(&mut self) -> &mut Entry<T> {
        debug_assert!(self.0.is_some());
        self.0.take().unwrap_unchecked()
    }

    pub(crate) unsafe fn manually_drop(&mut self) -> &mut Entry<T> {
        ManuallyDrop::drop(&mut self.as_entry_mut().data);
        self.0.take().unwrap_unchecked()
    }

    pub(crate) unsafe fn take(&mut self) -> T {
        debug_assert!(self.0.is_some());
        ManuallyDrop::take(&mut self.as_entry_mut().data)
    }
}

impl<'a, T> Drop for BasicBox<'a, T> {
    #[inline]
    fn drop(&mut self) {
        if self.0.is_some() {
            // Safety: we just checked 'is_some()'
            unsafe {
                self.manually_drop();
            }
        }
    }
}

impl<T> Deref for BasicBox<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety: Always contains a valid object when this function is callable, see above
        unsafe { &self.as_entry().data }
    }
}

impl<T> DerefMut for BasicBox<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: Always contains a valid object when this function is callable, see above
        unsafe { &mut self.as_entry_mut().data }
    }
}

// /// Implements the NaN-Tagging API. This is u64 that can be OR'ed with a mask to form a quiet
// /// NaN.
// impl<T> BasicBox<'_, T, NaNTagging> {
//     /// Zero cost conversion to a u64 identifier of the slot. This identifier is guaranteed
//     /// to represent a 48bit wide 8-aligned pointer. Thus highest 16 bits and the last 3 bits
//     /// can be used for storing auxiliary information (NaN tagging).
//     #[inline]
//     #[must_use]
//     pub fn into_u64(self) -> u64 {
//         debug_assert_eq!(
//             self.0.unwrap().as_ptr() as u64 & 0xffff000000000007,
//             0,
//             "Something is wrong on this platform"
//         );
//         self.0.unwrap().as_ptr() as u64
//     }
// FIXME: lifetime on Pool
//     /// Converts a usize identifier obtained by `as_u64()` back into a `BasicBox`.
//     ///
//     /// # Safety
//     ///
//     /// The identifier must point to the same allocation as the slot where it was got from.
//     #[inline]
//     #[must_use]
//     pub unsafe fn from_u64(id: u64) -> Self {
//         debug_assert_eq!(id & 0xffff000000000007, 0, "Invalid identifier");
//         Self(
//             Some(NonNull::new(id as *mut Entry<T>).expect("Invalid identifier")),
//             PhantomData,
//             PhantomData,
//             PhantomData,
//         )
//     }
//
//     /// Converts a usize identifier obtained by `as_usize()` back into a `BasicBox`. Before
//     /// doing so it applies a mask to strip away any auxiliary bits.
//     ///
//     /// # Safety
//     ///
//     /// The identifier must point to the same allocation as the slot where it was got from. It
//     /// may have the auxiliary bits set.
//     #[inline]
//     #[must_use]
//     pub unsafe fn from_u64_masked(id: u64) -> Self {
//         Self(
//             Some(NonNull::new((id & !0xffff000000000007) as *mut Entry<T>).expect("Invalid identifier")),
//             PhantomData,
//             PhantomData,
//             PhantomData,
//         )
//     }
// }
