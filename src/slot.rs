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
///
/// Access is guarded by a typestate Policy tag.
#[repr(transparent)]
pub struct Slot<T, S: Policy>(pub(crate) NonNull<Entry<T>>, PhantomData<T>, PhantomData<S>);

unsafe impl<T: Send, S: Policy> Send for Slot<T, S> {}
unsafe impl<T: Sync, S: Policy> Sync for Slot<T, S> {}

impl<T, S: Policy> Slot<T, S> {
    // Private ctor
    pub(crate) fn new(from: NonNull<Entry<T>>) -> Self {
        Self(from, PhantomData, PhantomData)
    }
}

/// Base of the typestate policies.
pub trait Policy {}

/// Implements how/if the content of a slot shall be dropped.
pub trait DropPolicy: Policy {
    // Less boilerplate when we provide a default impl here
    #[inline]
    #[doc(hidden)]
    fn manually_drop<T>(data: &mut std::mem::ManuallyDrop<T>) {
        unsafe { std::mem::ManuallyDrop::drop(data) };
    }
}

/// Permits getting a immutable reference to the value.
pub trait CanGetReference: Policy {}
/// Permits destroying the Slot by taking the Value out of it.
pub trait CanTakeValue: Policy {}

/// Holds uninitialized memory.
pub enum Uninitialized {}
impl Policy for Uninitialized {}
impl DropPolicy for Uninitialized {
    // The only case where this is a NOP
    #[inline]
    fn manually_drop<T>(_data: &mut std::mem::ManuallyDrop<T>) {}
}

/// Holds an initialized value.
pub enum Initialized {}
impl Policy for Initialized {}
impl DropPolicy for Initialized {}
impl CanGetReference for Initialized {}
impl CanTakeValue for Initialized {}

/// Initialized, mutable references are permitted.
pub enum Mutable {}
impl Policy for Mutable {}
impl DropPolicy for Mutable {}
impl CanGetReference for Mutable {}
impl CanTakeValue for Mutable {}

/// Initialized, pinned references are permitted.
pub enum Pinnable {}
impl Policy for Pinnable {}
impl DropPolicy for Pinnable {}
impl CanGetReference for Pinnable {}
impl CanTakeValue for Pinnable {}

/// Initialized, NaN tagged identifier API.
pub enum NaNTagging {}
impl Policy for NaNTagging {}
impl DropPolicy for NaNTagging {}
impl CanGetReference for NaNTagging {}
impl CanTakeValue for NaNTagging {}

/// Uninitialized slots are allocated with `pool.alloc_uninit()` as the name says they are
/// only allocated yet but do not contain an initialized value. Similar to `MaybeUninit` one
/// can obtain a reference to an `Entry` to this slot and then `write()` a value to it. When
/// done so the slot is transformed into a initialized slot with `assume_init()`.
impl<T> Slot<T, Uninitialized> {
    /// Get a reference to the uninitialized memory at slot.
    #[inline]
    pub fn get_uninit(&mut self) -> &mut Entry<T> {
        unsafe { self.0.as_mut() }
    }

    /// Tags the object at slot as initialized. Return an initialized Slot.
    ///
    /// # Safety
    ///
    /// The object must be fully initialized when calling this.
    #[inline]
    #[must_use]
    pub unsafe fn assume_init(self) -> Slot<T, Initialized> {
        Slot::<T, Initialized>(self.0, PhantomData, PhantomData)
    }
}

/// Initialized slots hold a valid value. When mutation is required it has to be
/// translated into on of the more specific policies with the following methods.
impl<T> Slot<T, Initialized> {
    /// Transforms an initialized Slot into one that can be mutated by references
    #[inline]
    #[must_use]
    pub fn for_mutation(self) -> Slot<T, Mutable> {
        Slot(self.0, PhantomData, PhantomData)
    }

    /// Transforms an initialized Slot into one that can be mutated by pinned references
    #[inline]
    #[must_use]
    pub fn for_pinning(self) -> Slot<T, Pinnable> {
        Slot(self.0, PhantomData, PhantomData)
    }

    /// Transforms an initialized Slot into one that can be use by nantagging facilities
    #[inline]
    #[must_use]
    pub fn for_nantagging(self) -> Slot<T, NaNTagging> {
        Slot(self.0, PhantomData, PhantomData)
    }
}

/// Allows one to obtain a mutable reference.
impl<T> Slot<T, Mutable> {
    /// Get a mutable reference to the object in slot, where slot must be an allocated slot.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut self.0.as_mut().data }
    }

    /// Copies a slot handle.
    ///
    /// # Safety
    ///
    /// Slots must be only once given back to the pool which as well invalidates any
    /// copies. See how Rc uses this for the replication.
    #[inline]
    #[must_use]
    pub unsafe fn copy(&self) -> Self {
        Slot(self.0, PhantomData, PhantomData)
    }
}

/// Allows one to obtain a mutable pinned reference with `get_pin()`.
impl<T> Slot<T, Pinnable> {
    /// Get a pinned reference to the object in slot, where slot must be an allocated
    /// slot. Since all Pool allocations are at stable slotesses it is straightforward to
    /// give Pin guarantees for them. One only need to make sure not to violate the Pin
    /// guarantees by calling unsafe functions
    pub fn get_pin(&mut self) -> Pin<&mut T> {
        unsafe { Pin::new_unchecked(&mut self.0.as_mut().data) }
    }
}

/// For slots that are initialized it is always possible to get a immutable reference to the
/// value.
impl<T, S: CanGetReference> Slot<T, S> {
    /// Get a immutable reference to the object in slot, where slot must hold an initialized
    /// object.
    #[inline]
    #[must_use]
    pub fn get(&self) -> &T {
        unsafe { &self.0.as_ref().data }
    }
}

/// Implements the NaN-Tagging API. This is u64 that can be OR'ed with a mask to form a quiet
/// NaN.
impl<T> Slot<T, NaNTagging> {
    /// Zero cost conversion to a u64 identifier of the slot. This identifier is guaranteed
    /// to represent a 48bit wide 8-aligned pointer. Thus highest 16 bits and the last 3 bits
    /// can be used for storing auxiliary information (NaN tagging).
    #[inline]
    #[must_use]
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
    #[must_use]
    pub unsafe fn from_u64(id: u64) -> Self {
        debug_assert_eq!(id & 0xffff000000000007, 0, "Invalid identifier");
        Self(
            NonNull::new(id as *mut Entry<T>).expect("Invalid identifier"),
            PhantomData,
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
    #[must_use]
    pub unsafe fn from_u64_masked(id: u64) -> Self {
        Self(
            NonNull::new((id & !0xffff000000000007) as *mut Entry<T>).expect("Invalid identifier"),
            PhantomData,
            PhantomData,
        )
    }
}
