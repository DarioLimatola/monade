//! The core of the Monade.
//!
//! One rule at every scale: a Monade of any size is `8^k` words, and the same
//! word operations (XOR, AND, popcount, Hamming, majority) work on all of them.
//!
//! Modules, in the order they were written:
//!
//! - [`word`]: the 64-bit word, the 64-byte block, the bit operations.
//! - [`record`]: the readable text format shared by point 0 and the specifications.
//! - [`origin`]: point 0, an append-only log of foundations (empty for now).
//! - [`specs`]: the specifications, mutable with history, one locked entry (the seed).
//! - [`size`]: the sizes of the Monade (64 bits to 2 MB in powers of 8) and arenas.
//!
//! No floating point, no `unsafe`, no external dependencies. Errors are values.

#![forbid(unsafe_code)]

pub mod origin;
pub mod record;
pub mod size;
pub mod specs;
pub mod word;

#[cfg(any(test, feature = "oracle"))]
pub mod oracle;
