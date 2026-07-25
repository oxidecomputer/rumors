//! Bit I/O: the Elias-gamma integer code, the preorder id/event encodings, and
//! the recursive `decode` with normal-form validation.
//!
//! At rest, a `Party`/`Version` holds its canonical packed preorder bit stream
//! (no trailing padding), so bit-equality is semantic equality. `encode` pads
//! that stream to a byte boundary; `decode` parses and *strictly validates*
//! normal form, then stores the (canonical) consumed prefix.

mod base;
mod bits;
mod cursor;
mod display;
mod gamma;
mod literal;
mod text;
mod tree;

#[cfg(test)]
mod tests;

pub use base::Base;
pub(crate) use bits::{
    bytes_as_bits, pack_to_writer, require_zero_padding, zero_dead_bits, Bits, BitsSlice,
};
pub(crate) use cursor::{BitCursor, SliceCursor};
pub(crate) use display::{write_ev, write_id};
pub(crate) use gamma::{decode_int, decode_int_from, encode_int, skip_int};
// The word fast path of the gamma decoder, exported for the wire-side
// `ReaderCursor` (`borsh_impls`), the one consumer outside this module; the
// cfg keeps the re-export from dangling when `borsh` is off.
#[cfg(feature = "borsh")]
pub(crate) use gamma::decode_int_window;
pub(crate) use literal::{ev_leaf, ev_node, id_is_empty, id_leaf, id_node};
pub(crate) use text::{parse_clock_str, parse_ev_str, parse_id_str};
pub(crate) use tree::{parse_ev, parse_ev_from, parse_id, parse_id_from, validate_ev, validate_id};
// Test-only: the spill tests (here and in `borsh_impls`) size their trees
// relative to the inline capacity so they keep testing the heap spill if the
// capacity is ever retuned.
#[cfg(test)]
pub(crate) use tree::PARSE_STACK_INLINE;
