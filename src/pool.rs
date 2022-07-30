use std::cell::RefCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

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
pub struct Pool<T: Sized, const E: usize> {
    blocks: RefCell<[Option<Vec<Entry<T>>>; NUM_BLOCKS]>,
    blocks_allocated: AtomicUsize,
    freelist: RefCell<*mut Entry<T>>,
    in_use: AtomicUsize,
}

impl<T, const E: usize> Pool<T, E> {
    /// Creates a new Pool for objects of type T with an initial block size of E.
    #[inline]
    pub fn new() -> Self {
        debug_assert!(E > 0);
        Self {
            blocks: RefCell::new([(); NUM_BLOCKS].map(|_| None)),
            blocks_allocated: AtomicUsize::new(0),
            freelist: RefCell::new(Entry::<T>::END_OF_FREELIST),
            in_use: AtomicUsize::new(0),
        }
    }

    /// Allocates a new entry, either from the freelist or by extending the pool.
    /// Returns Entry pointer tagged as UNINITIALIZED.
    fn alloc_entry(&self) -> *mut Entry<T> {
        let mut freelist = self.freelist.borrow_mut();
        if *freelist != Entry::<T>::END_OF_FREELIST {
            // from freelist
            let entry = *freelist;
            unsafe {
                debug_assert!(!(&*entry).is_allocated(), "Invalid freelist");
                *freelist = (*entry).descr;
                (*entry).descr = Entry::<T>::UNINITIALIZED_SENTINEL;
            }
            self.in_use.fetch_add(1, Relaxed);
            entry
        } else {
            let blocks_allocated = self.blocks_allocated.load(Relaxed);
            let mut blocks = self.blocks.borrow_mut();
            if self.blocks_allocated.load(Relaxed) == 0
                || blocks[blocks_allocated - 1].as_ref().unwrap().len()
                    == blocks[blocks_allocated - 1].as_ref().unwrap().capacity()
            {
                blocks[blocks_allocated] = Some(Vec::with_capacity(E << blocks_allocated));
                self.blocks_allocated.fetch_add(1, Relaxed);
            }

            let blocks_allocated = self.blocks_allocated.load(Relaxed);
            let current_block = blocks[blocks_allocated - 1].as_mut().unwrap();
            current_block.push(Entry {
                data: MaybeUninit::uninit(),
                descr: Entry::<T>::UNINITIALIZED_SENTINEL,
            });
            self.in_use.fetch_add(1, Relaxed);
            current_block.last_mut().unwrap() as *mut Entry<T>
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
            (*entry).data = MaybeUninit::new(t);
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
        debug_assert!(self.has_slot(slot));
        assert!(slot.is_allocated());
        let mut freelist = self.freelist.borrow_mut();
        if slot.is_initialized() {
            (*slot.0).data.assume_init_drop();
        };
        (*slot.0).descr = *freelist;
        *freelist = slot.0;
        self.in_use.fetch_sub(1, Relaxed);
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
        debug_assert!(self.has_slot(slot));
        assert!(slot.is_allocated());
        let mut freelist = self.freelist.borrow_mut();
        (*slot.0).descr = *freelist;
        *freelist = slot.0;
        self.in_use.fetch_sub(1, Relaxed);
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
        debug_assert!(self.has_slot(slot));
        assert!(slot.is_initialized() && !slot.is_pinned());
        let mut freelist = self.freelist.borrow_mut();
        (*slot.0).descr = *freelist;
        *freelist = slot.0;
        self.in_use.fetch_sub(1, Relaxed);
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
        let blocks_allocated = self.blocks_allocated.load(Relaxed);
        for block in (0..blocks_allocated).rev() {
            if self.blocks.borrow()[block].as_ref().unwrap()[..]
                .as_ptr_range()
                .contains(&(slot.0 as *const Entry<T>))
            {
                return true;
            }
        }
        false
    }

    /// Destroys a Pool while leaking its allocated blocks.  The fast way out when one knows
    /// that allocations still exist and will never be returned to to the Pool. Either because
    /// the program exits in some fast way or because the allocations are meant to stay
    /// static.
    #[inline]
    pub fn leak(self) {
        std::mem::forget(self);
    }
}

impl<T, const E: usize> Drop for Pool<T, E> {
    #[cfg(debug_assertions)]
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert_eq!(
                self.in_use.load(Relaxed),
                0,
                "Dropping Pool while Slots are still in use"
            );
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
        Pool::<$TYPE, { <$TYPE>::$BLOCKSIZE }>::new()
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
