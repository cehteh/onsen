use std::mem::MaybeUninit;
use std::pin::Pin;

use crate::*;

/// Handle to allocated memory. This wraps an internal pointer to the allocation and provides
/// an API for accessing the content. To free memory slots must eventually be given back to
/// the pool they belong to by `pool.free()`, `pool.forget()` or `pool.take()`. Slots do not
/// track which Pool they belong to. It is the responsibility of the user to give them back to
/// the correct pool and ensure that they do not outlive the pool they belong to. In debug
/// mode it asserted that a slot belongs to the pool when it is given back. Safe abstractions
/// should track the slots pool.
#[repr(transparent)]
pub struct Slot<T>(pub(crate) *mut Entry<T>);

impl<T> Slot<T> {
    /// Get a reference to the uninitialized memory at slot.
    ///
    /// # Safety
    ///
    /// The obtained references must be dropped before self.assume_init() is
    /// called as this violates the Pin guarantees.
    ///
    /// # Panics
    ///
    ///  * The slot does not contain an uninitialized object
    pub unsafe fn get_uninit(&mut self) -> &mut MaybeUninit<T> {
        assert!(self.is_uninitialized());
        &mut (*self.0).data
    }

    /// Tags the object at slot as initialized, return a reference to the data.
    ///
    /// # Safety
    ///
    /// The object must be fully initialized when calling this.
    ///
    /// # Panics
    ///
    ///  * The slot does not contain a uninitialized object
    pub unsafe fn assume_init(&mut self) -> &T {
        assert!(self.is_uninitialized());
        (*self.0).descr = Entry::<T>::INITIALIZED_SENTINEL;
        (*self.0).data.assume_init_ref()
    }

    /// Get a immutable reference to the object in slot, where slot must hold an initialized
    /// object.
    ///
    /// # Panics
    ///
    ///  * The slot does not contain a initialized object
    pub fn get(&self) -> &T {
        assert!(self.is_initialized());
        unsafe { (*self.0).data.assume_init_ref() }
    }

    /// Get a mutable reference to the object in slot, where slot must be an allocated slot.
    ///
    /// # Panics
    ///
    ///  * The slot does not contain a initialized object
    ///  * The object at slot was pinned before
    pub fn get_mut(&mut self) -> &mut T {
        assert!(self.is_initialized() && !self.is_pinned());
        unsafe {
            (*self.0).descr = Entry::<T>::REFERENCED_SENTINEL;
            (*self.0).data.assume_init_mut()
        }
    }

    /// Get a pinned reference to the object in slot, where slot must be an allocated
    /// slot. Since all Pool allocations are at stable slotesses it is straightforward to
    /// give Pin guarantees for them. One only need to make sure not to violate the Pin
    /// guarantees by calling unsafe functions
    ///
    /// # Panics
    ///
    ///  * A mutable reference of the object at slot was taken before.
    ///  * The slot is invalid, not obtained by a matching allocation.
    pub fn pin(&mut self) -> Pin<&mut T> {
        assert!(self.is_initialized() && !self.is_referenced());
        unsafe {
            (*self.0).descr = Entry::<T>::PINNED_SENTINEL;
            Pin::new_unchecked((*self.0).data.assume_init_mut())
        }
    }

    /// Zero cost conversion to a u64 identifier of the slot. This identifier is guaranteed
    /// to represent a 48bit wide 8-aligned pointer. Thus highest 16 bits and the last 3 bits
    /// can be used for storing auxiliary information (NaN tagging).
    pub fn into_u64(self) -> u64 {
        debug_assert_eq!(
            self.0 as u64 & 0xffff000000000007,
            0,
            "Something is wrong on this platform"
        );
        self.0 as u64
    }

    /// Converts a usize identifier obtained by `as_u64()` back into a Slot.
    ///
    /// # Safety
    ///
    /// The identifier must point to the same allocation as the slot where it was got from.
    pub unsafe fn from_u64(id: u64) -> Self {
        debug_assert_eq!(id & 0xffff000000000007, 0, "Invalid identifier");
        Self(id as *mut Entry<T>)
    }

    /// Converts a usize identifier obtained by `as_usize()` back into a Slot. Before doing so
    /// it applies a mask to strip away any auxiliary bits.
    ///
    /// # Safety
    ///
    /// The identifier must point to the same allocation as the slot where it was got from. It
    /// may have the auxiliary bits set.
    pub unsafe fn from_u64_masked(id: u64) -> Self {
        Self((id & !0xffff000000000007) as *mut Entry<T>)
    }

    /// Returns true when self belong to pool.
    pub fn is_in_pool<const E: usize>(&self, pool: &Pool<T, E>) -> bool {
        pool.has_slot(self)
    }

    /// Returns true when the slot is allocated and false when it is free.
    pub fn is_allocated(&self) -> bool {
        self.entry().is_allocated()
    }

    /// Returns true when the slot is uninitialized,
    /// false on anything else.
    pub fn is_uninitialized(&self) -> bool {
        self.entry().is_uninitialized()
    }

    /// Returns true when the slot is initialized, got referenced or pinned.
    /// Returns false when the slot is uninitialized.
    pub fn is_initialized(&self) -> bool {
        self.entry().is_initialized()
    }

    /// Returns true when the slot ever got referenced or pinned.
    pub fn is_referenced(&self) -> bool {
        self.entry().is_referenced()
    }

    /// Returns true when the slot ever got pinned.
    pub fn is_pinned(&self) -> bool {
        self.entry().is_pinned()
    }

    fn entry(&self) -> &Entry<T> {
        // Safety: Slots are always created from valid entries
        unsafe { &*self.0 }
    }
}
