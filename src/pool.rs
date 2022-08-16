use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

use crate::*;

/// A Memory Pool holding objects of type T with a initial block size of E objects.
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
    /// Creates a new Pool for objects of type T with an initial block size of E.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(RefCell::new(PoolInner {
            blocks: [(); NUM_BLOCKS].map(|_| None),
            blocks_allocated: 0,
            min_entries: 0,
            in_use: 0,
            freelist: None,
        }))
    }

    /// Configures the minimum of entries the first block will hold. Must be called before the
    /// first allocation is made, otherwise it has no effect. Can be used when the number of
    /// entries that will be used is roughly guessable and or the size of entries is small.
    /// Setting this improves cache locality.  Since blocks are allocated with exponentially
    /// growing size this should be still small enough, approx 1/4 of the average number of
    /// entries to be expected. The implementation will generously round this up to the next
    /// power of two.
    pub fn with_min_entries(&self, min_entries: usize) {
        self.0.borrow_mut().min_entries = min_entries;
    }

    /// Allocates a new entry, either from the freelist or by extending the pool.
    /// Returns Entry pointer tagged as UNINITIALIZED.
    fn alloc_entry(&self) -> NonNull<Entry<T>> {
        self.0.borrow_mut().alloc_entry()
    }

    /// Allocates a new slot from the pool, initializes it with the supplied object and
    /// returns a slot handle. Freeing the object can be done manually with `pool.free()`,
    /// `pool::forget()` or `pool.take()`. The user must take care that the slot is not used
    /// after free as this may panic or return another object.
    #[must_use = "Slot is required for freeing memory, dropping it will leak"]
    #[inline]
    pub fn alloc(&self, t: T) -> Slot<T> {
        let mut entry = self.alloc_entry();
        unsafe {
            entry.as_mut().maybe_data = MaybeData {
                data: ManuallyDrop::new(t),
            };
            entry.as_mut().descriptor = Descriptor::Initialized;
        }
        Slot::new(entry)
    }

    /// Allocates a new slot from the pool, keeps the content uninitialized returns its
    /// address in the pool. Freeing the object may be done manually with `pool.free()` or
    /// `pool.forget()`. Otherwise the object will say around until the whole Pool becomes
    /// dropped. The user must take care that the provided address is not used after free as
    /// this may panic or return another object.
    #[must_use = "Slot is required for freeing memory, dropping it will leak"]
    #[inline]
    pub fn alloc_uninit(&self) -> Slot<T> {
        Slot::new(self.alloc_entry())
    }

    /// Frees the slot at `slot` by calling its destructor when it contains an initialized
    /// object, uninitialized objects become forgotten as with `Pool::forget()`. Puts the
    /// given slot back into the freelist.
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
    pub unsafe fn free(&self, mut slot: Slot<T>) {
        self.free_by_ref(&mut slot);
    }

    /// Non consuming variant of `pool.free()`, allows freeing slots that are part of other
    /// structures while keeping Slot non-Copy. The slot must not be used after this.
    /// See `slot.free()` for details.
    #[allow(clippy::missing_safety_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub unsafe fn free_by_ref(&self, slot: &mut Slot<T>) {
        let mut pool = self.0.borrow_mut();
        if slot.is_initialized() {
            ManuallyDrop::drop(&mut slot.0.as_mut().maybe_data.data);
        };
        assert!(
            pool.free_entry(slot.0.as_ptr()),
            "Slot does not belong to pool"
        );
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
    pub unsafe fn forget(&self, mut slot: Slot<T>) {
        self.forget_by_ref(&mut slot);
    }

    /// Non consuming variant of `pool.forget()`, allows forgetting slots that are part of
    /// other structures while keeping Slot non-Copy.  The slot must not be used after this.
    /// See `slot.forget()` for details.
    #[allow(clippy::missing_safety_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub unsafe fn forget_by_ref(&self, slot: &mut Slot<T>) {
        let mut pool = self.0.borrow_mut();
        assert!(
            pool.free_entry(slot.0.as_ptr()),
            "Slot does not belong to pool"
        );
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
    pub unsafe fn take(&self, mut slot: Slot<T>) -> T {
        self.take_by_ref(&mut slot)
    }

    /// Non consuming variant of `pool.take()`, allows taking slots that are part of other
    /// structures while keeping Slot non-Copy.  The slot must not be used after this.  See
    /// `slot.take()` for details.
    #[allow(clippy::missing_safety_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub unsafe fn take_by_ref(&self, slot: &mut Slot<T>) -> T {
        let mut pool = self.0.borrow_mut();
        assert!(slot.is_initialized() && !slot.is_pinned());
        let ret = ManuallyDrop::take(&mut slot.0.as_mut().maybe_data.data);
        assert!(
            pool.free_entry(slot.0.as_ptr()),
            "Slot does not belong to pool"
        );
        ret
    }

    /// Destroys a Pool while leaking its allocated blocks.  The fast way out when one knows
    /// that allocations still exist and will never be returned to to the Pool. Either because
    /// the program exits in some fast way or because the allocations are meant to stay.
    #[inline]
    pub fn leak(self) {
        std::mem::forget(self);
    }
}

pub(crate) struct PoolInner<T: Sized> {
    blocks: [Option<Block<T>>; NUM_BLOCKS],
    blocks_allocated: usize,
    min_entries: usize,
    in_use: usize,
    freelist: Option<NonNull<Entry<T>>>,
}

impl<T> PoolInner<T> {
    // Allocate an entry, creating a new Block when required.
    fn alloc_entry(&mut self) -> NonNull<Entry<T>> {
        let mut entry = if let Some(mut entry) = self.freelist {
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

        unsafe { entry.as_mut().descriptor = Descriptor::Uninitialized };
        self.in_use += 1;
        entry
    }

    // Put entry back into the freelist. Returns 'false' when the entry does
    // not belong to this Pool.
    unsafe fn free_entry(&mut self, entry: *mut Entry<T>) -> bool {
        for i in (0..self.blocks_allocated).rev() {
            let block = self.blocks[i].as_mut().unwrap_unchecked();

            if block.contains_entry(entry) {
                debug_assert!(!Entry::ptr_is_free(entry));
                match self.freelist {
                    // first node, cyclic pointing to itself
                    None => Entry::init_free_node(entry),
                    Some(freelist_last) => {
                        let list_node = freelist_last.as_ptr();
                        Entry::insert_free_node(list_node, entry);
                    }
                }
                (*entry).descriptor = Descriptor::Free;
                self.in_use -= 1;
                self.freelist = Some(NonNull::new_unchecked(entry));
                return true;
            }
        }

        false
    }
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Should be valid for C, but lets test this.
#[test]
fn entry_layout() {
    let e = Entry {
        maybe_data: MaybeData {
            data: ManuallyDrop::new(String::from("Hello")),
        },
        descriptor: Descriptor::Uninitialized,
    };
    assert_eq!(
        (&e) as *const Entry<String> as usize,
        (&e.maybe_data) as *const MaybeData<String> as usize
    );
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let _pool: Pool<String> = Pool::new();
    }
}
