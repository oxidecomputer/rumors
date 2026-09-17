//! Canonical binary storage and encoding.
//!
//! A party or version stores one preorder bit stream. A final marker bit and
//! zero padding make each byte string self-delimiting and canonical, so byte
//! equality is value equality. Decoding validates the complete stream before
//! adopting its bytes. Operations read borrowed bit views and emit the same
//! canonical form without reconstructing trees.

pub(crate) mod accumulator;
mod bits;
mod buf;
mod build;
mod cursor;
mod dsi;
pub(crate) mod gamma;
pub(crate) mod scan;
mod stack;
mod tree;

#[cfg(test)]
mod tests;

pub(crate) use bits::{canonical_eq, canonical_hash, padding_is_canonical, require_marker_padding};
pub use bits::{Bits, BitsView};
#[cfg(test)]
pub(crate) use buf::bits_buf;
#[cfg(any(test, feature = "meter"))]
pub(crate) use buf::seal_padding;
pub use buf::BitsBuf;
pub(crate) use buf::{built_view, extend_from_view};
pub(crate) use build::BitBuilder;
pub(crate) use cursor::{BitCursor, SliceCursor};
pub(crate) use dsi::DsiCursor;
pub(crate) use stack::{BitStack, PopStack};
pub(crate) use tree::parse_id;
#[cfg(feature = "borsh")]
pub(crate) use tree::parse_id_core;
#[cfg(all(test, feature = "borsh"))]
pub(crate) use tree::parse_id_from;
