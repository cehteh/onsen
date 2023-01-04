use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;
use std::ptr::NonNull;

use crate::*;

// Hugepages can be at most 1GB on current architectures, this is the largest alignment that
// makes sense.
const MAX_ALIGN: usize = 1073741824usize;

/// The low level memory blocks.
pub(crate) struct Block<T: Sized> {
    /// Pointer to an `[Entry<T>, capacity]` with the first `len_used` entries in use.
    memory: NonNull<Entry<T>>,
    len_used: usize,
    capacity: usize,
    layout: Layout,
}

impl<T: Sized> Block<T> {
    // internal ctor
    fn new(capacity: usize) -> Self {
        let layout = Layout::array::<Entry<T>>(capacity).unwrap();
        let layout = layout
            .align_to(std::cmp::min(layout.size().next_power_of_two(), MAX_ALIGN))
            .unwrap();

        let memory =
            unsafe { NonNull::new(alloc(layout) as *mut Entry<T>).expect("Allocation failure") };

        Self {
            memory,
            len_used: 0,
            capacity,
            layout,
        }
    }

    /// Create a new first block, takes `min_entries` as hint for the initial blocksize
    /// calculation to contain at least this much entries.
    pub(crate) fn new_first(min_entries: usize) -> Self {
        let min_entries = std::cmp::max(64, min_entries);

        // generous rounding to next power of two
        let blocksize =
            (min_entries * size_of::<Entry<T>>()).next_power_of_two() / size_of::<Entry<T>>();
        Self::new(blocksize)
    }

    /// Create a sucessor block with twice the size than `self`.
    pub(crate) fn new_next(&self) -> Self {
        let blocksize =
            (self.capacity * 2 * size_of::<Entry<T>>()).next_power_of_two() / size_of::<Entry<T>>();
        Self::new(blocksize)
    }

    /// Get a slice to the used part of the bitmap
    fn entries(&self) -> &[Entry<T>] {
        unsafe { std::slice::from_raw_parts(self.memory.as_ptr(), self.len_used) }
    }

    /// Get a mutable slice to the used part of the bitmap
    fn entries_mut(&mut self) -> &mut [Entry<T>] {
        unsafe { std::slice::from_raw_parts_mut(self.memory.as_mut(), self.len_used) }
    }

    /// returns true when a blocks capacity is exhausted
    pub(crate) fn is_full(&self) -> bool {
        self.len_used == self.capacity
    }

    /// gets one entry from the unused capacity, panics when the block is full (in debug mode)
    pub(crate) fn extend(&mut self) -> NonNull<Entry<T>> {
        debug_assert!(self.len_used < self.capacity);
        let pos = self.len_used;
        self.len_used += 1;
        // Safety: checked len_used < capacity
        unsafe { NonNull::new_unchecked(self.entries_mut().get_unchecked_mut(pos)) }
    }

    /// returns true when entry belongs to self
    pub(crate) fn contains_entry(&self, entry: *mut Entry<T>) -> bool {
        self.entries()
            .as_ptr_range()
            .contains(&(entry as *const Entry<T>))
    }
}

impl<T> Drop for Block<T> {
    fn drop(&mut self) {
        unsafe { dealloc(self.memory.as_ptr() as *mut u8, self.layout) };
    }
}

use std::fmt;
impl<T> fmt::Debug for Block<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("Block")
            .field("len_used", &self.len_used)
            .field("capacity", &self.capacity)
            .field("layout", &self.layout)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let block: Block<String> = Block::new_first(0);
        let _block2 = Block::new_next(&block);
    }
}
