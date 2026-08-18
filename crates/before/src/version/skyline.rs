//! The skyline coding of a [`Version`]: preorder topology bits plus the
//! absolute leaf heights, delta-coded.
//!
//! A [`Version`] is a step function over the unit id interval — a *skyline* —
//! and each maximal constant run of it is a plateau: one leaf of the tree,
//! spanning a dyadic interval of width `2^-depth` (the `overlay` machinery
//! module's cursor vocabulary mints the term). Topology plus absolute leaf
//! heights determine the function completely. This module codes exactly that,
//! as two interleaved streams in one bit string:
//!
//! - **Topology**: one preorder flag bit per node (`0` internal, `1` leaf).
//!   Internal nodes carry no numbers, and a root-to-leaf descent is a
//!   unary run — zero or more `0`s ended by the leaf's `1` — so a reader
//!   takes a whole descent in one word-parallel unary read (the `codec`
//!   module's cursors share that vocabulary).
//! - **Leaf payloads**, in-stream at each leaf position: the first leaf's
//!   absolute height as `gamma(v1)` (this crate's gamma codes every
//!   natural, zero included), every later leaf as
//!   `zigzag-gamma(vi − vi−1)` over consecutive leaves in preorder. The
//!   zigzag map is `d >= 0 -> 2d`, `d < 0 -> 2|d| − 1`; the mapped value is
//!   then gamma-coded by the same machinery as every stored integer
//!   (`codec::encode_int`), so the code shape (`2k + 1` bits for a
//!   `k`-bit-position mantissa) and the decoder's window fast path carry
//!   over unchanged.
//!
//! This coding is the stored and wire form of a [`Version`]:
//! [`Version::encode`] and [`Version::decode`] carry these streams, and every
//! operation runs on them directly. The submodules:
//!
//! - [`sweep`] decides comparisons on skyline streams — the merge form
//!   the coding exists to enable.
//! - `masked` decides the same comparisons over *projected* streams
//!   (event × id overlays) without materializing any projection.
//! - `place` places one stream against a range's or an
//!   interval's bound streams in a single fused merge, generic over the
//!   verdict (`causally`'s placement kernel).
//! - [`emit`] runs the same merge as join and meet, re-delta-coding
//!   pointwise max/min into a canonical stream through the collapsing
//!   output builder (the private `build` submodule, which the tick
//!   splice drives too).
//! - [`query`] answers the linear functionals (rank, distance, lag,
//!   min_ticks) and projection from the same leaf sweeps.
//! - `fill` registers an event — the fused tick: one fill
//!   walk deciding in-pass whether raising full regions changed the
//!   stream, else the cheapest inflation along the route the walk
//!   recorded — with the `grow` submodule's splice emit
//!   rebuilding the one chosen root-to-leaf path.
//! - [`text`] renders and parses the paper's text notation directly on
//!   the streams.
//!
//! Machinery layers sit under the operations, each with its own essay:
//! `overlay` (the tiling cursors and the advance law every merge steps by),
//! `walk` (the leaf-walk driver the single-stream scanning passes share),
//! `watermark` (the anchored-minimum web the fill walk and the min-ticks fold
//! share), and `signed` (the sign-magnitude currency: the zigzag maps, the
//! signed folds and sums, the gamma codes). A submodule is public exactly where
//! the meter integration suite path-names it; the machinery stays
//! crate-private.
//!
//! Every kernel is differentially pinned against the recursive oracle
//! (`crate::oracle`), and the meter surface re-exports the module
//! ([`crate::meter::skyline`]) so the resource-envelope suite can pin its
//! internals.
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
//! prefix code with exactly one spelling per natural, and the zigzag map is a
//! bijection with no negative-zero form (odd codes decode to magnitude `>= 1`),
//! so a non-minimal gamma code or a non-canonical zigzag spelling cannot be
//! written at all. The tests pin both bijections exhaustively at small scope.
//!
//! Canonical skyline streams are unique representations of the step function:
//! minimal topology makes the tree unique, and heights are function-determined.
//! Byte-equality is therefore semantic equality on this coding, and the
//! construction-language transcoder (`encode_bits`, from the generators'
//! min-lifted packed preorder streams) lands exactly on the one canonical
//! stream per value.
//!
//! # Validation cost
//!
//! [`validate`](fn@validate) runs one forward pass holding, per open ancestor,
//! two bits — "is my left child complete" and "was that child a leaf" — on a
//! packed bit stack, plus one [`Accumulator`](suanpan::Accumulator) carrying
//! the running leaf height for the nonnegativity check. The bit stack costs ~2
//! bits per level where machine-word parse frames would cost tens of bytes; the
//! resource-envelope suite (`tests/meter.rs`) pins both that transient and the
//! validator's limb behavior.
//!
//! The accumulator choice is load-bearing, not an optimization: on the boundary
//! comb (`meter::cliff_comb`) the payload stream is 3-bit `±1` codes sitting
//! exactly on a `2^k` carry boundary, so a plain big-integer running height
//! pays a full `k`-bit carry per 3-bit delta — `Θ(W²)` limb work in skyline
//! wire bits `W` (`meter::tier2`'s plain-sweep pin measures it). The balanced
//! signed-digit [`Accumulator`](suanpan::Accumulator) applies a small delta and
//! answers the sign check in amortized O(1) digit touches on every input
//! sequence, so validation stays linear per wire bit; the envelope suite pins
//! the per-delta touch cost flat across size doublings on the comb.
//!
//! # Cost of encode and decode
//!
//! The stored form *is* this coding, so neither byte-level entry point
//! transcodes anything: [`encode`](fn@encode) clones the stored stream, and
//! [`decode`](fn@decode) runs [`validate`](fn@validate)'s wire-bit-linear pass
//! and then adopts the accepted bits as the version's storage directly — no
//! height, base, or node is materialized beyond validation's one payload in
//! flight. The construction-language transcoder (`encode_bits`, test- and
//! meter-only) is the one walk that materializes path sums, priced by the
//! packed stream it reads.
//!
//! # Testing
//!
//! - **Length agreement**: the encoder's output length equals
//!   [`crate::meter::tier2::tier2_size`] bit for bit, proptested over every
//!   adversarial generator family, arbitrary trees, and organic histories.
//!   The sizer and the encoder implement the coding independently (separate
//!   walks, separate zigzag maps), so agreement cross-checks both.
//! - **Round-trip**: `decode(encode(v))` reproduces `v` exactly (stored
//!   byte equality, which canonical uniqueness makes semantic equality).
//! - **Byte uniqueness**: op-path-independent equality — value-equal
//!   versions built along different operation paths produce identical
//!   skyline bytes; exhaustive small-scope injectivity over all normal
//!   forms to the depth bound the test-only `testing::exhaustive` module
//!   states and argues.
//! - **Strict rejection**: a deterministic corpus per reject genre (zero
//!   right-sibling delta, negative running height, truncation at every cut
//!   point, trailing bits), each with its exact error; plus a
//!   mutation proptest — flipping any single bit of a valid encoding
//!   either rejects or round-trips to a *different* version whose canonical
//!   encoding is the mutated stream itself.
//! - **Resource envelopes**: `tests/meter.rs` pins validate/decode heap,
//!   segment, limb, and accumulator-touch envelopes on the adversarial
//!   families.

#[cfg(any(test, feature = "meter"))]
use crate::error::Decode;
#[cfg(any(test, feature = "meter"))]
use crate::Version;

// The storage forms, re-exported so the resource-envelope suite can name the
// streams this module's entry points exchange. The frozen form rides the meter
// gate: only the suite (and the public docs the meter feature exposes) name it
// through this module.
#[cfg(any(test, feature = "meter"))]
pub use crate::codec::Bits;
#[cfg(any(test, feature = "meter"))]
pub use crate::codec::BitsMut;
pub use crate::codec::BitsView;

// The admission walk: the span wire form's fused second-component parse,
// consumed by `Span::decode` and the borsh span leg.
mod admit;
mod build;
pub(crate) mod place;
// The strict byte-level decode of one whole stream: consumed by this module's
// `decode` entry, which only the meter surface and the tests reach (production
// decode paths run `validate_prefix` + `from_bits`).
#[cfg(any(test, feature = "meter"))]
mod decode;
pub mod emit;
// The generators' construction-language transcoder: consumed by the meter
// surface and the transcoding tests only.
#[cfg(any(test, feature = "meter"))]
mod encode;
pub(crate) mod fill;
pub(crate) mod grow;
// Literal skyline construction from paper-notation event trees: the doctest
// and unit-test vocabulary's builder.
pub(crate) mod literal;
pub(crate) mod masked;
// The overlay cursors and the advance law: crate-private walk machinery shared
// by every merge.
pub(crate) mod overlay;
pub mod query;
mod signed;
pub mod sweep;
pub mod text;
mod validate;
// The leaf-walk driver: the descend/backtrack skeleton and shared leaf
// actions of the single-stream scanning passes.
mod walk;
// The anchored-minimum web the fill walk and the min-ticks fold share: the
// range-minimum discipline stated once, each client thin over it.
mod watermark;
// The web's accumulator-pool miss counter: leases the pool could not
// serve, read through `meter::pool_misses`.
pub(crate) mod pool_traffic;
// The web's domination-read counters: which emission arm answered, read
// through `meter::emit_traffic`.
pub(crate) mod web_traffic;

#[cfg(test)]
mod tests;

pub(crate) use admit::{validate_dominating_from, Admission};
#[cfg(any(test, feature = "meter"))]
pub(crate) use decode::decode_bits;
#[cfg(any(test, feature = "meter"))]
pub(crate) use encode::encode_bits;
#[cfg(any(test, feature = "meter"))]
pub(crate) use validate::validate_bits;
pub(crate) use validate::validate_prefix;
// The wire decoder's from-cursor entry: reached from the borsh event leg.
#[cfg(feature = "borsh")]
pub(crate) use validate::validate_from;

/// A build buffer's contents as a [`BitsView`]: the instrument surface's
/// bridge from [`encode`]'s buffer to the view the walk entries read.
///
/// Test- and meter-only, like the buffers it views.
#[cfg(any(test, feature = "meter"))]
pub fn view(bits: &BitsMut) -> BitsView<'_> {
    crate::codec::built_view(bits)
}

/// A [`Version`]'s canonical skyline stream: the stored form, cloned.
///
/// Test- and meter-only: production callers reach the stored stream through
/// [`Version::as_bytes`]/[`Version::encode`].
#[cfg(any(test, feature = "meter"))]
pub fn encode(version: &Version) -> BitsMut {
    let view = version.as_bits();
    let mut bits = BitsMut::with_capacity(view.len() as usize);
    // Live bits only — no padding: consumers walk these as a stream.
    crate::codec::extend_from_view(&mut bits, view, 0, view.len());
    bits
}

/// Strictly validate a skyline stream without materializing any height.
///
/// Enforces the module doc's canonical form in one forward pass: minimal
/// topology and stream exactness on ~2 bits of stack per open ancestor, and
/// leaf-height nonnegativity on the cliff-free accumulator. Every error is
/// [`Decode::Truncated`] (the stream ended mid-tree or mid-integer),
/// [`Decode::TrailingBits`] (live bits remain after the tree), or
/// [`Decode::NotCanonical`] (collapsible sibling leaves, or a delta driving the
/// running height negative).
///
/// Test- and meter-only: the production byte-level entries
/// ([`Version::decode`], the borsh leg) run the underlying pass through
/// `validate_prefix`/`validate_from` directly.
#[cfg(any(test, feature = "meter"))]
pub fn validate(bits: BitsView<'_>) -> Result<(), Decode> {
    validate_bits(bits)
}

/// Strictly validate a skyline stream and wrap it as a [`Version`].
///
/// [`validate`](fn@validate)'s pass runs first and gates acceptance, bit for
/// bit; the accepted stream then becomes the version's storage directly (the
/// stored form *is* this coding), so decoding materializes nothing beyond the
/// copy.
///
/// Test- and meter-only: the production decode ([`Version::decode`]) validates
/// the prefix and adopts the buffer without this wrapper.
#[cfg(any(test, feature = "meter"))]
pub fn decode(bits: BitsView<'_>) -> Result<Version, Decode> {
    decode_bits(bits)
}

/// Whether a skyline stream is the canonical empty version.
///
/// The empty version is exactly the 2-bit stream `11` (leaf flag `1`, then
/// gamma(0), the single bit `1`). Canonical uniqueness makes this O(1) test the
/// whole question.
pub(crate) fn is_empty_stream(bits: BitsView<'_>) -> bool {
    bits.len() == 2 && bits.bit(0) && bits.bit(1)
}
