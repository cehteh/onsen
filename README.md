# Description

Onsen provides a hot Pool for objects.  Allocation from this Pool is faster and offers better
locality than the standard allocator in most cases.

A Box implementation for safe handling of Pool allocated objects is included.

# Details

The first block in a Pool is size `[Entry<T>; E]`, when a Pool runs out of storage it
allocates a new block from the system which is twice as big as the previous block.  E should
be be optimized for the intended use. That is blocks should become close or equal to multiples
of cache lines, pages, huge pages, whatever makes most sense. Memory allocation happens only
when necessary, creating a pool is a cheap operation.


# Slots and safety

Allocating from a Pool returns Slot handles. These are lightweight abstractions to memory
addresses, they do not keep a relation to the Pool they are allocated from. The rationale for
this design is to make them usable in a VM that uses NaN tagging.

Because of this Slots need to be handled with care and certain contracts need to be
enforced. The library provides some help to ensure correctness. The more expensive checks are
only run in debug mode. Few things can not be asserted and are guarded by unsafe functions.

  1. Slots must be given back to the Pool they originate from.
     * This is asserted only in debug mode because it is more expensive.
  2. Slots must not outlive the Pool they are allocated from.
     * When a Pool gets dropped while it still has live allocations it will panic in debug
       mode.
     * When a Pool with live allocations gets dropped in release mode it leaks its memory.
       This is unfortunate but ensures memory safety of the program.
     * There is `pool.leak()` which drops a pool while leaking its memory blocks. This can be
       used when one will never try to free memory obtained from that Pool.
  3. Slots must be freed only once.
     * This is always asserted. But the assertion may fail when the slot got allocated again.
  4. References obtained from Slots must not outlive the freeing of the Slot.
     * This is the main reason that makes the Slot freeing functions unsafe. There is no way
       for a Pool to know if references are still in use. One should provide a safe
       abstraction around referenced to enforce this.
  5. Slots can hold uninitialized data, then no references or Pins must be taken from them.
     * This is always asserted.
  6. Obtaining a `&mut T` from a Slot is mutually exclusive to obtaining a `Pin<&mut T>`.
     * This is always asserted.
  7. Any mutable reference obtained while initializing an uninitialized Slot must be dropped
     before calling `slot.assume_init()`. This would break the Pin guarantees.
     * This is part of the reason that `slot.get_uninit()` and `slot.assume_init()` are
       unsafe and must be enforced by the programmer.
  8. All the above applies to the NaN tagging facilities `slot.into_u64()`, `Slot::from_u64()`
     and `Slot::from_u64_masked()`.
  9. The NaN tagging facilities allow to duplicate slots which is not supported. Be careful
     when convert an u64 back to a Slot.
