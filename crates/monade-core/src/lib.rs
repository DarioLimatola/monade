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
//! - [`address`]: the octal address (a root and a path of base-8 digits), down to one bit.
//! - [`space`]: the generated space: vectors computed from an id and the seed (BLAKE3).
//! - [`tally`]: counters per bit, the first learning: the bit is the reading, the counter the memory.
//!
//! No floating point, no `unsafe`; one dependency, `blake3`, for the generated space. Errors are values.

#![forbid(unsafe_code)]

pub mod address;
pub mod origin;
pub mod record;
pub mod size;
pub mod space;
pub mod specs;
pub mod tally;
pub mod word;

#[cfg(any(test, feature = "oracle"))]
pub mod oracle;
