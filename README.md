# Description

Onsen provides hot Pools for objects.  In most cases allocation from such a Pool is faster and
offers better locality than the standard allocator. For small to medium sized objects the
performance improvement is around 20% or better. For large objects the gains become smaller as
caching effects even out. These improvements cover operating on objects because of locality,
not just faster allocation speeds. Onsen is made for to be used from single threads. This
means that in multithreaded applications it works best when each thread can keep its own pool
of objects. It is extremely fast when one can use alloc only or alloc mostly temporary Pools
which where memory becomes deallocated all at once when the Pool becomes destructed.


# Pools

Onsen pools allocate blocks with exponentially growing sizes. Allocations are served from
these blocks. Freed entries are kept in a double linked cyclic freelist. This freelist is kept
in weak ordered and the entry point always point close to where the last action happend to
keep the caches hot. Access to Pools is done by `PoolApi` traits. The visible pools wrap an
internal representation for interior mutable and/or shared access.


## Pool

This pool is only protected by a `RefCell`, it can not be shared. This makes it only usable
with `BasicBox` on a single thread. Its advantage is that it has the best performance and can
be used like an arena allocator where the `BasicBoxes` are just dropped in a leaky way and
memory becomes reclaimed when the `Pool` becomes dropped.


## RcPool

Adds a reference counter to the pool. Thus makes it available to the different `Box`
implementations. `RcPool` is still meant for singlethread use.


# Boxes

## UnsafeBox

Allocating from a pool returns `UnsafeBox` handles. These are lightweight abstractions to memory
allocations, they do not keep a relation to the pool and its lifetime. They are the underlying
facility to build the safe abstractions below. These boxes are unsafe because they have no
lifetime bound to their Pool but must not outlive it. This is asserted in debug builds but
because of cost left out in release builds.


## BasicBox

These are Boxes that may leak memory when not explicitly given back to the Pool. Still their
use is memory safe under all circumstances. They offer the most efficient way to allocate
memory, they have the same performance as `UnsafeBox`.


## Box, Rc and Sc

Onsen comes with its own `Box` and `Rc`/`Weak` implementations that wrap the underlying
`UnsafeBox` in a safe way by keeping a reference to the associated pool around. A `Sc`
reference counted box without weak reference support is available as well and provides an
advantage for small objects where the weak count would add some weight.

All these boxes need pools that are shareable, either because they are reference counted or
static.


# Features

Onsen provides a singlethreaded `Pool`, a singlethreaded reference counted `RcPool` and a
multithreaded `TPool`.  Additional features are gated with feature flags.

 * **parking_lot** use parking_lot for the `TPool` (instead `std::sync::Mutex`). This makes
   sense when parking lot is already in use. There is no significant performance benefit from
   this in onsen.
 * **stpool** Makes `STPool` available, a singlethreaded pool that uses a `ThreadCell` which
   is much faster than mutex protected pools. This pools can be moved cooperatively between
   threads with acquire/release semantics.
 * **tbox** Adds the API for `TBox`, `TRc`, `TSc` that use a global pool per type. The
   advantage is that the box does not need to store a reference to its pool which saves a bit
   memory and improves locality for small objects.
 * **st_tbox** use `STPool` for the tbox API, this enables **tbox** and **stpool** as well.

**st_tbox** is the default. This enables the most complete API with best performance.


## Performance Characteristics

 * Allocation from a Pool is much faster, 2-3 times faster as the standard allocator.

 * Freeing is about the same speed or slightly slower as the standard allocator.

 * Overall alloc/process/free operations are significantly faster than using the standard
   allocator. This is especially true when the processing part can benefit from the cache
   locatity where hot objects stay close together.

 * Onsen pools are optimized for cache locality and with that to some extend for
   singlethreaded use. It is best to have one pool per type per thread.

 * The `TPool` adds a mutex to be used in multithreaded cases but its performance is
   considerably less than the singlethreaded pools but in many cases still better than the
   std allocator. One will still benefit from locality though.

 * The `STPool` is singlethreaded but can be cooperatively passed between threads, its
   performance is on par with the other singlethreaded pools. This is especially important
   when one uses `TBox`, `TRc` or `TSc`.


# Benchmarking

Onsen uses criterion for benchmarking, since onsen is made for singlethreaded application its
best to be tested when locked on a single CPU core and lock the core to some frequency well
below the max to give more consistent results. At higher priority so it wont be disturbed as
much from other programs. On Linux you may do something like:

```shell,ignore
sudo renice -15 $$
sudo cpupower -c 1 frequency-set -f 2.8GHz
taskset 2 cargo bench
```

Will produce `target/criterion/report/index.html`.
