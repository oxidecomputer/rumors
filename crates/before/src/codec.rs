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
//! stream, marker-padded to a byte boundary (`bits`' docs give the coding),
//! so the stored bytes are injective on streams — byte equality is semantic
//! equality — and are exactly what `encode` emits. Each `decode` parses and
//! *strictly validates* normal form (iteratively — nothing here recurses),
//! then adopts the (canonical) input bytes.

pub(crate) mod base;
mod bits;
mod buf;
mod build;
mod code;
mod cursor;
mod display;
mod dsi;
mod gamma;
mod int;
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
pub(crate) use bits::{canonical_eq, canonical_hash, padding_is_canonical, require_marker_padding};
#[cfg(test)]
pub(crate) use buf::bits_buf;
pub(crate) use buf::{built_view, extend_from_view};
// Production streams seal at the freeze seam (`Bits::freeze`); the
// standalone form serves the buffers that stay build-side, all of them
// meter/test instruments producing decodable bytes (the generators'
// packed outputs, the board's defect shapes, the snapshot corpus).
#[cfg(any(test, feature = "meter"))]
pub(crate) use buf::seal_padding;
// The storage forms are `pub` (the enclosing module is not), so the
// meter surface can re-export them for the resource-envelope suite.
pub use bits::{Bits, BitsView};
pub use buf::BitsBuf;
pub(crate) use build::PackedBuilder;
pub(crate) use code::Code;
pub(crate) use cursor::{BitCursor, SliceCursor};
pub(crate) use display::write_id;
pub(crate) use dsi::DsiCursor;
pub(crate) use gamma::{code_int, code_int_small, decode_int, decode_int_from, encode_int};
pub(crate) use int::Int;
// The word fast path of the gamma decoder, exported for the wire-side
// `ReaderCursor` (`borsh_impls`), the one consumer outside this module; the
// cfg keeps the re-export from dangling when `borsh` is off.
#[cfg(feature = "borsh")]
pub(crate) use gamma::decode_int_window;
pub(crate) use literal::{id_is_empty, id_leaf, id_node};
pub(crate) use stack::{BitStack, PopStack};
pub(crate) use text::{parse_clock_str, parse_id_str};
pub(crate) use tree::{parse_id, validate_id};
// The mid-stream parser entry is consumed only by the borsh wire format
// (everything else parses whole streams); gating the re-export keeps default
// builds warning-free for downstream consumers.
#[cfg(feature = "borsh")]
pub(crate) use tree::parse_id_core;
// The generic-position parse serves the wire-side (borsh) test suite; the
// grammar body above is what production readers drive.
#[cfg(all(test, feature = "borsh"))]
pub(crate) use tree::parse_id_from;
