//! The bit-level substrate both codings share, plus the id tree's codec.
//!
//! What lives here: the packed bit storage and its canonicality helpers
//! (`bits`), the sequential read cursors (`cursor`, `dsi`), the Elias-gamma
//! integer code (`gamma`), the arbitrary-precision [`Base`] the payloads
//! decode into (`base`), the append-truncate output builder (`build`), the
//! pop-able bit stack the deep walks hold word values in (`stack`), the
//! packed-stream write meter (`scan`), the text-notation parsers (`text`),
//! and the *id* tree's parse and strict validation (`tree`, with `literal`
//! and `display`). The event coding — the skyline — and its validation are
//! `version::skyline`'s, built on these primitives.
//!
//! At rest, a `Party`/`Version` holds its canonical packed preorder bit
//! stream (no trailing padding), so bit-equality is semantic equality.
//! `encode` pads that stream to a byte boundary; each `decode` parses and
//! *strictly validates* normal form (iteratively — nothing here recurses),
//! then stores the (canonical) consumed prefix.

pub(crate) mod base;
mod bits;
mod build;
mod cursor;
mod display;
mod dsi;
mod gamma;
mod literal;
pub(crate) mod scan;
mod stack;
pub(crate) mod text;
mod tree;

#[cfg(test)]
mod tests;

#[cfg(feature = "limb-meter")]
pub(crate) use base::limb_meter;
pub use base::Base;
pub(crate) use bits::{
    byte_view, bytes_as_bits, canonical_eq, canonical_hash, dead_bits_are_zero,
    require_zero_padding,
};
// Production streams canonicalize at the freeze seam (`Bits::freeze`);
// the standalone form serves the buffers that stay build-side, all of
// them meter/test instruments (the generators' packed outputs, the
// board's defect shapes, the snapshot corpus).
#[cfg(any(test, feature = "meter"))]
pub(crate) use bits::zero_dead_bits;
// The storage forms are `pub` (the enclosing module is not), so the
// meter surface can re-export them for the resource-envelope suite.
pub use bits::{Bits, BitsMut, BitsSlice};
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
pub(crate) use stack::PopStack;
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
