use std::mem::MaybeUninit;

use crate::*;

/// A Memory Pool holding objects of type T with a initial block size of E objects.
///
/// The pool can not track how many references to a slot are active. This makes all
/// `drop()`, `forget()` and `take()` unsafe. Thus they have to be carefully protected by RAII
/// guards or other means. Another approach is to use the *address* for all addressing and
/// convert to references only on demand and drop the reference as soon as possible.
///
/// The Pool API is pretty low-level and frequently unsafe is intended to be used to build
/// safe high level abstractions.
pub struct Pool<T: Sized, const E: usize> {
    blocks: [Option<Vec<Entry<T>>>; NUM_BLOCKS],
    blocks_allocated: usize,
    freelist: *mut Entry<T>,
}

impl<T, const E: usize> Pool<T, E> {
    /// Creates a new Pool for objects of type T with an initial block size of E.
    pub fn new() -> Self {
        debug_assert!(E > 0);
        Self {
            blocks: [(); NUM_BLOCKS].map(|_| None),
            blocks_allocated: 0,
            freelist: Entry::<T>::END_OF_FREELIST,
        }
    }

    /// Allocates a new entry, either from the freelist or by extending the pool.
    /// Returns Entry pointer tagged as UNINITIALIZED.
    fn alloc_entry(&mut self) -> *mut Entry<T> {
        if self.freelist != Entry::<T>::END_OF_FREELIST {
            // from freelist
            let entry = self.freelist;
            unsafe {
                debug_assert!(!Slot(entry).is_allocated(), "Invalid freelist");
                self.freelist = (*entry).descr;
                (*entry).descr = Entry::<T>::UNINITIALIZED_SENTINEL;
            }
            entry
        } else {
            if self.blocks_allocated == 0
                || self.blocks[self.blocks_allocated - 1]
                    .as_ref()
                    .unwrap()
                    .len()
                    == self.blocks[self.blocks_allocated - 1]
                        .as_ref()
                        .unwrap()
                        .capacity()
            {
                self.blocks[self.blocks_allocated] =
                    Some(Vec::with_capacity(E << self.blocks_allocated));
                self.blocks_allocated += 1;
            }

            let current_block = self.blocks[self.blocks_allocated - 1].as_mut().unwrap();
            current_block.push(Entry {
                data: MaybeUninit::uninit(),
                descr: Entry::<T>::UNINITIALIZED_SENTINEL,
            });
            current_block.last_mut().unwrap() as *mut Entry<T>
        }
    }

    /// Allocates a new slot from the pool, initializes it with the supplied object and
    /// returns a slot handle. Freeing the object can be done manually with
    /// `Pool::drop()` or `Pool::forget()`. The user must take care that the slot is not
    /// used after free as this may panic or return another object.
    #[must_use = "Slot is required for freeing memory, dropping it will leak"]
    pub fn alloc(&mut self, t: T) -> Slot<T> {
        let entry = self.alloc_entry();
        unsafe {
            (*entry).data = MaybeUninit::new(t);
            (*entry).descr = Entry::<T>::INITIALIZED_SENTINEL;
        }
        Slot(entry)
    }

    /// Allocates a new slot from the pool, keeps the content uninitialized returns its
    /// address in the pool. Freeing the object may be done manually with `Pool::drop()` or
    /// `Pool::forget()`. Otherwise the object will say around until the whole Pool becomes
    /// dropped. The user must take care that the provided address is not used after free as
    /// this may panic or return another object.
    #[must_use = "Slot is required for freeing memory, dropping it will leak"]
    pub fn alloc_uninit(&mut self) -> Slot<T> {
        Slot(self.alloc_entry())
    }

    /// Frees the slot at `slot` by calling its destructor when it contains an initialized
    /// object, uninitialized objects become forgotten as with `Pool::forget()`. Puts the
    /// given slot back into the freelist.
    ///
    /// # Safety
    ///
    /// Slots must not be dropped while references pointing to it.
    ///
    /// # Panics
    ///
    ///  * The slot is already free
    ///  * The slot is invalid, not from this pool (debug only).
    pub unsafe fn drop(&mut self, slot: Slot<T>) {
        debug_assert!(self.has_slot(&slot));
        assert!(slot.is_allocated());
        if slot.is_initialized() {
            (*slot.0).data.assume_init_drop();
        };
        (*slot.0).descr = self.freelist;
        self.freelist = slot.0;
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
    pub unsafe fn forget(&mut self, slot: Slot<T>) {
        debug_assert!(self.has_slot(&slot));
        assert!(slot.is_allocated());
        (*slot.0).descr = self.freelist;
        self.freelist = slot.0;
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
    ///  * The object at slot was ever pinned
    ///  * The slot is already free
    ///  * The slot is invalid, not from this pool (debug only).
    pub unsafe fn take(&mut self, slot: Slot<T>) -> T {
        debug_assert!(self.has_slot(&slot));
        assert!(slot.is_initialized() && !slot.is_pinned());
        (*slot.0).descr = self.freelist;
        self.freelist = slot.0;
        (*slot.0).data.assume_init_read()
    }

    // fn block_is_empty() -> bool {
    //     todo!()
    // }
    //
    // /// Returns true when the pool holds no data.
    // /// Useful for debugging/assertions
    // pub fn is_empty(&self) -> bool {
    //     todo!()
    // }

    /// Returns true when the slot is in self.
    pub fn has_slot(&self, slot: &Slot<T>) -> bool {
        for block in (0..self.blocks_allocated).rev() {
            if self.blocks[block].as_ref().unwrap()[..]
                .as_ptr_range()
                .contains(&(slot.0 as *const Entry<T>))
            {
                return true;
            }
        }
        false
    }
}

/// Convenient helper that calls `Pool::new()` with a optimized size for E.
///
/// For example:
/// ```
/// # use onsen::*;
/// struct Data(u64);
/// let pool = pool!(Data, PAGE);
/// ```
#[macro_export]
macro_rules! pool {
    ($TYPE:ty, $BLOCKSIZE:ident) => {
        Pool::< $TYPE, { <$TYPE>::$BLOCKSIZE}>::new()
    }
}

impl<T, const E: usize> Default for Pool<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

// Should be valid for C, but lets test this.
#[test]
fn entry_layout() {
    let e = Entry {
        data: MaybeUninit::new(String::from("Hello")),
        descr: std::ptr::null_mut(),
    };
    assert_eq!(
        (&e) as *const Entry<String> as usize,
        (&e.data) as *const MaybeUninit<String> as usize
    );
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let _pool: Pool<String, 128> = Pool::new();
    }
}
