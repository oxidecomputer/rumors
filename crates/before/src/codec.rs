//! Canonical binary storage and encoding.
//!
//! A party or version stores one preorder bit stream. A final marker bit and
//! zero padding make each byte string self-delimiting and canonical, so byte
//! equality is value equality. Decoding validates the complete stream before
//! adopting its bytes. Operations read borrowed bit views and emit the same
//! canonical form without reconstructing trees.

pub(crate) mod base;
mod bits;
mod buf;
mod build;
mod code;
mod cursor;
mod dsi;
mod gamma;
mod int;
pub(crate) mod scan;
mod stack;
mod tree;

#[cfg(test)]
mod tests;

#[cfg(feature = "limb-meter")]
pub(crate) use base::limb_meter;
pub use base::Base;
pub(crate) use bits::{canonical_eq, canonical_hash, padding_is_canonical, require_marker_padding};
#[cfg(test)]
pub(crate) use buf::bits_buf;
pub(crate) use buf::{built_view, extend_from_view};
// Tests and meters also construct complete mutable buffers directly.
pub use bits::{Bits, BitsView};
#[cfg(any(test, feature = "meter"))]
pub(crate) use buf::seal_padding;
pub use buf::BitsBuf;
pub(crate) use build::BitBuilder;
pub(crate) use code::Code;
pub(crate) use cursor::{BitCursor, SliceCursor};
pub(crate) use dsi::DsiCursor;
#[cfg(any(test, feature = "meter"))]
pub(crate) use gamma::encode_int;
pub(crate) use gamma::{code_int, code_int_small, decode_int, decode_int_from};
pub(crate) use int::Int;
// Borsh decoding reads word-sized windows from its own cursor.
#[cfg(feature = "borsh")]
pub(crate) use gamma::decode_int_window;
pub(crate) use stack::{BitStack, PopStack};
pub(crate) use tree::parse_id;
// Borsh validates a value within a larger input stream.
#[cfg(feature = "borsh")]
pub(crate) use tree::parse_id_core;
#[cfg(all(test, feature = "borsh"))]
pub(crate) use tree::parse_id_from;
