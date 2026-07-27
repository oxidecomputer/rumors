//! Bit I/O: the Elias-gamma integer code, the preorder id/event encodings, and
//! the recursive `decode` with normal-form validation.
//!
//! At rest, a `Party`/`Version` holds its canonical packed preorder bit stream
//! (no trailing padding), so bit-equality is semantic equality. `encode` pads
//! that stream to a byte boundary; `decode` parses and *strictly validates*
//! normal form, then stores the (canonical) consumed prefix.

// Unconditional: the rank fold sums through `Accum`, so the accumulator is
// load-bearing in every build. `before::meter` additionally re-exports it so
// the resource-envelope suite can pin its digit-touch cost.
pub mod accum;
pub(crate) mod base;
mod bits;
mod build;
mod cursor;
mod display;
mod dsi;
mod gamma;
mod literal;
pub(crate) mod scan;
pub(crate) mod text;
mod tree;

#[cfg(test)]
mod tests;

#[cfg(feature = "limb-meter")]
pub(crate) use base::limb_meter;
pub use base::Base;
pub(crate) use bits::{
    bytes_as_bits, canonical_eq, canonical_hash, dead_bits_are_zero, require_zero_padding,
    zero_dead_bits,
};
// The storage aliases are `pub` (the enclosing module is not), so the
// meter surface can re-export them for the resource-envelope suite.
pub use bits::{Bits, BitsSlice};
pub(crate) use build::PackedBuilder;
pub(crate) use cursor::{BitCursor, SliceCursor};
pub(crate) use display::write_id;
pub(crate) use dsi::DsiCursor;
pub(crate) use gamma::{decode_int, decode_int_from, encode_int};
// The word fast path of the gamma decoder, exported for the wire-side
// `ReaderCursor` (`borsh_impls`), the one consumer outside this module; the
// cfg keeps the re-export from dangling when `borsh` is off.
#[cfg(feature = "borsh")]
pub(crate) use gamma::decode_int_window;
pub(crate) use literal::{id_is_empty, id_leaf, id_node};
pub(crate) use text::{parse_clock_str, parse_id_str};
pub(crate) use tree::{parse_id, validate_id};
// The mid-stream parser entry is consumed only by the borsh wire format
// (everything else parses whole streams); gating the re-export keeps default
// builds warning-free for downstream consumers.
#[cfg(feature = "borsh")]
pub(crate) use tree::parse_id_from;
// Test-only: the spill tests (here and in `borsh_impls`) size their trees
// relative to the inline capacity so they keep testing the heap spill if the
// capacity is ever retuned.
#[cfg(test)]
pub(crate) use tree::PARSE_STACK_INLINE;
