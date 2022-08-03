use std::cell::RefCell;
use std::mem::ManuallyDrop;

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
pub struct Pool<T: Sized, const E: usize>(pub(crate) RefCell<PoolInner<T, E>>);

impl<T, const E: usize> Pool<T, E> {
    /// Creates a new Pool for objects of type T with an initial block size of E.
    #[inline]
    pub fn new() -> Self {
        debug_assert!(E > 0);
        Self(RefCell::new(PoolInner {
            blocks: [(); NUM_BLOCKS].map(|_| None),
            blocks_allocated: 0,
            freelist: Entry::<T>::END_OF_FREELIST,
            in_use: 0,
        }))
    }

    /// Allocates a new entry, either from the freelist or by extending the pool.
    /// Returns Entry pointer tagged as UNINITIALIZED.
    fn alloc_entry(&self) -> *mut Entry<T> {
        let mut pool = self.0.borrow_mut();
        if pool.freelist != Entry::<T>::END_OF_FREELIST {
            // from freelist
            let entry = pool.freelist;
            unsafe {
                debug_assert!(!(&*entry).is_allocated(), "Invalid freelist");
                pool.freelist = (*entry).descr;
                (*entry).descr = Entry::<T>::UNINITIALIZED_SENTINEL;
            }
            pool.in_use = pool.in_use.wrapping_add(1); // can never overflow
            entry
        } else {
            let blocks_allocated_minus_1 = pool.blocks_allocated.wrapping_sub(1);
            if pool.blocks_allocated == 0
                || unsafe {
                    // Safety: blocks_allocated_minus_1 will not be used on underflow
                    let block = pool
                        .blocks
                        .get_unchecked(blocks_allocated_minus_1)
                        .as_ref()
                        .unwrap();

                    block.len() == block.capacity()
                }
            {
                let blocks_allocated = pool.blocks_allocated;
                pool.blocks[blocks_allocated] =
                    Some(Vec::with_capacity(E << pool.blocks_allocated));
                pool.blocks_allocated = pool.blocks_allocated.wrapping_add(1);
            }

            pool.in_use = pool.in_use.wrapping_add(1); // can never overflow
            let blocks_allocated_minus_1 = pool.blocks_allocated.wrapping_sub(1);
            let block = pool.blocks[blocks_allocated_minus_1].as_mut().unwrap();

            block.push(Entry {
                maybe_data: MaybeData { uninit: () },
                descr: Entry::<T>::UNINITIALIZED_SENTINEL,
            });
            block.last_mut().unwrap() as *mut Entry<T>
        }
    }

    /// Allocates a new slot from the pool, initializes it with the supplied object and
    /// returns a slot handle. Freeing the object can be done manually with `pool.free()`,
    /// `pool::forget()` or `pool.take()`. The user must take care that the slot is not used
    /// after free as this may panic or return another object.
    #[must_use = "Slot is required for freeing memory, dropping it will leak"]
    #[inline]
    pub fn alloc(&self, t: T) -> Slot<T> {
        let entry = self.alloc_entry();
        unsafe {
            (*entry).maybe_data = MaybeData {
                data: ManuallyDrop::new(t),
            };
            (*entry).descr = Entry::<T>::INITIALIZED_SENTINEL;
        }
        Slot(entry)
    }

    /// Allocates a new slot from the pool, keeps the content uninitialized returns its
    /// address in the pool. Freeing the object may be done manually with `pool.free()` or
    /// `pool.forget()`. Otherwise the object will say around until the whole Pool becomes
    /// dropped. The user must take care that the provided address is not used after free as
    /// this may panic or return another object.
    #[must_use = "Slot is required for freeing memory, dropping it will leak"]
    #[inline]
    pub fn alloc_uninit(&self) -> Slot<T> {
        Slot(self.alloc_entry())
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
    pub unsafe fn free(&self, slot: Slot<T>) {
        self.free_by_ref(&slot);
    }

    /// Non consuming variant of `pool.free()`, allows freeing slots that are part of other
    /// structures while keeping Slot non-Copy. The slot must not be used after this.
    /// See `slot.free()` for details.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn free_by_ref(&self, slot: &Slot<T>) {
        let mut pool = self.0.borrow_mut();
        debug_assert!(pool.has_slot(slot));
        assert!(slot.is_allocated());
        if slot.is_initialized() {
            ManuallyDrop::drop(&mut (*slot.0).maybe_data.data);
        };
        (*slot.0).descr = pool.freelist;
        pool.freelist = slot.0;
        pool.in_use = pool.in_use.wrapping_sub(1); // can never underflow
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
    pub unsafe fn forget(&self, slot: Slot<T>) {
        self.forget_by_ref(&slot);
    }

    /// Non consuming variant of `pool.forget()`, allows forgetting slots that are part of
    /// other structures while keeping Slot non-Copy.  The slot must not be used after this.
    /// See `slot.forget()` for details.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn forget_by_ref(&self, slot: &Slot<T>) {
        let mut pool = self.0.borrow_mut();
        debug_assert!(pool.has_slot(slot));
        assert!(slot.is_allocated());
        (*slot.0).descr = pool.freelist;
        pool.freelist = slot.0;
        pool.in_use = pool.in_use.wrapping_sub(1); // can never underflow
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
    pub unsafe fn take(&self, slot: Slot<T>) -> T {
        self.take_by_ref(&slot)
    }

    /// Non consuming variant of `pool.take()`, allows taking slots that are part of other
    /// structures while keeping Slot non-Copy.  The slot must not be used after this.  See
    /// `slot.take()` for details.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn take_by_ref(&self, slot: &Slot<T>) -> T {
        let mut pool = self.0.borrow_mut();
        debug_assert!(pool.has_slot(slot));
        assert!(slot.is_initialized() && !slot.is_pinned());
        (*slot.0).descr = pool.freelist;
        pool.freelist = slot.0;
        pool.in_use = pool.in_use.wrapping_sub(1); // can never underflow
        ManuallyDrop::take(&mut (*slot.0).maybe_data.data)
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

    /// Destroys a Pool while leaking its allocated blocks.  The fast way out when one knows
    /// that allocations still exist and will never be returned to to the Pool. Either because
    /// the program exits in some fast way or because the allocations are meant to stay
    /// static.
    #[inline]
    pub fn leak(self) {
        std::mem::forget(self);
    }
}

pub(crate) struct PoolInner<T: Sized, const E: usize> {
    blocks: [Option<Vec<Entry<T>>>; NUM_BLOCKS],
    blocks_allocated: usize,
    freelist: *mut Entry<T>,
    in_use: usize,
}

impl<T, const E: usize> PoolInner<T, E> {
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

impl<T, const E: usize> Drop for PoolInner<T, E> {
    #[cfg(debug_assertions)]
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert_eq!(self.in_use, 0, "Dropping Pool while Slots are still in use");
        }
    }

    #[cfg(not(debug_assertions))]
    fn drop(&mut self) {
        for block in 0..self.blocks_allocated {
            self.blocks[block].take().map(|v| v.leak());
        }
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
        $crate::Pool::<$TYPE, { <$TYPE as $crate::OptimalBlockSize>::$BLOCKSIZE }>::new()
    };
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
        maybe_data: MaybeData {
            data: ManuallyDrop::new(String::from("Hello")),
        },
        descr: std::ptr::null_mut(),
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
        let _pool: Pool<String, 128> = Pool::new();
    }
}
