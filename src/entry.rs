use std::mem::ManuallyDrop;

use crate::*;

/// Special purpose version of MaybeUninit that may hold a forward pointer of the linked
/// freelist when the Slot is free.
// TODO: #[repr(transparent)]
#[repr(C)]
pub union MaybeData<T> {
    pub(crate) uninit: (),
    pub(crate) data: ManuallyDrop<T>,
    pub(crate) fwd_ptr: *mut Entry<T>,
}

impl<T> MaybeData<T> {
    /// Overwrites the potentially uninitialized MaybeData with new data without dropping the
    /// old value. Returns a reference to the new data.
    #[inline(always)]
    pub fn write(&mut self, val: T) -> &mut T {
        self.data = ManuallyDrop::new(val);
        // SAFETY: We just initialized this value.
        unsafe { &mut self.data }
    }
}

/// Entries within a Pool.
#[repr(C, align(8))]
pub(crate) struct Entry<T> {
    pub(crate) maybe_data: MaybeData<T>,
    pub(crate) descr_rev_ptr: *mut Entry<T>,
}

impl<T> Entry<T> {
    pub(crate) const END_OF_FREELIST: *mut Self = std::ptr::null_mut();
    pub(crate) const PINNED_SENTINEL: *mut Self = PINNED as *mut Self;
    pub(crate) const REFERENCED_SENTINEL: *mut Self = REFERENCED as *mut Self;
    pub(crate) const INITIALIZED_SENTINEL: *mut Self = INITIALIZED as *mut Self;
    pub(crate) const UNINITIALIZED_SENTINEL: *mut Self = UNINITIALIZED as *mut Self;

    /// Returns true when the slot at 'slot' is allocated and false when it is free.
    pub fn is_allocated(&self) -> bool {
        matches!(
            self.descr_rev_ptr as usize,
            UNINITIALIZED | INITIALIZED | REFERENCED | PINNED
        )
    }

    /// Returns true when the slot is uninitialized,
    /// false on anything else.
    pub fn is_uninitialized(&self) -> bool {
        matches!(self.descr_rev_ptr as usize, UNINITIALIZED)
    }

    /// Returns true when the slot is initialized, got referenced or pinned.
    /// Returns false when the slot is uninitialized.
    pub fn is_initialized(&self) -> bool {
        matches!(
            self.descr_rev_ptr as usize,
            INITIALIZED | REFERENCED | PINNED
        )
    }

    /// Returns true when the slot at 'slot' ever got referenced or pinned.
    pub fn is_referenced(&self) -> bool {
        matches!(self.descr_rev_ptr as usize, REFERENCED | PINNED)
    }

    /// Returns true when the slot at 'slot' ever got pinned.
    pub fn is_pinned(&self) -> bool {
        matches!(self.descr_rev_ptr as usize, PINNED)
    }
}
