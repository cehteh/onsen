use std::fmt;
use std::ptr::NonNull;

use crate::*;

/// Actual Pool implementations bits which need protected access
#[doc(hidden)]
pub struct PoolInner<T: Sized> {
    blocks: [Option<Block<T>>; NUM_BLOCKS],
    blocks_allocated: usize,
    pub(crate) min_entries: usize,
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
}

impl<T> PoolInner<T> {
    pub(crate) fn min_entries(&mut self, min_entries: usize) {
        self.min_entries = min_entries;
    }

    /// Allocate an entry, creating a new Block when required.
    pub(crate) fn alloc_entry(&mut self) -> NonNull<Entry<T>> {
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
    pub(crate) unsafe fn free_entry(&mut self, entry: &mut Entry<T>) {
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

    /// Diagnostics returning a (used+free, capacity) tuple
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn reserved(&self) -> (usize, usize) {
        self.blocks[0..self.blocks_allocated]
            .iter()
            .fold((0, 0), |(used, capacity), block| {
                let (block_used, block_capacity) = block.as_ref().unwrap().reserved();
                (used + block_used, capacity + block_capacity)
            })
    }

    /// Diagnostics returning a (used, free, capacity) tuple. This function is rather
    /// expensive because it walks the freelist!
    #[must_use]
    pub fn stat(&self) -> (usize, usize, usize) {
        let (reserved, capacity) = self.reserved();
        let free = self.freelist_len();
        (reserved - free, free, capacity)
    }

    /// Diagnostics checking that no allocations are active. This can be used to check that no
    /// `UnsafeBox` outlived the Pool before it becomes dropped. This is expensive because it
    /// calls `stat()`
    #[must_use]
    pub fn is_all_free(&self) -> bool {
        let (used, _, _) = self.stat();
        used == 0
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
