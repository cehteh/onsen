use std::mem::MaybeUninit;

use crate::*;

/// Entries within a Pool.
#[repr(C, align(8))]
pub(crate) struct Entry<T> {
    pub(crate) data: MaybeUninit<T>,
    pub(crate) descr: *mut Entry<T>,
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
            self.descr as usize,
            UNINITIALIZED | INITIALIZED | REFERENCED | PINNED
        )
    }

    /// Returns true when the slot is uninitialized,
    /// false on anything else.
    pub fn is_uninitialized(&self) -> bool {
        matches!(self.descr as usize, UNINITIALIZED)
    }

    /// Returns true when the slot is initialized, got referenced or pinned.
    /// Returns false when the slot is uninitialized.
    pub fn is_initialized(&self) -> bool {
        matches!(self.descr as usize, INITIALIZED | REFERENCED | PINNED)
    }

    /// Returns true when the slot at 'slot' ever got referenced or pinned.
    pub fn is_referenced(&self) -> bool {
        matches!(self.descr as usize, REFERENCED | PINNED)
    }

    /// Returns true when the slot at 'slot' ever got pinned.
    pub fn is_pinned(&self) -> bool {
        matches!(self.descr as usize, PINNED)
    }
}
