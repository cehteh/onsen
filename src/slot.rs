use std::marker::PhantomData;
use std::pin::Pin;
use std::ptr::NonNull;

use crate::*;

/// Handle to allocated memory. This wraps an internal pointer to the allocation and provides
/// an API for accessing the content. To free memory slots must eventually be given back to
/// the pool they belong to by `pool.free()`, `pool.forget()` or `pool.take()`. Slots do not
/// track which Pool they belong to. It is the responsibility of the user to give them back to
/// the correct pool and ensure that they do not outlive the pool they belong to. In debug
/// mode it asserted that a slot belongs to the pool when it is given back. Safe abstractions
/// should track the slots pool.
#[repr(transparent)]
pub struct Slot<T>(pub(crate) NonNull<Entry<T>>, PhantomData<T>);

// While onsen itself is not send/sync the allocated objects may be.
unsafe impl<T: Send> Send for Slot<T> {}
unsafe impl<T: Sync> Sync for Slot<T> {}

impl<T> Slot<T> {
    // Private ctor
    pub(crate) fn new(from: NonNull<Entry<T>>) -> Self {
        Self(from, PhantomData)
    }

    /// Get a reference to the uninitialized memory at slot.
    ///
    /// # Safety
    ///
    /// The obtained references must be dropped before self.assume_init() is
    /// called as this would violate the Pin guarantees.
    ///
    /// # Panics
    ///
    ///  * The slot does not contain an uninitialized object
    #[inline]
    pub unsafe fn get_uninit(&mut self) -> &mut MaybeData<T> {
        assert!(self.is_uninitialized());
        &mut self.0.as_mut().maybe_data
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
    #[inline]
    pub unsafe fn assume_init(&mut self) -> &T {
        assert!(self.is_uninitialized());
        self.0.as_mut().descriptor = Descriptor::Initialized;
        &self.0.as_ref().maybe_data.data
    }

    /// Get a immutable reference to the object in slot, where slot must hold an initialized
    /// object.
    ///
    /// # Panics
    ///
    ///  * The slot does not contain a initialized object
    #[inline]
    pub fn get(&self) -> &T {
        assert!(self.is_initialized());
        unsafe { &self.0.as_ref().maybe_data.data }
    }

    /// Get a immutable reference to the object in slot, where slot must hold an initialized
    /// object.
    ///
    /// # Safety:
    ///
    /// This unsafe function does not assert the correct state of the Slot. It is intended to
    /// be used from higher level abstraction which can ensure correctness on a type level.
    #[inline]
    pub unsafe fn get_unchecked(&self) -> &T {
        &self.0.as_ref().maybe_data.data
    }

    /// Get a mutable reference to the object in slot, where slot must be an allocated slot.
    ///
    /// # Panics
    ///
    ///  * The slot does not contain a initialized object
    ///  * The object at slot was pinned before
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        assert!(self.is_initialized() && !self.is_pinned());
        unsafe {
            self.0.as_mut().descriptor = Descriptor::Referenced;
            &mut self.0.as_mut().maybe_data.data
        }
    }

    /// Get a mutable reference to the object in slot, where slot must be an allocated slot.
    ///
    /// # Safety:
    ///
    /// This unsafe function does not assert the correct state of the Slot. It is intended to
    /// be used from higher level abstraction which can ensure correctness on a type level.
    #[inline]
    pub unsafe fn get_mut_unchecked(&mut self) -> &mut T {
        self.0.as_mut().descriptor = Descriptor::Referenced;
        &mut self.0.as_mut().maybe_data.data
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
            self.0.as_mut().descriptor = Descriptor::Pinned;
            Pin::new_unchecked(&mut self.0.as_mut().maybe_data.data)
        }
    }

    /// Get a pinned reference to the object in slot, where slot must be an allocated
    /// slot.
    ///
    /// # Safety:
    ///
    /// This unsafe function does not assert the correct state of the Slot. It is intended to
    /// be used from higher level abstraction which can ensure correctness on a type level.
    pub unsafe fn pin_unchecked(&mut self) -> Pin<&mut T> {
        self.0.as_mut().descriptor = Descriptor::Pinned;
        Pin::new_unchecked(&mut self.0.as_mut().maybe_data.data)
    }

    /// Zero cost conversion to a u64 identifier of the slot. This identifier is guaranteed
    /// to represent a 48bit wide 8-aligned pointer. Thus highest 16 bits and the last 3 bits
    /// can be used for storing auxiliary information (NaN tagging).
    #[inline]
    pub fn into_u64(self) -> u64 {
        debug_assert_eq!(
            self.0.as_ptr() as u64 & 0xffff000000000007,
            0,
            "Something is wrong on this platform"
        );
        self.0.as_ptr() as u64
    }

    /// Converts a usize identifier obtained by `as_u64()` back into a Slot.
    ///
    /// # Safety
    ///
    /// The identifier must point to the same allocation as the slot where it was got from.
    #[inline]
    pub unsafe fn from_u64(id: u64) -> Self {
        debug_assert_eq!(id & 0xffff000000000007, 0, "Invalid identifier");
        Self(
            NonNull::new(id as *mut Entry<T>).expect("Invalid identifier"),
            PhantomData,
        )
    }

    /// Converts a usize identifier obtained by `as_usize()` back into a Slot. Before doing so
    /// it applies a mask to strip away any auxiliary bits.
    ///
    /// # Safety
    ///
    /// The identifier must point to the same allocation as the slot where it was got from. It
    /// may have the auxiliary bits set.
    #[inline]
    pub unsafe fn from_u64_masked(id: u64) -> Self {
        Self(
            NonNull::new((id & !0xffff000000000007) as *mut Entry<T>).expect("Invalid identifier"),
            PhantomData,
        )
    }

    /// Copies a slot handle.
    ///
    /// # Safety
    ///
    /// Slots must be given back to the pool only once which as well invalidates any copies.
    #[inline]
    pub unsafe fn copy(&self) -> Slot<T> {
        Slot(self.0, PhantomData)
    }

    /// Returns true when the slot is free and false when it is allocated.
    #[inline]
    pub fn is_free(&self) -> bool {
        self.entry().is_free()
    }

    /// Returns true when the slot is uninitialized,
    /// false on anything else.
    #[inline]
    pub fn is_uninitialized(&self) -> bool {
        self.entry().is_uninitialized()
    }

    /// Returns true when the slot is initialized, got referenced or pinned.
    /// Returns false when the slot is uninitialized.
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.entry().is_initialized()
    }

    /// Returns true when the slot ever got referenced or pinned.
    #[inline]
    pub fn is_referenced(&self) -> bool {
        self.entry().is_referenced()
    }

    /// Returns true when the slot ever got pinned.
    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.entry().is_pinned()
    }

    #[inline]
    fn entry(&self) -> &Entry<T> {
        // Safety: Slots are always created from valid entries
        unsafe { self.0.as_ref() }
    }
}
