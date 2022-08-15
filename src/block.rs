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
    /// Pointer to an bitmap with one bit for each entry in 'memory'. A set bit means that this entry is free.
    freelist: Option<NonNull<Entry<T>>>,
    in_use: usize,
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
            freelist: None,
            in_use: 0,
            len_used: 0,
            capacity,
            layout,
        }
    }

    /// Create a new first block, takes min_entries as hint for the initial blocksize
    /// calculation to contain at least this much entries.
    pub(crate) fn new_first(min_entries: usize) -> Self {
        let min_entries = std::cmp::max(1, min_entries); // at least one
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

    // gets one entry from the unused capacity, returns None when the block is full
    fn extend(&mut self) -> Option<NonNull<Entry<T>>> {
        if self.len_used < self.capacity {
            let pos = self.len_used;
            self.len_used += 1;
            self.in_use += 1;

            // Safety: checked len_used < capacity
            let entry = unsafe { self.entries_mut().get_unchecked_mut(pos) };
            entry.descriptor = Descriptor::Uninitialized;
            Some(unsafe { NonNull::new_unchecked(entry) })
        } else {
            None
        }
    }

    pub fn alloc_entry(&mut self) -> Option<NonNull<Entry<T>>> {
        if let Some(mut entry) = self.freelist {
            unsafe {
                self.freelist = entry.as_mut().remove_free_node();
                entry.as_mut().descriptor = Descriptor::Uninitialized;
            }
            self.in_use += 1;
            Some(entry)
        } else {
            self.extend()
        }
    }

    fn entry_index(&self, entry: *mut Entry<T>) -> Option<usize> {
        if self
            .entries()
            .as_ptr_range()
            .contains(&(entry as *const Entry<T>))
        {
            Some(unsafe { entry.offset_from(self.memory.as_ptr()) as usize })
        } else {
            None
        }
    }

    /// returns true when any entry is still in use
    pub fn in_use(&self) -> bool {
        self.in_use > 0
    }

    /// Frees an entry, putting it back into the freelist, returns false when the entry
    /// doesn't belong to this block
    pub(crate) unsafe fn free_entry(&mut self, entry: *mut Entry<T>) -> bool {
        if self.entry_index(entry).is_some() {
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
            true
        } else {
            false
        }
    }
}

impl<T> Drop for Block<T> {
    #[cfg(debug_assertions)]
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert!(!self.in_use(), "Dropping Pool while Slots are still in use");
        };
        unsafe { dealloc(self.memory.as_ptr() as *mut u8, self.layout) };
    }

    #[cfg(not(debug_assertions))]
    fn drop(&mut self) {
        if !self.in_use() {
            // leak in release mode to enforce memory safety. This non intentional leaking is
            // not supported, use explicit leaking. This is only here to ensure safety as last resort.
            unsafe { dealloc(self.memory.as_ptr() as *mut u8, self.layout) };
        }
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

    #[test]
    fn extend() {
        let mut block: Block<String> = Block::new_first(10);
        let _ = block.alloc_entry().unwrap();
        let _ = block.alloc_entry().unwrap();
        let _ = block.alloc_entry().unwrap();
        let _ = block.alloc_entry().unwrap();
        let _ = block.alloc_entry().unwrap();
        let _ = block.alloc_entry().unwrap();
        block.leak();
    }

    #[test]
    fn alloc_free() {
        let mut block: Block<String> = Block::new_first(0);
        let entry = block.alloc_entry().unwrap();
        assert!(unsafe { block.free_entry(entry.as_ptr()) });
    }
}
