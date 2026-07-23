//! Adversarial input generators and deterministic resource meters.
//!
//! This module is the measurement half of the crate's resource-proportionality
//! work: transient cost — peak heap, stack segments, big-integer limb work —
//! as a function of packed input size, with no bound on value magnitude, tree
//! depth, or encoded size.
//! The generators below build the canonical packed encodings that maximize
//! each cost against its input size; the meters read the deterministic
//! counters the envelopes are pinned against. Public under the `meter`
//! feature so the metering test binaries (and benches) can reach it; never
//! part of a production build.
//!
//! Every generator output is strict normal form: it round-trips through
//! [`Party::decode`](crate::Party::decode)/[`Version::decode`](crate::Version::decode)
//! and re-encodes byte-identically,
//! and its exact bit length is a closed formula in the parameters (pinned by
//! this module's tests). Bit layouts follow the crate codec: an event node is
//! a flag bit plus the Elias-gamma code of its base (`gamma(n)` codes
//! `m = n + 1`); an id node is a 2-bit child-presence tag, absent children
//! occupying no bits.

pub mod board;
pub mod tier2;

/// The cliff-immune signed accumulator, re-exported so the resource-envelope
/// suite can drive its delta streams and pin its digit-touch cost.
pub use crate::codec::accum;

use crate::codec::{self, Base, Bits};

/// A generator's output: canonical packed bytes plus the exact bit length.
///
/// `bytes` is what `decode` accepts and `encode` reproduces (final partial
/// byte zero-padded); `bits` is the live bit length before that padding, so
/// tests can pin the closed-form size of each shape.
#[derive(Debug, Clone)]
pub struct Packed {
    /// The canonical packed bytes, zero-padded to a byte boundary.
    pub bytes: Vec<u8>,
    /// The exact number of live bits in `bytes` before the zero pad.
    pub bits: usize,
}

impl Packed {
    /// Canonicalize a built bit stream: zero the dead pad bits and keep the
    /// live length.
    fn from_bits(mut bits: Bits) -> Self {
        let len = bits.len();
        codec::zero_dead_bits(&mut bits);
        Packed {
            bytes: bits.into_vec(),
            bits: len,
        }
    }
}

/// Append an event leaf with base `n`: flag `0`, then `gamma(n)`.
fn ev_leaf(bits: &mut Bits, n: u64) {
    bits.push(false);
    codec::encode_int(bits, &Base::from(n));
}

/// Append the dense event spine body: `d` zero-base internal nodes leaning
/// left, each with a 0-leaf right sibling, bottoming out in `(0, 0, 1)`.
///
/// Layout: `"11" × d` (internal flag + `gamma(0)`), `"01"` (bottom-left leaf
/// 0), `"0010"` (bottom-right leaf 1), `"01" × (d − 1)` (each ancestor's
/// right sibling). Exactly `4d + 4` bits for `2d + 1` nodes at depth `d` —
/// the densest shape normal form admits (~2 bits per node, depth ~n/4 for
/// `n` bits), maximizing node count and recursion depth simultaneously.
/// Normal form holds everywhere: each internal node's spine child has base
/// 0, and the only leaf pair is `(0, 1)`.
fn ev_spine(bits: &mut Bits, d: usize) {
    for _ in 0..d {
        bits.push(true); // internal-node flag
        codec::encode_int(bits, &Base::from(0u8)); // gamma(0) = "1"
    }
    ev_leaf(bits, 0); // bottom node's left child
    ev_leaf(bits, 1); // bottom node's right child: distinct, so no collapse
    for _ in 0..d - 1 {
        ev_leaf(bits, 0); // each ancestor's right sibling
    }
}

/// The dense event spine `S(d)`: depth `d`, `2d + 1` nodes, `4d + 4` bits.
///
/// The node-count and recursion-depth maximizer; drives every per-node and
/// per-level cost (decode parse stacks, walk frames, working-form arrays) to
/// its worst case per input bit.
///
/// # Panics
///
/// Panics if `d == 0`: the spine needs at least one internal node.
pub fn dense(d: usize) -> Packed {
    assert!(d >= 1, "dense spine needs at least one internal node");
    let mut bits = Bits::with_capacity(4 * d + 4);
    ev_spine(&mut bits, d);
    Packed::from_bits(bits)
}

/// A root with base `2^b − 1` over `S(d)` and a 0-leaf: `2b + 4d + 8` bits.
///
/// Layout: `"1" · gamma(2^b − 1) · S(d) · "01"`, where
/// `gamma(2^b − 1) = 0^b · 1 · 0^b` (`2b + 1` bits). Puts a `b`-bit magnitude
/// on every root-to-node path sum while keeping paths long — the shape that
/// makes owned per-frame path sums quadratic in the input.
///
/// # Panics
///
/// Panics if `b == 0` or `d == 0`.
pub fn bigroot(b: usize, d: usize) -> Packed {
    assert!(b >= 1, "bigroot needs a nonzero root magnitude");
    assert!(d >= 1, "bigroot needs a nonzero spine depth");
    let mut bits = Bits::with_capacity(2 * b + 4 * d + 8);
    bits.push(true); // root node flag
    codec::encode_int(&mut bits, &pow2_minus_1(b));
    ev_spine(&mut bits, d); // left child: the dense spine (its root has base 0)
    ev_leaf(&mut bits, 0); // right child: the root's required zero-base leaf
    Packed::from_bits(bits)
}

/// A single event leaf of value `2^b − 1`: one node, `2b + 2` bits.
///
/// Maximizes bit length per node: the whole input is one gamma code, so any
/// cost superlinear in a single code's width (big-integer accumulation,
/// size-from-bit-length allocations) shows up undiluted.
///
/// # Panics
///
/// Panics if `b == 0`.
pub fn hugeleaf(b: usize) -> Packed {
    assert!(b >= 1, "hugeleaf needs a nonzero magnitude");
    let mut bits = Bits::with_capacity(2 * b + 2);
    bits.push(false); // leaf flag
    codec::encode_int(&mut bits, &pow2_minus_1(b));
    Packed::from_bits(bits)
}

/// The boundary comb `C(k, n)`: `n` cliff teeth, `n(2k + 10) + 2` bits.
///
/// A zero-base spine leaning right, each spine node's left child a *tooth*
/// `(2^k − 1, 0, 1)` — an internal node with base `2^k − 1` over leaves 0
/// and 1 — terminated by a leaf 0. Its preorder leaf values oscillate
/// `2^k − 1 ↔ 2^k`: every consecutive-leaf difference is `±1` sitting
/// exactly on the `2^k` carry boundary, so any sweep that maintains a
/// running leaf value (or a running difference of leaf values) pays a full
/// `k`-bit carry or borrow per crossing. In this coding each tooth stores
/// its own `gamma(2^k − 1)` — `2k + 1` bits — so every crossing is paid for
/// by a comparably-wide input code and operations stay linear per input
/// bit; a delta coding of the same tree stores 3-bit `±1` codes per
/// crossing instead, which is what makes this the separating family for the
/// leaf-delta representation question.
///
/// Layout per tooth: `"11"` (spine node, `gamma(0)`),
/// `"1" · gamma(2^k − 1)` (tooth node), `"01"` (leaf 0),
/// `"0011"` (leaf 1); after all `n` teeth, `"01"` (the terminal leaf 0).
/// `2k + 10` bits per tooth plus 2, over `4n + 1` nodes of which `2n + 1`
/// are leaves. Normal form holds everywhere: each spine node's right child
/// has base 0, each tooth's left leaf has base 0, and the only leaf pairs
/// are `(0, 1)`.
///
/// # Panics
///
/// Panics if `k == 0` or `n == 0`.
pub fn cliff_comb(k: usize, n: usize) -> Packed {
    assert!(k >= 1, "cliff comb needs a nonzero tooth magnitude");
    assert!(n >= 1, "cliff comb needs at least one tooth");
    let mut bits = Bits::with_capacity(n * (2 * k + 10) + 2);
    let tooth = pow2_minus_1(k);
    for _ in 0..n {
        bits.push(true); // spine node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
        bits.push(true); // tooth node flag
        codec::encode_int(&mut bits, &tooth);
        ev_leaf(&mut bits, 0); // tooth's left leaf: value 2^k − 1
        ev_leaf(&mut bits, 1); // tooth's right leaf: value 2^k
    }
    ev_leaf(&mut bits, 0); // terminal spine leaf
    Packed::from_bits(bits)
}

/// The id spine `I(d, divert)`: a unary chain of depth `d`, `2d + 2` bits.
///
/// Layout: `d` left-only tags (`10`) ending in a terminal (`00`). With
/// `divert`, the last unary node is right-only (`01`) instead, so
/// `I(d, false)` and `I(d, true)` share their first `d − 1` levels and own
/// disjoint regions — the pair shape that drives two-operand id walks to
/// full lockstep depth. Normal form: no `(1, 1)` node anywhere.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn id_spine(d: usize, divert: bool) -> Packed {
    assert!(d >= 1, "id spine needs at least one unary node");
    let mut bits = Bits::with_capacity(2 * d + 2);
    for _ in 0..d - 1 {
        bits.push(true); // left child present ...
        bits.push(false); // ... right child absent
    }
    // The last unary node: left-only, or right-only when diverted.
    bits.push(!divert);
    bits.push(divert);
    bits.push(false); // terminal tag "00": the single owned tip
    bits.push(false);
    Packed::from_bits(bits)
}

/// The base `2^b − 1`, whose gamma code is `0^b · 1 · 0^b`.
fn pow2_minus_1(b: usize) -> Base {
    let b = u32::try_from(b).expect("magnitude bit count fits u32");
    (Base::from(1u8) << b) - &Base::from(1u8)
}

/// The number of heap stack segments the deep traversals have grown since the
/// last [`reset_stack_segments`].
///
/// The deterministic stand-in for recursion-driven stack consumption: the
/// segments the stack guard allocates never pass through the global
/// allocator, so no heap meter can see them; this reads the counter bumped at
/// the one place a segment is created. Process-global — meaningful per
/// scenario only under one-scenario-per-process isolation (nextest's model)
/// or a single-threaded caller.
pub fn stack_segments() -> u64 {
    crate::recurse::segments_grown()
}

/// Reset the grown-segment counter behind [`stack_segments`] to zero.
pub fn reset_stack_segments() {
    crate::recurse::reset_segments_grown()
}

/// The big-integer limb operations counted since the last [`reset_limb_ops`].
///
/// The deterministic stand-in for arithmetic-width cost, which no other meter
/// can see: a magnitude blowup allocates little and visits no extra nodes —
/// the work is wider, not more frequent. The count is the operands' 64-bit
/// limb counts per `Base` operation (arithmetic, comparison, equality, and
/// hashing) plus one value-width record
/// per wide-gamma decode, so an amortized-linear algorithm counts
/// linearly in packed input bits and a magnitude-quadratic one counts
/// quadratically. Process-global, same isolation requirement as
/// [`stack_segments`]; only compiled under the `limb-meter` feature, which
/// adds the counting to the arithmetic itself.
#[cfg(feature = "limb-meter")]
pub fn limb_ops() -> u64 {
    crate::codec::limb_meter::limb_ops()
}

/// Reset the limb-operation counter behind [`limb_ops`] to zero.
#[cfg(feature = "limb-meter")]
pub fn reset_limb_ops() {
    crate::codec::limb_meter::reset()
}

#[cfg(test)]
mod tests;
