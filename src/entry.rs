use std::mem::ManuallyDrop;
use std::ptr::NonNull;

/// Special purpose version of `MaybeUninit` that may the linked freelist node when the Slot
/// is free.
#[repr(C)]
pub union MaybeData<T> {
    pub(crate) uninit: (),
    pub(crate) data: ManuallyDrop<T>,
    pub(crate) freelist_node: ManuallyDrop<FreelistNode<T>>,
}

impl<T> MaybeData<T> {
    /// Overwrites the potentially uninitialized `MaybeData` with new data without dropping the
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
/// pointers *always* pointing to some valid entry (pointing to itself when this is the only
/// node in the list).
pub(crate) struct FreelistNode<T> {
    pub prev: *mut Entry<T>,
    pub next: *mut Entry<T>,
}

/// Entries within a Pool.
#[repr(C, align(8))]
pub(crate) struct Entry<T> {
    pub(crate) maybe_data: MaybeData<T>,
}

// PLANNED: eventually (when stable) use https://github.com/rust-lang/rust/issues/44874
//          pub(crate) unsafe fn foo(self: *mut Self)

impl<T> Entry<T> {
    /// Removes an entry from the freelist and returns the entry that was next to self, if any.
    pub(crate) unsafe fn remove_free_node(&mut self) -> Option<NonNull<Entry<T>>> {
        let next = self.maybe_data.freelist_node.next;
        if next == self {
            // single node in list, nothing need to be done.
            None
        } else {
            // unlink from list
            let prev = self.maybe_data.freelist_node.prev;
            Entry::set_next(prev, next);
            Entry::set_prev(next, prev);

            Some(NonNull::new_unchecked(
                // decide which side to return as new freelist head
                if (next as usize + prev as usize) / 2 < self as *const Self as usize {
                    prev
                } else {
                    next
                },
            ))
        }
    }

    /// Initializes the freelist node to be pointing to itself.
    pub(crate) unsafe fn init_free_node(this: *mut Self) {
        Entry::set_next(this, this);
        Entry::set_prev(this, this);
    }

    /// Partial ordered insert if a freed node into the freelist. Order is determined by address of
    /// given nodes. The `freed_node` is either inserted before or after 'this'.
    pub(crate) unsafe fn insert_free_node(mut this: *mut Self, freed_node: *mut Self) {
        if freed_node < this {
            // insert freed_node before this

            // one more sorting step has no measurable impact on performance but may lead to
            // better cache locality
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

// Should be valid for C, but lets test this.
#[test]
fn entry_layout() {
    let e = Entry {
        maybe_data: MaybeData {
            data: ManuallyDrop::new(String::from("Hello")),
        },
    };
    assert_eq!(
        (&e) as *const Entry<String> as usize,
        (&e.maybe_data) as *const MaybeData<String> as usize
    );
}
