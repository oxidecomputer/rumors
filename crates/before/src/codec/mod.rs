//! Bit I/O: the Elias-gamma integer code, the preorder id/event encodings, and
//! the iterative `decode` with normal-form validation.
//!
//! At rest, a `Party`/`Version` holds its canonical packed preorder bit stream
//! (no trailing padding), so bit-equality is semantic equality. `encode` pads
//! that stream to a byte boundary; `decode` parses and *strictly validates*
//! normal form, then stores the (canonical) consumed prefix.

mod base;
mod bits;
mod display;
mod gamma;
mod literal;
mod text;
mod tree;

#[cfg(test)]
mod tests;

pub use base::Base;
pub(crate) use bits::{bytes_as_bits, pack_to_bytes, require_zero_padding, Bits, BitsSlice};
pub(crate) use display::{write_ev, write_id};
pub(crate) use gamma::{decode_int, encode_int, encoded_int_len, skip_int};
pub(crate) use literal::{ev_leaf, ev_node, id_is_empty, id_leaf, id_node};
pub(crate) use text::{parse_clock_str, parse_ev_str, parse_id_str};
pub(crate) use tree::{parse_ev, parse_id, validate_ev, validate_id};
