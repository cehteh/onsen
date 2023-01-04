use std::cell::RefCell;
use std::fmt;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

use crate::*;

/// A single threaded, interior mutable memory Pool holding objects of type T.  Onsen Pools
/// obtain memory blocks from the global allocator. As long the Pool exists these blocks are
/// not given back to the allocator even when all entries are free. Only destruction of the
/// pool frees the associated blocks.
pub struct Pool<T: Sized>(RefCell<PoolInner<T>>);

impl<T> Pool<T> {
    /// Creates a new Pool for objects of type T.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(RefCell::new(PoolInner::new()))
    }
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for Pool<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_tuple("Pool").field(&self.0).finish()
    }
}

impl<T> PrivPoolApi<T> for Pool<T> {}

impl<T> PoolApi<T> for Pool<T> {}

impl<T> PoolLock<T> for &Pool<T> {
    #[inline]
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(self, f: F) -> R {
        f(&mut self.0.borrow_mut())
    }
}

/// Interior mutability of a pool.
#[doc(hidden)]
pub trait PoolLock<T> {
    fn with_lock<R, F: FnOnce(&mut PoolInner<T>) -> R>(self, f: F) -> R;
}

/// internal API
#[doc(hidden)]
pub trait PrivPoolApi<T>
where
    for<'a> &'a Self: PoolLock<T>,
    Self: Sized,
{
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
    for<'a> &'a Self: PoolLock<T>,
    Self: Sized,
{
    /// Configures the minimum number of entries the first block will hold. Must be called
    /// before the first allocation is made otherwise it will have no effect. Since blocks are
    /// allocated with exponentially growing size this should be reasonable small. The
    /// implementation will generously round this up to the next power of two. When not set it
    /// defaults to 64 entries.
    fn with_min_entries(&self, min_entries: usize) {
        self.with_lock(|pool| pool.min_entries = min_entries);
    }

    /// Destroys a Pool while leaking its allocated blocks.  The fast way out when one knows
    /// that allocations still exist and will never be returned to to the Pool. Either because
    /// the program exits or because the allocations are meant to stay.
    #[inline]
    fn leak(self) {
        std::mem::forget(self);
    }

    // PLANNED: try_alloc() with graceful backing off allocation and error handling

    /// Allocates a new `BasicBox` from this pool, initializes it with the supplied object.
    /// Freeing the object should be done manually with `pool.dealloc()`, `pool.forget()` or
    /// `pool.take()`. When a `BasicBox` is not deallocated, taken or forgotten as above then
    /// its it will leak until the Pool becomes dropped, this happens when panicking or might
    /// be intentional when the whole Pool becomes dropped at a later time.
    #[inline]
    fn alloc(&self, t: T) -> BasicBox<T> {
        let mut entry = self.alloc_entry();
        unsafe {
            entry.as_ptr().write(Entry {
                data: ManuallyDrop::new(t),
            });
            BasicBox::new(entry.as_mut())
        }
    }

    /// Frees `BasicBox` by calling its destructor. Puts the given memory back into the
    /// freelist.
    ///
    /// # Panics
    ///
    ///  * The `BasicBox` is not allocated from this pool.
    #[inline]
    fn dealloc(&self, mut bbox: BasicBox<T>) {
        self.with_lock(|pool| {
            bbox.assert_initialized();
            unsafe {
                pool.free_entry(bbox.manually_drop());
            }
        });
    }

    /// Free a `BasicBox` by calling its destructor. Puts the given memory back into the
    /// freelist. This function does not check if the object belongs to the pool. This makes
    /// it slightly faster but unsafe for that reason. Nevertheless many uses of `BasicBox`
    /// can guarantee this invariant because there is only one pool in use or the associated
    /// pool is stored along in a safe abstraction that keeps the `BasicBox`.
    ///
    /// # Safety
    ///
    ///  * The `BasicBox` must be allocated from this `Pool`, otherwise this is UB.
    #[inline]
    unsafe fn dealloc_unchecked(&self, mut bbox: BasicBox<T>) {
        self.with_lock(|pool| {
            pool.fast_free_entry_unchecked(bbox.manually_drop());
        });
    }

    /// Puts the given slot back into the freelist. Will not call the the destructor.
    ///
    /// # Panics
    ///
    ///  * The `BasicBox` is not allocated from this pool
    #[inline]
    fn forget(&self, mut bbox: BasicBox<T>) {
        self.with_lock(|pool| unsafe {
            pool.free_entry(bbox.take_entry());
        });
    }

    /// Takes an object out of the Pool and returns it. The `BasicBox` is put back to the
    /// freelist.
    ///
    /// # Panics
    ///
    ///  * The `BasicBox` is not allocated from this pool
    #[inline]
    fn take(&self, mut bbox: BasicBox<T>) -> T {
        self.with_lock(|pool| unsafe {
            let ret = bbox.take();
            pool.free_entry(bbox.take_entry());
            ret
        })
    }
}

/// Actual Pool implementations bits which need protected access
#[doc(hidden)]
pub struct PoolInner<T: Sized> {
    blocks: [Option<Block<T>>; NUM_BLOCKS],
    blocks_allocated: usize,
    min_entries: usize,
    freelist: Option<NonNull<Entry<T>>>,
}

unsafe impl<T: Sized + Send> Send for PoolInner<T> {}

impl<T> PoolInner<T> {
    pub(crate) const fn new() -> Self {
        Self {
            // blocks: [(); NUM_BLOCKS].map(|_| None),  // doesn't work in constfn :/

            // TODO:  https://github.com/rust-lang/rust/issues/76001
            // becomes:  blocks: [const { None }; NUM_BLOCKS],
            blocks: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None,
            ],
            blocks_allocated: 0,
            min_entries: 64,
            freelist: None,
        }
    }

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

        entry
    }

    /// Put entry back into the freelist.
    ///
    /// # Panics
    ///
    ///  * `entry` is not in Pool `self`
    ///
    /// # Safety
    ///
    ///  * The object must be already destructed (if possible)
    unsafe fn free_entry(&mut self, entry: &mut Entry<T>) {
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
        self.freelist = Some(NonNull::new_unchecked(entry));
    }

    /// Put entry back into the freelist. This does not check if entry belongs to the
    /// Pool which makes the freeing considerably faster.
    ///
    /// # Safety
    ///
    ///  * `entry` must be already destructed (if possible)
    ///  * `entry` must be be from Pool `self`
    pub(crate) unsafe fn fast_free_entry_unchecked(&mut self, entry: &mut Entry<T>) {
        if let Some(freelist_last) = self.freelist {
            Entry::insert_free_node(freelist_last.as_ptr(), entry);
        } else {
            Entry::init_free_node(entry);
        }
        self.freelist = Some(entry.into());
    }

    fn freelist_len(&self) -> usize {
        let mut len = 0;
        if self.freelist.is_some() {
            len += 1;
            let start = self.freelist.unwrap().as_ptr();
            let mut entry = start;
            unsafe {
                while (*entry).freelist_node.next != start {
                    len += 1;
                    entry = (*entry).freelist_node.next;
                }
            }
        }

        len
    }
}

impl<T> fmt::Debug for PoolInner<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("PoolInner")
            .field("blocks", &self.blocks)
            .field("blocks_allocated", &self.blocks_allocated)
            .field("min_entries", &self.min_entries)
            .field("freelist.len()", &self.freelist_len())
            .finish()
    }
}

#[cfg(test)]
mod pool_tests {
    use crate::*;

    #[test]
    fn smoke() {
        let _pool: Pool<String> = Pool::new();
    }
}
