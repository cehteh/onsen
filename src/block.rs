use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;

use crate::Entry;

/// The low level memory blocks and bitmaps.
///
/// PANICS: One must not drop blocks while they are still in use. In debug mode this
/// panics. In release the memory will be leaked to maintain memory safety.
///
/// Dropping blocks is slightly more expensive because of the check above. The expected
/// use-case is that pools are rarely dropped. For program termination one can consider
/// leaking the blocks explicitly.
pub(crate) struct Block<T: Sized> {
    /// Pointer to an `[*mut Entry<T>, capacity]` with the first `len_used` entries in use.
    memory: *mut Entry<T>,
    /// Pointer to an bitmap with one bit for each entry in 'memory'. A set bit means that this entry is free.
    bitmap: *mut usize,
    freelist: *mut Entry<T>,
    #[cfg(not(feature = "bitmap_scan"))]
    in_use: usize,
    len_used: usize,
    capacity: usize,
    layout: Layout,
}

impl<T: Sized> Block<T> {
    // internal ctor
    fn new(capacity: usize) -> Self {
        let entries_layout = Layout::array::<Entry<T>>(capacity).unwrap();
        let bitmap_layout = Layout::array::<usize>(capacity).unwrap();

        let (layout, bitmap_offset) = entries_layout.extend(bitmap_layout).unwrap();
        let layout = layout.align_to(layout.size().next_power_of_two()).unwrap();

        let memory = unsafe { alloc(layout) };
        assert!(!memory.is_null());
        let bitmap = unsafe { memory.add(bitmap_offset) } as *mut usize;

        Self {
            memory: memory as *mut Entry<T>,
            bitmap,
            freelist: Entry::<T>::END_OF_FREELIST,
            #[cfg(not(feature = "bitmap_scan"))]
            in_use: 0,
            len_used: 0,
            capacity,
            layout,
        }
    }

    /// Create a new first block, takes min_entries as hint for the initial blocksize
    /// calculation to contain at least this much entries.
    pub(crate) fn new_first(min_entries: usize) -> Self {
        let min_entries = std::cmp::max(1, (min_entries + BITMAP_WORD_BITS - 1) / BITMAP_WORD_BITS);
        // as much entries as filling a single word in the bitmap
        let min_entries = min_entries * BITMAP_WORD_BITS * size_of::<Entry<T>>()
            + min_entries * BITMAP_WORD_BITS / 8;
        // round up to the next power of 2, that is our blocksize
        let initial_blocksize = min_entries.next_power_of_two();
        Self::new(initial_blocksize * BITMAP_WORD_BITS / min_entries)
    }

    /// Create a sucessor block with twice the size than `self`.
    pub(crate) fn new_next(&self) -> Self {
        // slightly lossy calculation, might be better packed at bigger blocks but at soon the
        // loss spans more than one page the kernel won't map the memory anyway.
        // TODO: improve calculation, increase capacity while it fits
        Self::new(self.capacity * 2)
    }

    /// Destroys the Block, leaking its allocations.
    #[inline]
    #[allow(dead_code)]
    pub fn leak(self) {
        std::mem::forget(self);
    }

    /// Get a slice to the initialized part of the bitmap
    fn bitmap(&self) -> &[usize] {
        unsafe {
            std::slice::from_raw_parts(
                self.bitmap,
                if self.len_used > 0 {
                    self.len_used / BITMAP_WORD_BITS + 1
                } else {
                    0
                },
            )
        }
    }

    /// Get a mutable slice to the used part of the bitmap
    fn bitmap_mut(&mut self) -> &mut [usize] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.bitmap,
                if self.len_used > 0 {
                    self.len_used / BITMAP_WORD_BITS + 1
                } else {
                    0
                },
            )
        }
    }

    fn bitmap_at_index(&self, index: usize) -> usize {
        self.bitmap()[index / BITMAP_WORD_BITS]
    }

    fn bitmap_at_index_mut(&mut self, index: usize) -> &mut usize {
        &mut self.bitmap_mut()[index / BITMAP_WORD_BITS]
    }

    fn bitmap_clear_bit(&mut self, index: usize) {
        let (pos, bit) = bitmap_divmod(index);
        self.bitmap_mut()[pos] &= !(1usize << bit);
    }

    fn bitmap_set_bit(&mut self, index: usize) {
        let (pos, bit) = bitmap_divmod(index);
        self.bitmap_mut()[pos] |= 1usize << bit;
    }

    /// Get a slice to the used part of the bitmap
    fn entries(&self) -> &[Entry<T>] {
        unsafe { std::slice::from_raw_parts(self.memory, self.len_used) }
    }

    /// Get a mutable slice to the used part of the bitmap
    fn entries_mut(&mut self) -> &mut [Entry<T>] {
        unsafe { std::slice::from_raw_parts_mut(self.memory, self.len_used) }
    }

    /// Get a mutable slice to the used part of the bitmap
    fn entry_ptr(&mut self, index: usize) -> *mut Entry<T> {
        &mut self.entries_mut()[index]
    }

    // gets one entry from the unused capacity, returns None when the block is full
    fn extend(&mut self) -> Option<*mut Entry<T>> {
        if self.len_used < self.capacity {
            let pos = self.len_used;
            self.len_used += 1;

            if pos % BITMAP_WORD_BITS == 0 {
                // initialize the next bitmap word
                *self.bitmap_at_index_mut(pos) = usize::MAX & !1;
            } else {
                // otherwise just clear the 'free' bit
                self.bitmap_clear_bit(pos);
            }

            #[cfg(not(feature = "bitmap_scan"))]
            {
                self.in_use += 1;
            }
            // Safety: checked len_used < capacity
            let entry = unsafe { self.entries_mut().get_mut(pos).unwrap_unchecked() };
            (*entry).descr_rev_ptr = Entry::<T>::UNINITIALIZED_SENTINEL;
            Some(entry)
        } else {
            None
        }
    }

    pub fn alloc_entry(&mut self) -> Option<*mut Entry<T>> {
        let entry = self.freelist;
        if entry == Entry::<T>::END_OF_FREELIST {
            self.extend()
        } else {
            unsafe {
                debug_assert!(!(&*entry).is_allocated(), "Invalid freelist");
                let next = (*entry).maybe_data.fwd_ptr;
                self.freelist = if next == entry {
                    // single node in freelist
                    Entry::<T>::END_OF_FREELIST
                } else {
                    // unlink from list
                    let prev = (*entry).descr_rev_ptr;
                    (*next).descr_rev_ptr = (*entry).descr_rev_ptr;
                    (*prev).maybe_data.fwd_ptr = (*entry).maybe_data.fwd_ptr;
                    prev
                };
                (*entry).descr_rev_ptr = Entry::<T>::UNINITIALIZED_SENTINEL;
            }
            #[cfg(not(feature = "bitmap_scan"))]
            {
                self.in_use += 1;
            }
            Some(entry)
        }
    }

    fn entry_index(&self, entry: *mut Entry<T>) -> Option<usize> {
        if self
            .entries()
            .as_ptr_range()
            .contains(&(entry as *const Entry<T>))
        {
            Some(unsafe { entry.offset_from(self.memory) as usize })
        } else {
            None
        }
    }

    #[inline]
    fn freelist_is_empty(&self) -> bool {
        self.freelist == Entry::<T>::END_OF_FREELIST
    }

    /// returns true when any entry is still in use
    #[cfg(not(feature = "bitmap_scan"))]
    pub fn in_use(&self) -> bool {
        self.in_use > 0
    }

    #[cfg(feature = "bitmap_scan")]
    pub fn in_use(&self) -> bool {
        if self.len_used > 0 {
            !self.bitmap().iter().all(|&i| i == usize::MAX)
        } else {
            false
        }
    }

    pub fn free_entry(&mut self, entry: *mut Entry<T>) -> bool {
        if let Some(index) = self.entry_index(entry) {
            unsafe {
                debug_assert!(entry.as_ref().unwrap().is_allocated());
                if self.freelist_is_empty() {
                    // first node, cyclic pointing to itself
                    (*entry).descr_rev_ptr = entry;
                    (*entry).maybe_data.fwd_ptr = entry;
                } else {
                    // locality
                    // TODO: locality

                    // no locality, add to the tail of the freelist
                    let tail = self.freelist;
                    let head = (*tail).maybe_data.fwd_ptr;

                    (*entry).descr_rev_ptr = tail;
                    (*entry).maybe_data.fwd_ptr = head;

                    (*head).descr_rev_ptr = entry;
                    (*tail).maybe_data.fwd_ptr = entry;
                }
            }
            self.bitmap_set_bit(index);
            self.freelist = entry;
            #[cfg(not(feature = "bitmap_scan"))]
            {
                self.in_use -= 1;
            }
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
        unsafe { dealloc(self.memory as *mut u8, self.layout) };
    }

    #[cfg(not(debug_assertions))]
    fn drop(&mut self) {
        if !self.in_use() {
            // leak in release mode to enforce memory safety. This non intentional leaking is
            // not supported, use explicit leaking.
            unsafe { dealloc(self.memory as *mut u8, self.layout) };
        }
    }
}

const BITMAP_WORD_BITS: usize = usize::BITS as usize;

#[inline(always)]
fn bitmap_divmod(index: usize) -> (usize, usize) {
    (index / BITMAP_WORD_BITS, index % BITMAP_WORD_BITS)
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
        let mut block: Block<String> = Block::new_first(0);
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
        assert!(block.free_entry(entry));
    }
}
