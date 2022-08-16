use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;
use std::ptr::NonNull;

use crate::*;

/// The low level memory blocks and bitmaps.
///
/// PANICS: One must not drop blocks while they are still in use. In debug mode this
/// panics. In release the memory will be leaked to maintain memory safety.
/// This emergency leaking is only there to prevent UB, it is not the intended
/// use! There are two mechanisms to track the free entries.
///
/// In case fast application shutdown is important one can explicitly leak the memory.
pub(crate) struct Block<T: Sized> {
    /// Pointer to an `[*mut Entry<T>, capacity]` with the first `len_used` entries in use.
    memory: NonNull<Entry<T>>,
    len_used: usize,
    capacity: usize,
    layout: Layout,
}

impl<T: Sized> Block<T> {
    // internal ctor
    fn new(capacity: usize) -> Self {
        let layout = Layout::array::<Entry<T>>(capacity).unwrap();
        let layout = layout.align_to(layout.size().next_power_of_two()).unwrap();

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
        // slightly lossy calculation, might be better packed at bigger blocks but at soon the
        // loss spans more than one page the kernel won't map the memory anyway.
        let blocksize =
            (self.capacity * 2 * size_of::<Entry<T>>()).next_power_of_two() / size_of::<Entry<T>>();
        Self::new(blocksize)
    }

    /// Destroys the Block, leaking its allocations.
    #[inline]
    #[allow(dead_code)]
    pub fn leak(self) {
        std::mem::forget(self);
    }

    // TODO: test if making the accessors unsafe over the index, caller needs to provide correct index

    /// Get a slice to the used part of the bitmap
    fn entries(&self) -> &[Entry<T>] {
        unsafe { std::slice::from_raw_parts(self.memory.as_ptr(), self.len_used) }
    }

    /// Get a mutable slice to the used part of the bitmap
    fn entries_mut(&mut self) -> &mut [Entry<T>] {
        unsafe { std::slice::from_raw_parts_mut(self.memory.as_mut(), self.len_used) }
    }

    // returns true when a blocks capacity is exhausted
    #[inline]
    pub(crate) fn is_full(&self) -> bool {
        self.len_used == self.capacity
    }

    // gets one entry from the unused capacity, panics when the block is full
    pub(crate) fn extend(&mut self) -> NonNull<Entry<T>> {
        debug_assert!(self.len_used < self.capacity);
        let pos = self.len_used;
        self.len_used += 1;
        // Safety: checked len_used < capacity, valid entry
        unsafe { NonNull::new_unchecked(self.entries_mut().get_unchecked_mut(pos)) }
    }

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

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn smoke() {
        let block: Block<String> = Block::new_first(0);
        let _block2 = Block::new_next(&block);
    }
}
