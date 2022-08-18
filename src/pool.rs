use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

use crate::*;

/// A Memory Pool holding objects of type T.
///
/// The pool can not track how many references to a slot are active. This makes all
/// `free()`, `forget()` and `take()` unsafe. Thus they have to be carefully protected by RAII
/// guards or other means. Another approach is to use the *address* for all addressing and
/// convert to references only on demand and drop the reference as soon as possible.
///
/// The Pool API is pretty low-level and frequently unsafe is intended to be used to build
/// safe high level abstractions.
pub struct Pool<T: Sized>(pub(crate) RefCell<PoolInner<T>>);

impl<T> Pool<T> {
    /// Creates a new Pool for objects of type T.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(RefCell::new(PoolInner {
            blocks: [(); NUM_BLOCKS].map(|_| None),
            blocks_allocated: 0,
            min_entries: 64,
            in_use: 0,
            freelist: None,
        }))
    }

    /// Configures the minimum of entries the first block will hold. Must be called before the
    /// first allocation is made. Can be used when the number of entries that will be used is
    /// roughly guessable and or the size of entries is small.  Setting this improves cache
    /// locality.  Since blocks are allocated with exponentially growing size this should be
    /// still small enough, approx 1/4 of the average number of entries to be expected. The
    /// implementation will generously round this up to the next power of two. When not set it
    /// defaults to 64 entries.
    ///
    /// # Panics
    ///
    /// When called after the pool made its first allocation.
    pub fn with_min_entries(&self, min_entries: usize) {
        assert!(
            self.0.borrow_mut().blocks[0].is_none(),
            "min_entries must be set before using the pool"
        );
        self.0.borrow_mut().min_entries = min_entries;
    }

    /// Destroys a Pool while leaking its allocated blocks.  The fast way out when one knows
    /// that allocations still exist and will never be returned to to the Pool. Either because
    /// the program exits or because the allocations are meant to stay.
    #[inline]
    pub fn leak(self) {
        std::mem::forget(self);
    }

    /// Allocates a new entry, either from the freelist or by extending the pool.
    /// Returns Entry pointer tagged as UNINITIALIZED.
    fn alloc_entry(&self) -> NonNull<Entry<T>> {
        self.0.borrow_mut().alloc_entry()
    }

    /// Allocates a new slot from the pool, initializes it with the supplied object and
    /// returns a slot handle. Freeing the object should be done manually with `pool.free()`,
    /// `pool::forget()` or `pool.take()`. The user must take care that the slot/references
    /// obtained from it are not used after free as this may panic or return another object.
    #[must_use = "Slot is required for freeing memory, dropping it will leak"]
    #[inline]
    pub fn alloc(&self, t: T) -> Slot<T, Initialized> {
        let mut entry = self.alloc_entry();
        unsafe {
            entry.as_mut().maybe_data = MaybeData {
                data: ManuallyDrop::new(t),
            };
        }
        Slot::new(entry)
    }

    /// Non consuming variant of `pool.free()`, allows freeing slots that are part of other
    /// structures while keeping Slot non-Copy. The slot must not be used after this.
    /// See `slot.free()` for details.
    #[allow(clippy::missing_safety_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub unsafe fn free_by_ref<S: DropPolicy>(&self, slot: &mut Slot<T, S>) {
        let mut pool = self.0.borrow_mut();
        S::manually_drop(&mut slot.0.as_mut().maybe_data.data);
        pool.free_entry(slot.0.as_ptr());
    }

    /// Non consuming variant of `pool.take()`, allows taking slots that are part of other
    /// structures while keeping Slot non-Copy.  The slot must not be used after this.  See
    /// `slot.take()` for details.
    #[allow(clippy::missing_safety_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub unsafe fn take_by_ref<S: CanTakeValue>(&self, slot: &mut Slot<T, S>) -> T {
        let mut pool = self.0.borrow_mut();
        let ret = ManuallyDrop::take(&mut slot.0.as_mut().maybe_data.data);
        pool.free_entry(slot.0.as_ptr());
        ret
    }

    /// Allocates a new slot from the pool, keeps the content uninitialized returns a Slot
    /// handle to it. Freeing the object should be done manually with `pool.free()` or
    /// `pool.forget()`. The user must take care that the provided handle or references
    /// obtained from is are used after free as this may panic or return another object.
    #[must_use = "Slot is required for freeing memory, dropping it will leak"]
    #[inline]
    pub fn alloc_uninit(&self) -> Slot<T, Uninitialized> {
        Slot::new(self.alloc_entry())
    }

    /// Frees `slot` by calling its destructor when it contains an initialized object,
    /// uninitialized objects become forgotten as with `Pool::forget()`. Puts the given slot
    /// back into the freelist.
    ///
    /// # Safety
    ///
    /// Slots must not be freed while references pointing to it.
    ///
    /// # Panics
    ///
    ///  * The slot is already free
    ///  * The slot is invalid, not from this pool (debug only).
    #[inline]
    pub unsafe fn free<S: DropPolicy>(&self, mut slot: Slot<T, S>) {
        self.free_by_ref(&mut slot);
    }

    /// Puts the given slot back into the freelist. Will not call the the destructor.
    ///
    /// # Safety
    ///
    /// Slots must not be forgotten while references pointing to it.
    ///
    /// # Panics
    ///
    ///  * The slot is already free
    ///  * The slot is invalid, not from this pool (debug only).
    #[inline]
    pub unsafe fn forget<S: Policy>(&self, mut slot: Slot<T, S>) {
        self.forget_by_ref(&mut slot);
    }

    /// Non consuming variant of `pool.forget()`, allows forgetting slots that are part of
    /// other structures while keeping Slot non-Copy.  The slot must not be used after this.
    /// See `slot.forget()` for details.
    #[allow(clippy::missing_safety_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub unsafe fn forget_by_ref<S: Policy>(&self, slot: &mut Slot<T, S>) {
        let mut pool = self.0.borrow_mut();
        pool.free_entry(slot.0.as_ptr());
    }

    /// Takes an object out of the Pool and returns it. The slot at `slot` is put back to the
    /// freelist. This operation violates the Pin guarantees, thus in presence of pinned
    /// operation it must not be used.
    ///
    /// # Safety
    ///
    /// Slots must not be taken while references pointing to it.
    ///
    /// # Panics
    ///
    ///  * The object at slot is not initialized
    ///  * The object at slot was ever pinned
    ///  * The slot is already free
    ///  * The slot is invalid, not from this pool (debug only).
    #[inline]
    pub unsafe fn take<S: CanTakeValue>(&self, mut slot: Slot<T, S>) -> T {
        self.take_by_ref(&mut slot)
    }
}

/// Actual Pool implementations bits are behind a `RefCell`
pub(crate) struct PoolInner<T: Sized> {
    blocks: [Option<Block<T>>; NUM_BLOCKS],
    blocks_allocated: usize,
    min_entries: usize,
    in_use: usize,
    freelist: Option<NonNull<Entry<T>>>,
}

impl<T> PoolInner<T> {
    /// Allocate an entry, creating a new Block when required.
    fn alloc_entry(&mut self) -> NonNull<Entry<T>> {
        let entry = if let Some(mut entry) = self.freelist {
            // from freelist
            self.freelist = unsafe { entry.as_mut().remove_free_node() };
            entry
        } else {
            // from block
            if self.blocks_allocated == 0 {
                // allocate initial block
                self.blocks[0] = Some(Block::new_first(self.min_entries));
                self.blocks_allocated += 1;
            } else if unsafe {
                self.blocks
                    .get_unchecked(self.blocks_allocated - 1)
                    .as_ref()
                    .unwrap_unchecked()
                    .is_full()
            } {
                // allocate new block
                self.blocks[self.blocks_allocated] = Some(Block::new_next(unsafe {
                    self.blocks
                        .get_unchecked(self.blocks_allocated - 1)
                        .as_ref()
                        .unwrap_unchecked()
                }));
                self.blocks_allocated += 1;
            }

            unsafe {
                self.blocks
                    .get_unchecked_mut(self.blocks_allocated - 1)
                    .as_mut()
                    .unwrap_unchecked()
                    .extend()
            }
        };

        self.in_use += 1;
        entry
    }

    /// Put entry back into the freelist.
    ///
    /// # Safety
    ///
    ///  * The object must be already destructed (if possible)
    ///  * No references to the entry must exist
    ///
    /// This is internal, only called from Slot
    unsafe fn free_entry(&mut self, entry: *mut Entry<T>) {
        if let Some(freelist_last) = self.freelist {
            self.blocks[0..self.blocks_allocated]
                .iter()
                .rev()
                .map(|block| block.as_ref().unwrap_unchecked())
                .find(|block| block.contains_entry(entry))
                .map(|_| {
                    let list_node = freelist_last.as_ptr();
                    Entry::insert_free_node(list_node, entry);
                })
                .expect("Entry not in Pool");
        } else {
            Entry::init_free_node(entry);
        }
        self.in_use -= 1;
        self.freelist = Some(NonNull::new_unchecked(entry));
    }
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let _pool: Pool<String> = Pool::new();
    }

    #[test]
    fn leak() {
        let pool: Pool<u64> = Pool::new();
        let _ = pool.alloc(1234);
        pool.leak();
    }
}
