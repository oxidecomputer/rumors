//! The skyline coding of a [`Version`]: preorder topology bits plus
//! delta-coded absolute leaf heights.
//!
//! A [`Version`] is a step function over the unit id interval — a *skyline*
//! — and topology plus absolute leaf heights determine it completely.
//! This module codes exactly that, as two interleaved streams in one bit
//! string:
//!
//! - **Topology**: one preorder flag bit per node (`1` internal, `0` leaf),
//!   exactly as the packed form stores today. Internal nodes carry no
//!   numbers.
//! - **Leaf payloads**, in-stream at each leaf position: the first leaf's
//!   absolute height as `gamma(v1)`, every later leaf as
//!   `zigzag-gamma(vi − vi−1)` over consecutive leaves in preorder. The
//!   zigzag map is `k >= 0 -> 2k`, `k < 0 -> 2|k| − 1`; the mapped value is
//!   then gamma-coded by the same [`codec::encode_int`] machinery as every
//!   stored integer, so the code shape (`2k + 1` bits) and the decoder's
//!   window fast path carry over unchanged.
//!
//! Nothing routes wire bytes through this module: [`Version::encode`] and
//! [`Version::decode`] carry the packed preorder form, which doubles as
//! this codec's behavioral oracle — [`encode`] transcodes a stored
//! [`Version`] into skyline bits, [`decode`] strictly validates skyline
//! bits and transcodes them back, and the test suite pins the two codings
//! against each other (see Testing below). The module is test- and
//! meter-visible only, via [`crate::meter::skyline`].
//!
//! # Canonical form
//!
//! A skyline stream is canonical iff:
//!
//! - **The topology is minimal**: no internal node whose two children are
//!   both leaves with a zero right delta — equal sibling leaves are exactly
//!   what collapse removes. A zero delta between *non-sibling* consecutive
//!   leaves is a real, canonical shape: two plateaus of equal height
//!   separated by a subtree boundary.
//! - **Every leaf height is a natural**: the payload stream is signed, and
//!   nothing else stops a delta from driving the running height negative.
//! - **The stream is exact**: one complete tree, no truncation, no
//!   trailing bits.
//!
//! There is no code-level canonicality obligation beyond these: gamma is a
//! prefix code with exactly one spelling per natural, and the zigzag map is
//! a bijection with no negative-zero form (odd codes decode to magnitude
//! `>= 1`), so a non-minimal gamma code or a non-canonical zigzag spelling
//! cannot be written at all. The tests pin both bijections exhaustively at
//! small scope.
//!
//! Canonical skyline streams and canonical packed [`Version`]s are in
//! bijection: both are unique representations of the step function
//! (minimal topology makes the tree unique; heights are
//! function-determined; the packed form's min-lifted bases are determined
//! by topology and heights). Byte-equality therefore remains semantic
//! equality on this coding, and the transcoding round-trip is exact.
//!
//! # Validation cost
//!
//! [`validate`] runs one forward pass holding, per open ancestor, two
//! bits — "is my left child complete" and "was that child a leaf" — on a
//! packed bit stack, plus one [`Accum`](codec::accum::Accum) carrying the
//! running leaf height for the nonnegativity check. The bit stack replaces
//! the packed form's per-ancestor parse frames (two [`codec::Base`] values,
//! ~56 bytes per level) with ~2 bits per level; the resource-envelope suite
//! (`tests/meter.rs`) pins both that transient and the validator's limb
//! behavior.
//!
//! The accumulator choice is load-bearing, not an optimization: on the
//! boundary comb (`meter::cliff_comb`) the payload stream is 3-bit `±1`
//! codes sitting exactly on a `2^k` carry boundary, so a plain big-integer
//! running height pays a full `k`-bit carry per 3-bit delta — `Θ(W²)` limb
//! work in skyline wire bits `W` (`meter::tier2`'s plain-sweep pin measures
//! it). The balanced signed-digit [`Accum`](codec::accum::Accum) applies a
//! small delta and answers the sign check in amortized O(1) digit touches
//! on every input sequence, so validation stays linear per wire bit; the
//! envelope suite pins the per-delta touch cost flat across size doublings
//! on the comb.
//!
//! # Cost of the transcoders
//!
//! Both transcoding directions materialize absolute heights and are priced
//! by the *packed* form, never by skyline bits: [`encode`] walks the stored
//! packed stream accumulating root-to-leaf path sums (bounded by the packed
//! form it reads), and [`decode`] rebuilds per-node subtree floors to emit
//! min-lifted bases (bounded by the packed form it writes — on the comb
//! that output is `Θ(nk)` bits behind `Θ(n + k)` skyline bits, so no
//! transcode can be skyline-linear). Only [`validate`] carries the
//! wire-bit-linear guarantee, and it is the piece whose envelope is pinned.
//!
//! # Testing
//!
//! - **Length agreement**: the encoder's output length equals
//!   [`crate::meter::tier2::tier2_size`] bit for bit, proptested over every
//!   adversarial generator family, arbitrary trees, and organic histories.
//!   The sizer and the encoder implement the coding independently (separate
//!   walks, separate zigzag maps), so agreement cross-checks both.
//! - **Round-trip**: `decode(encode(v))` reproduces `v` exactly (packed
//!   byte equality, which is stronger than per-leaf height equality).
//! - **Byte uniqueness**: op-path-independent equality — value-equal
//!   versions built along different operation paths produce identical
//!   skyline bytes; exhaustive small-scope injectivity over all normal
//!   forms to a bounded depth.
//! - **Strict rejection**: a deterministic corpus per reject genre (zero
//!   right-sibling delta, negative running height, truncation at every cut
//!   point, trailing bits), each with its exact error; plus a
//!   mutation proptest — flipping any single bit of a valid encoding
//!   either rejects or round-trips to a *different* version whose canonical
//!   encoding is the mutated stream itself.
//! - **Resource envelopes**: `tests/meter.rs` pins validate/decode heap,
//!   segment, limb, and accumulator-touch envelopes on the adversarial
//!   families.

use crate::codec::{self, Base, BitsSlice};
use crate::error::Decode;
use crate::Version;

mod decode;
mod encode;
mod validate;

#[cfg(test)]
mod tests;

pub(crate) use decode::decode_bits;
pub(crate) use encode::encode_bits;
pub(crate) use validate::validate_bits;

/// A skyline bit stream packed into bytes, with its exact live bit length.
///
/// `bytes` is the stream zero-padded to a byte boundary; `bits` is the live
/// length before that padding. The pair is what the byte-level entry points
/// exchange: the stream is not self-delimiting at the byte level, so the
/// live length travels with the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// The skyline stream, final partial byte zero-padded.
    pub bytes: Vec<u8>,
    /// The exact number of live bits in `bytes` before the zero pad.
    pub bits: usize,
}

/// Transcode a stored [`Version`] into its canonical skyline stream.
///
/// One preorder pass over the packed form, accumulating absolute leaf
/// heights as root-to-leaf path sums; transient state is the inherited-sum
/// stack, priced by the packed input (see the module doc's cost section).
pub fn encode(version: &Version) -> Encoded {
    let mut bits = encode_bits(version);
    let live = bits.len();
    codec::zero_dead_bits(&mut bits);
    Encoded {
        bytes: bits.into_vec(),
        bits: live,
    }
}

/// Strictly validate a skyline stream without materializing any height.
///
/// Enforces the module doc's canonical form in one forward pass: minimal
/// topology and stream exactness on ~2 bits of stack per open ancestor,
/// and leaf-height nonnegativity on the cliff-immune accumulator. Every
/// error is [`Decode::Truncated`] (the stream ended mid-tree or
/// mid-integer), [`Decode::TrailingBits`] (live bits remain after the
/// tree), or [`Decode::NotCanonical`] (collapsible sibling leaves, or a
/// delta driving the running height negative).
///
/// # Panics
///
/// Panics if `bits` exceeds the live bits in `bytes`.
pub fn validate(bytes: &[u8], bits: usize) -> Result<(), Decode> {
    validate_bits(live_bits(bytes, bits))
}

/// Strictly validate a skyline stream and transcode it back to a [`Version`].
///
/// [`validate`]'s pass runs first and gates the transcode, so acceptance
/// is identical to [`validate`]'s; the transcode then materializes heights
/// and subtree floors, priced by the packed form it emits (see the module
/// doc's cost section).
///
/// # Panics
///
/// Panics if `bits` exceeds the live bits in `bytes`.
pub fn decode(bytes: &[u8], bits: usize) -> Result<Version, Decode> {
    decode_bits(live_bits(bytes, bits))
}

/// Borrow the live prefix of a padded byte stream as bits.
///
/// # Panics
///
/// Panics if `bits` exceeds the live bits in `bytes`.
fn live_bits(bytes: &[u8], bits: usize) -> &BitsSlice {
    let all = codec::bytes_as_bits(bytes);
    assert!(
        bits <= all.len(),
        "skyline stream length overruns its bytes: {bits} live bits declared over {} available",
        all.len(),
    );
    &all[..bits]
}

/// Map the signed difference `cur − prev` to its zigzag magnitude:
/// `k >= 0 -> 2k`, `k < 0 -> 2|k| − 1`.
fn zigzag(prev: &Base, cur: &Base) -> Base {
    if cur >= prev {
        (cur.clone() - prev) << 1u32
    } else {
        ((prev.clone() - cur) << 1u32) - &Base::from(1u8)
    }
}

/// Split a zigzag magnitude into its delta's sign and absolute value:
/// even `m -> +m/2`, odd `m -> −(m + 1)/2`.
///
/// The inverse of [`zigzag`]: total, and never yields a negative zero (an
/// odd code's magnitude is at least 1).
fn unzigzag(code: Base) -> (bool, Base) {
    if code.bit(0) {
        (true, (code + 1u32) >> 1u32)
    } else {
        (false, code >> 1u32)
    }
}
