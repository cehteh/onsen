//use crate::*;
use erasable::*;

/// Any `Pool` stores objects implementing `PoolEntry`.
#[doc(hidden)]
pub trait PoolEntry: Sized {
    /// The actual user type stored in the pool.
    type Value: Sized;
}

/// Entry types of owned box types.
#[doc(hidden)]
pub trait OwnedPoolEntry: PoolEntry {
    /// Creates a new entry
    fn new(value: Self::Value) -> Self;
}

/// Entry types of shared box types. Needs to be constructed with a backreference to the pool
/// itself as type erased pointer.
#[doc(hidden)]
pub trait SharedPoolEntry: PoolEntry {
    /// Creates a new entry
    fn new(value: Self::Value, ptr: ErasedPtr) -> Self;
}

/// A thin `PoolEntry` which does not store a reference back to its pool.
#[doc(hidden)]
#[repr(transparent)]
pub struct ThinPoolEntry<T>(pub(crate) T);

impl<T> PoolEntry for ThinPoolEntry<T> {
    type Value = T;
}

impl<T> OwnedPoolEntry for ThinPoolEntry<T> {
    #[inline(always)]
    fn new(value: Self::Value) -> Self {
        ThinPoolEntry(value)
    }
}

/// A `PoolEntry` that stores a reference to its pool.
//FIXME: DOC when Inner is needed
pub struct FatPoolEntry<T> {
    pub(crate) slot: T,
    // Until someone knows better we need type-erasure here because we constructing a
    // recursive type.
    pub(crate) pool: ErasedPtr,
}

impl<T> PoolEntry for FatPoolEntry<T> {
    type Value = T;
}

impl<T> SharedPoolEntry for FatPoolEntry<T> {
    #[inline(always)]
    fn new(value: Self::Value, ptr: ErasedPtr) -> Self {
        FatPoolEntry {
            slot: value,
            pool: ptr,
        }
    }
}

// PLANNED: RcPoolEntry ScPoolEntry  ArcPoolEntry AscPoolEntry
