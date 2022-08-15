use std::mem::ManuallyDrop;
use std::ptr::NonNull;

/// Special purpose version of MaybeUninit that may hold a forward pointer of the linked
/// freelist when the Slot is free.
#[repr(C)]
pub union MaybeData<T> {
    pub(crate) uninit: (),
    pub(crate) data: ManuallyDrop<T>,
    pub(crate) freelist_node: ManuallyDrop<FreelistNode<T>>,
}

impl<T> MaybeData<T> {
    /// Overwrites the potentially uninitialized MaybeData with new data without dropping the
    /// old value. Returns a reference to the new data. This is safe because rust does not
    /// give the guarantees that destructors are always called. Still there is a danger
    /// because of this that some resources may be leaked.
    #[inline(always)]
    pub fn write(&mut self, val: T) -> &mut T {
        self.data = ManuallyDrop::new(val);
        // SAFETY: We just initialized this value.
        unsafe { &mut self.data }
    }
}

/// The type of the freelist node. When used (node is free) then this is a cyclic list with
/// pointers *always* pointing to some valid entry (itself when this is the only node in the
/// list).
pub(crate) struct FreelistNode<T> {
    pub prev: *mut Entry<T>,
    pub next: *mut Entry<T>,
}

/// For improved safety entries are tagged with their current state.
pub(crate) enum Descriptor {
    Free,
    Uninitialized,
    Initialized,
    Referenced,
    Pinned,
}
use Descriptor::*;

/// Entries within a Pool.
#[repr(C, align(8))]
pub(crate) struct Entry<T> {
    pub(crate) maybe_data: MaybeData<T>,
    pub(crate) descriptor: Descriptor,
}

// PLANNED: eventually (when stable) use https://github.com/rust-lang/rust/issues/44874
//          pub(crate) unsafe fn foo(self: *mut Self)

impl<T> Entry<T> {
    /// Returns true when the entry is free and false when it is allocated.
    #[inline(always)]
    pub fn is_free(&self) -> bool {
        unsafe { Self::ptr_is_free(self) }
    }

    #[inline(always)]
    pub(crate) unsafe fn ptr_is_free(this: *const Self) -> bool {
        matches!((*this).descriptor, Free)
    }

    /// Returns true when the entry is uninitialized,
    /// false on anything else.
    #[inline(always)]
    pub fn is_uninitialized(&self) -> bool {
        unsafe { Self::ptr_is_uninitialized(self) }
    }

    #[inline(always)]
    pub(crate) unsafe fn ptr_is_uninitialized(this: *const Self) -> bool {
        matches!((*this).descriptor, Uninitialized)
    }

    /// Returns true when the entry is initialized, got referenced or pinned.
    /// Returns false when the entry is uninitialized.
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        unsafe { Self::ptr_is_initialized(self) }
    }

    #[inline(always)]
    pub(crate) unsafe fn ptr_is_initialized(this: *const Self) -> bool {
        matches!((*this).descriptor, Initialized | Referenced | Pinned)
    }

    /// Returns true when the entry at 'entry' ever got referenced or pinned.
    #[inline(always)]
    pub fn is_referenced(&self) -> bool {
        unsafe { Self::ptr_is_referenced(self) }
    }

    #[inline(always)]
    pub(crate) unsafe fn ptr_is_referenced(this: *const Self) -> bool {
        matches!((*this).descriptor, Referenced | Pinned)
    }

    /// Returns true when the entry at 'entry' ever got pinned.
    #[inline(always)]
    pub fn is_pinned(&self) -> bool {
        unsafe { Self::ptr_is_pinned(self) }
    }

    #[inline(always)]
    pub(crate) unsafe fn ptr_is_pinned(this: *const Self) -> bool {
        matches!((*this).descriptor, Pinned)
    }

    /// Removes an entry from the freelist and returns the entry that was before self, if any.
    pub(crate) unsafe fn remove_free_node(&mut self) -> Option<NonNull<Entry<T>>> {
        debug_assert!(self.is_free(), "Invalid allocation");

        let next = self.maybe_data.freelist_node.next;
        if next == self {
            // single node in list, nothing need to be done.
            None
        } else {
            // unlink from list
            let prev = self.maybe_data.freelist_node.prev;
            Entry::set_next(prev, next);
            Entry::set_prev(next, prev);
            Some(NonNull::new_unchecked(prev))
        }
    }

    /// Initializes the freelist node to be pointing to itself.
    pub(crate) unsafe fn init_free_node(this: *mut Self) {
        debug_assert!(!Entry::ptr_is_free(this), "Double free");

        Entry::set_next(this, this);
        Entry::set_prev(this, this);
    }

    /// Ordered insert if a freed node into the freelist. Order is determined by address of
    /// given nodes. The 'freed_node' is either inserted before or after 'this'.
    pub(crate) unsafe fn insert_free_node(mut this: *mut Self, freed_node: *mut Self) {
        debug_assert!(!Entry::ptr_is_free(freed_node), "Double free");
        debug_assert!(Entry::ptr_is_free(this), "Corrupted freelist");

        if freed_node < this {
            // insert freed_node before this

            // one more sorting step has no impact on performance but may lead to better cache
            // locality
            if freed_node < Entry::prev(this) {
                this = Entry::prev(this);
            }

            Entry::set_next(freed_node, this);
            Entry::set_prev(freed_node, Entry::prev(this));
            Entry::set_next(Entry::prev(this), freed_node);
            Entry::set_prev(this, freed_node);
        } else {
            // insert freed_node after this

            if freed_node > Entry::next(this) {
                this = Entry::next(this);
            }

            Entry::set_prev(freed_node, this);
            Entry::set_next(freed_node, Entry::next(this));
            Entry::set_prev(Entry::next(this), freed_node);
            Entry::set_next(this, freed_node);
        }
    }

    #[inline(always)]
    unsafe fn next(this: *mut Self) -> *mut Self {
        (*(*this).maybe_data.freelist_node).next
    }

    #[inline(always)]
    unsafe fn prev(this: *mut Self) -> *mut Self {
        (*(*this).maybe_data.freelist_node).prev
    }

    #[inline(always)]
    unsafe fn set_next(this: *mut Self, that: *mut Self) {
        (*(*this).maybe_data.freelist_node).next = that;
    }

    #[inline(always)]
    unsafe fn set_prev(this: *mut Self, that: *mut Self) {
        (*(*this).maybe_data.freelist_node).prev = that;
    }
}
