# Description

Onsen provides hot Pools for objects.  In most cases allocation from such a Pool is faster and
offers better locality than the standard allocator. For small to medium sized objects the
performance improvement is around 20% or better. For large objects the gains become smaller as
caching effects even out. These improvements cover operating on objects because of locality,
not just faster allocation speeds.


# Details

Onsen pools allocate blocks with exponentially growing sizes. Allocations are served from
these blocks. Freed entries are kept in a double linked cyclic freelist. This freelist is kept
in weakly ordered and the entry point always point close to where the last action happend to
keep the caches hot.


# Box, Rc and Sc

Onsen comes with its own `Box` and `Rc`/`Weak` implementations that wrap the underlying
`RcPool` in a safe way. A `Sc` reference counted box without weak reference support is
available as well and provides an advantage for small objects where the weak count would add
some weight.

For each of these a variant that uses static global pools is avaialble as well.

# Slots

Allocating from a pool returns `Slot` handles. These are lightweight abstractions to memory
addresses, they do not keep a relation to the pool they are allocated from. The rationale for
this design is to make them usable in a VM that uses NaN tagging.


## Slot Policies

Slots are guarded by typestate policies which prevent some wrong use at compile time.


## Slots and Safety

Because of this slots need to be handled with care and certain contracts need to be
enforced. The library provides some help to ensure correctness. Few things can not be asserted
and are guarded by unsafe functions. Higher level API's (Such as `Box`, `Rc` and `Sc` above) can
easily enforce these in a safe way.

  1. Slots must be given back to the pool they originate from.
  2. Slots must not outlive the pool they are allocated from.
     * When a Pool gets dropped while it still has live allocations it will panic in debug
       mode.
     * When a pool with live allocations gets dropped in release mode it leaks its memory.
       This is unfortunate but ensures memory safety of the program.
     * There is `pool.leak()` which drops a pool while leaking its memory blocks. This can be
       used when one will never try to free memory obtained from that Pool.
     * This applies to u64 NaN tags as well.
  3. Slots must be freed only once.
     * This is always asserted. But the assertion may fail when the slot got allocated again.
     * Slots are not 'Copy' thus one can not safely free a slot twice but there is an explicit
       'copy()' function used by the reference count implementations and the NaN tagging
       facilities can copy an 'u64' and try to attempt to free this multiple times. These are
       'unsafe' functions becasue of that.
  4. References obtained from slots must not outlive the freeing of the `Slot`.
     * This is the main reason that makes the `Slot` freeing functions unsafe. There is no way
       for a pool to know if references are still in use. One should provide or use a safe
       abstraction around references to enforce this.


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

 * Onsen pools are optimized for cache locality and with that to some extend for
   singlethreaded use. It is best to have one pool per type per thread.

 * The `TPool` adds a mutex to be used in multithreaded cases but its performance is
   significantly less than the singlethreaded pools but in many cases still better than the
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
sudo cpupower -c 1 frequenc-set -f 2.8GHz
taskset 2 cargo bench
```

Will produce `target/criterion/report/index.html`.
