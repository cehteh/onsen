#![allow(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]
// uncomment for discovering new lints:
//#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![warn(clippy::cargo_common_metadata)]
#![warn(clippy::doc_markdown)]
#![warn(clippy::missing_panics_doc)]
#![warn(clippy::must_use_candidate)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(clippy::perf)]
#![warn(clippy::style)]

/// Maximum number of blocks in a Pool
pub(crate) const NUM_BLOCKS: usize = 44;

#[cfg(feature = "tbox")]
#[doc(hidden)]
pub use assoc_static::*;

mod block;
use block::*;

//mod boxed;
mod basicbox;
mod entry;
mod macros;
mod pool;
// mod rcpool;
mod poolapi;
mod poolinner;
mod unsafebox;
// mod refcounted;
// mod stpool;
// mod strongcounted;
// mod tboxed;
// mod tpool;
// mod trefcounted;
// mod tstrongcounted;

//pub use boxed::*;
pub use basicbox::*;
pub use entry::*;
pub use pool::*;
// pub use rcpool::*;
pub use poolapi::*;
pub use poolinner::*;
pub use unsafebox::*;
// pub use refcounted::*;
// pub use strongcounted::*;
// pub use tpool::*;

// #[cfg(feature = "stpool")]
// pub use stpool::*;
//
// #[cfg(feature = "tbox")]
// pub use {tboxed::*, trefcounted::*, tstrongcounted::*};

/// The error returned when a `STPool` can not be acquired or released.
#[derive(Debug, Copy, Clone)]
pub struct PoolOwnershipError;
