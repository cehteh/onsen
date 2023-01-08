use std::mem::ManuallyDrop;
use std::ptr::NonNull;

use crate::*;

/// internal API
#[doc(hidden)]
pub trait PrivPoolApi<T>: Sized {
    /// Implements the interior mutability of a pool, if any.
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(&self, f: F) -> R;

    /// Allocates a new entry, either from the freelist or by extending the pool.
    /// Returns an uninitialized Entry pointer.
    fn alloc_entry(&self) -> NonNull<Entry<T>> {
        self.with_lock(|pool| pool.alloc_entry())
    }
}

/// The API for a Pool. This trait takes care for the locking the interior mutable pools and
/// default implements all its methods. It is not intended to be implemented by a user.
///
/// This trait must be in scope to be used.
pub trait PoolApi<T>
where
    Self: PrivPoolApi<T>,
{
    /// Configures the minimum number of entries the first block will hold. Must be called
    /// before the first allocation is made otherwise it will have no effect. Since blocks are
    /// allocated with exponentially growing size this should be reasonable small. The
    /// implementation will generously round this up to the next power of two. When not set it
    /// defaults to 64 entries.
    fn with_min_entries(&self, min_entries: usize) {
        self.with_lock(|pool| pool.min_entries(min_entries));
    }

    /// Destroys a Pool while leaking its allocated blocks.  The fast way out when one knows
    /// that allocations still exist and will never be returned to to the Pool. Either because
    /// the program exits or because the allocations are meant to stay.
    #[inline]
    fn leak(self) {
        std::mem::forget(self);
    }

    // PLANNED: try_alloc() with graceful backing off allocation and error handling

    /// Allocates a new `UnsafeBox` from this pool, initializes it with the supplied object.
    /// Freeing the object should be done manually with `pool.dealloc()`, `pool.forget()` or
    /// `pool.take()`. When a `UnsafeBox` is not deallocated, taken or forgotten as above then
    /// its it will leak until the Pool becomes dropped, this happens when panicking or might
    /// be intentional when the whole Pool becomes dropped at a later time.
    #[inline]
    fn alloc(&self, t: T) -> UnsafeBox<T> {
        let mut entry = self.alloc_entry();
        unsafe {
            entry.as_ptr().write(Entry {
                data: ManuallyDrop::new(t),
            });
            UnsafeBox::new(entry.as_mut())
        }
    }

    /// Frees `UnsafeBox` by calling its destructor. Puts the given memory back into the
    /// freelist.
    ///
    /// # Panics
    ///
    ///  * The `UnsafeBox` is not allocated from this pool.
    #[inline]
    fn dealloc(&self, mut ubox: UnsafeBox<T>) {
        self.with_lock(|pool| {
            ubox.assert_initialized();
            unsafe {
                pool.free_entry(ubox.manually_drop());
            }
        });
    }

    /// Free a `UnsafeBox` by calling its destructor. Puts the given memory back into the
    /// freelist. This function does not check if the object belongs to the pool. This makes
    /// it slightly faster but unsafe for that reason. Nevertheless many uses of `UnsafeBox`
    /// can guarantee this invariant because there is only one pool in use or the associated
    /// pool is stored along in a safe abstraction that keeps the `UnsafeBox`.
    ///
    /// # Safety
    ///
    ///  * The `UnsafeBox` must be allocated from this `Pool`, otherwise this is UB.
    #[inline]
    unsafe fn dealloc_unchecked(&self, mut ubox: UnsafeBox<T>) {
        self.with_lock(|pool| {
            pool.fast_free_entry_unchecked(ubox.manually_drop());
        });
    }

    /// Puts the given slot back into the freelist. Will not call the the destructor.
    ///
    /// # Panics
    ///
    ///  * The `UnsafeBox` is not allocated from this pool
    #[inline]
    fn forget(&self, mut ubox: UnsafeBox<T>) {
        self.with_lock(|pool| unsafe {
            pool.free_entry(ubox.take_entry());
        });
    }

    /// Takes an object out of the Pool and returns it. The `UnsafeBox` is put back to the
    /// freelist.
    ///
    /// # Panics
    ///
    ///  * The `UnsafeBox` is not allocated from this pool
    #[inline]
    fn take(&self, mut ubox: UnsafeBox<T>) -> T {
        self.with_lock(|pool| unsafe {
            let ret = ubox.take();
            pool.free_entry(ubox.take_entry());
            ret
        })
    }
}

/// A pool that can be shared, just adds `Clone` to the `PoolApi`
pub trait SharedPoolApi<T>: PoolApi<T> + Clone {}

/// Getting a reference to the shared pool. This makes it possible that not only pools
/// themselves can be referenced but also boxed values that contains reference to the pool
/// they are created from.
pub trait AsSharedPool<T, P: SharedPoolApi<T>> {
    /// Returns a reference to the underlying shared pool.
    fn as_shared_pool(&self) -> &P;
}
