# Description

Onsen provides a Pool for objects.  Allocation from this Pool is faster and offers better
locality than the standard allocator in most cases.


# Details

The first block in a Pool is size `[Entry<T>; E]`, when a Pool runs out of storage it allocates
a new block from the system which is twice as big as the previous block.  E should be a power
of two and be optimized for the intended use. That is blocks should become close or equal to
multiples of cache lines, pages, huge pages, whatever makes most sense. Memory allocation
happens only when necessary, creating a pool is a cheap operation.

