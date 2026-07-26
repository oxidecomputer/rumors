//! Adversarial input generators and deterministic resource meters.
//!
//! This module is the measurement half of the crate's resource-proportionality
//! work: transient cost — peak heap, stack segments, big-integer limb work,
//! packed-stream scan work — as a function of packed input size, with no
//! bound on value magnitude, tree depth, or encoded size.
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

/// The skyline transcoding codec, re-exported so the resource-envelope
/// suite can pin its validator's transient state and limb behavior.
pub use crate::version::skyline;

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

    /// The generator's live bits, borrowed.
    pub fn as_bits(&self) -> &codec::BitsSlice {
        &codec::bytes_as_bits(&self.bytes)[..self.bits]
    }

    /// Lift an event-shape generator's output into a stored [`Version`](crate::Version),
    /// transcoding the construction language (a min-lifted packed preorder
    /// stream) into the skyline coding the version stores.
    pub fn version(&self) -> crate::Version {
        crate::Version::from_bits(skyline::encode_bits(self.as_bits()))
    }
}

/// Append an event leaf with base `n`: flag `0`, then `gamma(n)`.
fn ev_leaf(bits: &mut Bits, n: u64) {
    ev_leaf_wide(bits, &Base::from(n));
}

/// Append an event leaf with an arbitrary-width stored base: flag `0`, then
/// `gamma(base)`.
fn ev_leaf_wide(bits: &mut Bits, base: &Base) {
    bits.push(false);
    codec::encode_int(bits, base);
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
/// per-level cost (walk frames and per-level stack state) to
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

/// The jump comb `J(k, n)`: one low tooth, then `n − 1` cliff teeth,
/// `(n − 1)(2k + 10) + 14` bits.
///
/// The boundary comb with its first tooth lowered to `(1, 0, 1)`: preorder
/// leaf values run `1, 2`, jump to `2^k − 1`, then oscillate
/// `2^k − 1 ↔ 2^k`. In a delta coding the jump is the one wide
/// leaf-to-leaf code, arriving mid-stream with only 3-bit codes behind
/// it — the stale-drift shape: a sweep that keeps running height state
/// must move the jump out of its cheap-delta path exactly once, paid by
/// the jump's own code, or pay the jump's width again on every 3-bit
/// delta that follows. The wide-tooth comb prices bounded wide
/// oscillation (state that must *stay* live); this family prices the
/// eviction (state that must *leave*), so together they pin a height
/// split from both sides.
///
/// Layout: one tooth `"11" · "1" · gamma(1) · "01" · "0010"` (12 bits),
/// then `n − 1` [`cliff_comb`] teeth at `2k + 10` bits, then the terminal
/// `"01"`. Normal form holds everywhere by the comb's own argument: every
/// spine node's right child has base 0, every tooth's left leaf has base
/// 0, and the only leaf pairs are `(0, 1)`.
///
/// # Panics
///
/// Panics if `k == 0` or `n < 2`: the jump needs a low tooth and at
/// least one cliff tooth to jump between.
pub fn jump_comb(k: usize, n: usize) -> Packed {
    assert!(k >= 1, "jump comb needs a nonzero cliff magnitude");
    assert!(n >= 2, "jump comb needs a low tooth and a cliff tooth");
    let mut bits = Bits::with_capacity((n - 1) * (2 * k + 10) + 14);
    let tooth = pow2_minus_1(k);
    let one = Base::from(1u8);
    for i in 0..n {
        bits.push(true); // spine node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
        bits.push(true); // tooth node flag
        codec::encode_int(&mut bits, if i == 0 { &one } else { &tooth });
        ev_leaf(&mut bits, 0); // tooth's left leaf
        ev_leaf(&mut bits, 1); // tooth's right leaf: distinct, no collapse
    }
    ev_leaf(&mut bits, 0); // terminal spine leaf
    Packed::from_bits(bits)
}

/// The wide-tooth comb `W(k, w, n)`: `n` teeth of width `2^w` oscillating
/// across the `2^k` cliff, `n(2k + 2w + 6) + 2` bits.
///
/// The boundary comb's wide-delta sibling: the same zero-base spine, each
/// tooth `(2^k − 2^w, 0, 2^w)` — an internal node with base `2^k − 2^w` over
/// leaves 0 and `2^w` — terminated by a leaf 0. Its preorder leaf values
/// oscillate `2^k − 2^w ↔ 2^k`: every consecutive-leaf difference is
/// `±2^w`, and applying it carries or borrows across the `k − w` bits up to
/// the `2^k` boundary. Machine-word deltas are what a fixed-width lazy
/// window absorbs, so this family prices the deltas *wider than any such
/// window*: a two-zone accumulator (normalized prefix plus fixed-width
/// buffer) is forced through its normalized prefix on every tooth, while a
/// representation with no normalized region pays O(delta limbs). Each tooth
/// stores `gamma(2^k − 2^w)` — `2k − 1` bits — so under today's coding every
/// crossing is paid for by a comparably-wide input code.
///
/// Layout per tooth: `"11"` (spine node, `gamma(0)`),
/// `"1" · gamma(2^k − 2^w)` (tooth node), `"01"` (leaf 0),
/// `"0" · gamma(2^w)` (leaf `2^w`); after all `n` teeth, `"01"` (the
/// terminal leaf 0). `2k + 2w + 6` bits per tooth plus 2. Normal form holds
/// everywhere: each spine node's right child has base 0, each tooth's left
/// leaf has base 0, and the only leaf pairs are `(0, 2^w)`.
///
/// # Panics
///
/// Panics if `w == 0`, `w ≥ k`, or `n == 0`.
pub fn wide_tooth_comb(k: usize, w: usize, n: usize) -> Packed {
    assert!(w >= 1, "wide-tooth comb needs a nonzero tooth width");
    assert!(
        w < k,
        "wide-tooth comb needs its cliff above its tooth width"
    );
    assert!(n >= 1, "wide-tooth comb needs at least one tooth");
    let mut bits = Bits::with_capacity(n * (2 * k + 2 * w + 6) + 2);
    let tooth_width = pow2(w);
    let tooth_base = pow2(k) - &tooth_width;
    for _ in 0..n {
        bits.push(true); // spine node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
        bits.push(true); // tooth node flag
        codec::encode_int(&mut bits, &tooth_base);
        ev_leaf(&mut bits, 0); // tooth's left leaf: value 2^k − 2^w
        ev_leaf_wide(&mut bits, &tooth_width); // tooth's right leaf: value 2^k
    }
    ev_leaf(&mut bits, 0); // terminal spine leaf
    Packed::from_bits(bits)
}

/// The unpaid-crossing fan `F(k, n)`: `n` cheap teeth under one stored
/// magnitude, `12n + 2k + 6` bits.
///
/// A root with base `2^k − 1` whose left child is a zero-base fan spine of
/// `n` teeth `(1, 0, 1)` — an internal node with base 1 over leaves 0 and
/// 1 — terminated by a leaf 0, with the root's required 0-leaf on the
/// right. The root-to-node path sum sits at `2^k − 1` across the whole fan,
/// so a walk that maintains a running path sum (enter: add the stored base;
/// leave: subtract it) crosses the `2^k` carry boundary *twice per tooth* —
/// and each tooth costs only 12 stored bits. One comparably-coded magnitude
/// (the root's, paid once) funds `n` crossings: the excursions are
/// siblings, not nested, so no Dyck-structure argument bounds them, and any
/// accumulator that materializes each crossing as a full-width carry does
/// Θ(nk) limb work in a Θ(n + k)-bit input. Consecutive-leaf *values* stay
/// cliff-free (`2^k ↔ 2^k + 1`): the fan prices entry/exit accumulation,
/// the boundary comb prices leaf deltas.
///
/// Layout: `"1" · gamma(2^k − 1)` (root), then per tooth `"11"` (spine
/// node), `"1" · gamma(1)` (tooth node), `"01"` (leaf 0), `"0010"`
/// (leaf 1); after all `n` teeth, `"01"` (terminal fan leaf), `"01"` (the
/// root's right leaf). Normal form holds everywhere: the root's right leaf
/// and each spine node's non-tooth child have base 0, and the only leaf
/// pairs are `(0, 1)`.
///
/// # Panics
///
/// Panics if `k == 0` or `n == 0`.
pub fn cliff_fan(k: usize, n: usize) -> Packed {
    assert!(k >= 1, "cliff fan needs a nonzero root magnitude");
    assert!(n >= 1, "cliff fan needs at least one tooth");
    let mut bits = Bits::with_capacity(12 * n + 2 * k + 6);
    bits.push(true); // root node flag
    codec::encode_int(&mut bits, &pow2_minus_1(k));
    let one = Base::from(1u8);
    for _ in 0..n {
        bits.push(true); // fan spine node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
        bits.push(true); // tooth node flag
        codec::encode_int(&mut bits, &one); // gamma(1) = "010"
        ev_leaf(&mut bits, 0); // tooth's left leaf: value 2^k
        ev_leaf(&mut bits, 1); // tooth's right leaf: value 2^k + 1
    }
    ev_leaf(&mut bits, 0); // terminal fan leaf
    ev_leaf(&mut bits, 0); // the root's required zero-base right leaf
    Packed::from_bits(bits)
}

/// The cancelling-prefix chain `P(k, n)`: `n` peak-to-1 drops,
/// `n(2k + 10) + 2` bits.
///
/// The boundary comb's shape with the wide magnitude moved onto the *left*
/// leaf: teeth `(1, 2^k − 1, 0)` off a zero-base spine, terminated by a
/// leaf 0, so preorder leaf values oscillate `2^k ↔ 1`. Each drop from the
/// peak leaves a running-value accumulator holding a tiny value spelled
/// with wide digits — a high positive digit cancelled by a trail of
/// negative ones — so the next sign check cannot decide at the top digit
/// and must scan (and collapse) the whole cancelling prefix. Every drop is
/// paid by its own `gamma(2^k − 1)` input code, so the family prices deep
/// sign scans against the wide writes that immediately precede them. It
/// does not exercise the collapse: a scan funded by an adjacent write is
/// linear whether or not the fold rewrites what it scanned. The
/// collapse-is-load-bearing case — a cancelling prefix built once, then
/// read many times — is a delta-stream shape, not a packed input, and is
/// pinned by the accumulator envelope suite's static-prefix stream.
///
/// Layout per tooth: `"11"` (spine node, `gamma(0)`), `"1" · gamma(1)`
/// (tooth node), `"0" · gamma(2^k − 1)` (leaf `2^k − 1`), `"01"` (leaf 0);
/// after all `n` teeth, `"01"` (the terminal leaf 0). `2k + 10` bits per
/// tooth plus 2. Normal form holds everywhere: each spine node's right
/// child has base 0, each tooth's right leaf has base 0, and the only leaf
/// pairs are `(2^k − 1, 0)`.
///
/// # Panics
///
/// Panics if `k == 0` or `n == 0`.
pub fn cancelling_chain(k: usize, n: usize) -> Packed {
    assert!(k >= 1, "cancelling chain needs a nonzero peak magnitude");
    assert!(n >= 1, "cancelling chain needs at least one tooth");
    let mut bits = Bits::with_capacity(n * (2 * k + 10) + 2);
    let peak_drop = pow2_minus_1(k);
    let one = Base::from(1u8);
    for _ in 0..n {
        bits.push(true); // spine node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
        bits.push(true); // tooth node flag
        codec::encode_int(&mut bits, &one); // gamma(1) = "010"
        ev_leaf_wide(&mut bits, &peak_drop); // left leaf: value 2^k
        ev_leaf(&mut bits, 0); // right leaf: value 1
    }
    ev_leaf(&mut bits, 0); // terminal spine leaf
    Packed::from_bits(bits)
}

/// The harmonic spine `H(d)`: a 1-leaf at every depth, `6d + 2` bits, rank
/// `(2^d − 1)/2^d`.
///
/// `d` zero-base internal nodes leaning left, each with a 1-leaf right
/// sibling, bottoming out in a 0-leaf: level `i`'s leaf contributes area
/// `1/2^i`, so the whole tree's rank telescopes to `(2^d − 1)/2^d` — the
/// closed form this module's tests pin. The numerator is the all-ones
/// `d`-bit odd integer, so the rank fold's running numerator is as wide as
/// the depth already walked at *every* level: any fold that re-shifts its
/// accumulated numerator per level does `Θ(d²)` limb work against `Θ(d)`
/// input bits, which is what makes this the separating family for the
/// rank/distance/lag delta algebra. [`dense`] is the control: same density,
/// but its single 1-leaf keeps the fold's numerator one bit wide.
///
/// Layout: `"11" × d` (internal flag + `gamma(0)`), `"01"` (the bottom
/// 0-leaf), then `"0010" × d` (each level's 1-leaf right sibling,
/// innermost first). Exactly `6d + 2` bits for `2d + 1` nodes at depth `d`.
/// Normal form holds everywhere: each internal node's left child stores
/// base 0, and the only sibling leaf pair is the bottom `(0, 1)`.
///
/// # Panics
///
/// Panics if `d == 0`: the spine needs at least one internal node.
pub fn harmonic(d: usize) -> Packed {
    assert!(d >= 1, "harmonic spine needs at least one internal node");
    let mut bits = Bits::with_capacity(6 * d + 2);
    for _ in 0..d {
        bits.push(true); // internal-node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
    }
    ev_leaf(&mut bits, 0); // the bottom node's left child
    for _ in 0..d {
        ev_leaf(&mut bits, 1); // each level's right sibling: value 1
    }
    Packed::from_bits(bits)
}

/// The alternating-binary spine `A(d)`: depth `d`, `2d + 1` nodes, `4d + 4`
/// bits.
///
/// The dense spine's direction-alternating sibling: `d` zero-base internal
/// nodes whose single internal child sits *left at even depths and right at
/// odd depths*, each with a 0-leaf sibling, bottoming out in `(0, 0, 1)`.
/// Same density as [`dense`] (~2 bits per node, depth ~n/4 for `n` bits),
/// but the root-to-bottom route changes direction every level, so any
/// per-level saved state — walk frames, route bits, resume records — is
/// maximally non-uniform: nothing about a frame can be inferred from its
/// neighbors, which makes this the frame-count adversary for iterative
/// walks that keep per-level records (a fixed 16-byte frame per level costs
/// ~32 bytes per input byte here).
///
/// Layout: at each level, `"11"` (internal node, `gamma(0)`), preceded by
/// `"01"` when the internal child sits right (the leaf sibling is emitted
/// first in preorder); the bottom node is `"01" · "0010"` (leaves 0, 1);
/// unwinding, each level whose internal child sat left emits its trailing
/// `"01"` sibling. Normal form holds everywhere: every internal node has a
/// base-0 child, and the only leaf pair is `(0, 1)`.
///
/// # Panics
///
/// Panics if `d == 0`: the spine needs at least one internal node.
pub fn alt_spine(d: usize) -> Packed {
    assert!(d >= 1, "alternating spine needs at least one internal node");
    let mut bits = Bits::with_capacity(4 * d + 4);
    // Levels 0..d−1 have one internal child each (left at even levels,
    // right at odd); level d−1 is the bottom node with leaves (0, 1).
    for level in 0..d {
        bits.push(true); // internal-node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
        if level + 1 < d && level % 2 == 1 {
            ev_leaf(&mut bits, 0); // leaf sibling first: internal child right
        }
    }
    ev_leaf(&mut bits, 0); // bottom node's left child
    ev_leaf(&mut bits, 1); // bottom node's right child: distinct, no collapse
    for level in (0..d.saturating_sub(1)).rev() {
        if level % 2 == 0 {
            ev_leaf(&mut bits, 0); // trailing leaf sibling: internal child left
        }
    }
    Packed::from_bits(bits)
}

/// The scattered id `Z(e)`: `e` owned left subtrees at alternating depths of
/// a right-leaning spine, `6e + 2` bits.
///
/// The operand cross's id side for output-dominated projection: the party
/// owns the whole left child at every other level of a right-leaning spine —
/// the positions where [`cliff_comb`]'s teeth hang — so projecting a comb
/// through it keeps every second tooth. `e` disjoint owned fragments scatter
/// across the id tree at `Θ(1)` stored bits each, so the *input* is linear
/// in `e` while every kept fragment boundary forces a fresh wide magnitude
/// into the projected *output*.
///
/// Layout, repeated `e` times: `11` (both children present), `00` (the owned
/// left leaf), `01` (a right-only gap level); terminated by `00` (the owned
/// tip). 6 bits per fragment plus 2. Normal form: no node has two
/// fully-owned children (each `11` node's right child is a gap node) and no
/// node has two absent children.
///
/// # Panics
///
/// Panics if `e == 0`.
pub fn scattered_id(e: usize) -> Packed {
    assert!(e >= 1, "scattered id needs at least one owned fragment");
    let mut bits = Bits::with_capacity(6 * e + 2);
    for _ in 0..e {
        bits.push(true); // fragment node: left child present ...
        bits.push(true); // ... and the spine continues right
        bits.push(false); // the owned left leaf: terminal tag "00"
        bits.push(false);
        bits.push(false); // gap node: left child absent ...
        bits.push(true); // ... spine continues right
    }
    bits.push(false); // terminal tag "00": the owned tip
    bits.push(false);
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

/// The nested-full-sibling id `N(d)`: `(x, 1)` repeated down a left
/// spine, `4d + 4` bits.
///
/// Layout: `d` both-children tags (`11`) descending left, then the
/// left-only terminus `(1, 0)` (`10 · 00`; a `(1, 1)` terminus would
/// break normal form), then the `d` right-child terminals (`00`),
/// innermost first — preorder closes the spine's right children in
/// reverse. Every level is a right-full shortcut site over a matching
/// event spine: the deepest stacking of the fill walk's deferred
/// right-full decisions and per-level raise bookkeeping per input bit.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn nested_full_id(d: usize) -> Packed {
    assert!(d >= 1, "nested-full id needs at least one shortcut level");
    let mut bits = Bits::with_capacity(4 * d + 4);
    for _ in 0..d {
        bits.push(true); // left child present (the spine continues) ...
        bits.push(true); // ... and a right child follows it
    }
    bits.push(true); // the terminus: left-only node `(1, 0)` ...
    bits.push(false);
    bits.push(false); // ... whose left child is the terminal
    bits.push(false);
    for _ in 0..d {
        bits.push(false); // each level's right child: the full terminal
        bits.push(false);
    }
    Packed::from_bits(bits)
}

/// The mirror nested-full id `M(d)`: `(1, x)` repeated down a right
/// spine, `4d + 4` bits.
///
/// Layout: `d` both-children tags (`11`) descending right, each followed
/// immediately by its full left terminal (`00` — preorder visits the left
/// child first, so the terminals interleave with the spine tags instead
/// of trailing them), then the right-only terminus `(0, 1)` (`01 · 00`;
/// a `(1, 1)` terminus would break normal form). Every level is a
/// left-full shortcut site over a right-leaning event spine: the raised
/// leaf precedes the range its minimum comes from at every level, so the
/// walk's memoized pre-scan (and the pre-scan's own per-level
/// bookkeeping) runs at maximal nesting per input bit.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn nested_left_full_id(d: usize) -> Packed {
    assert!(
        d >= 1,
        "nested-left-full id needs at least one shortcut level"
    );
    let mut bits = Bits::with_capacity(4 * d + 4);
    for _ in 0..d {
        bits.push(true); // left child present (the full terminal) ...
        bits.push(true); // ... and the spine continues right
        bits.push(false); // the left child: the full terminal
        bits.push(false);
    }
    bits.push(false); // the terminus: right-only node `(0, 1)` ...
    bits.push(true);
    bits.push(false); // ... whose right child is the terminal
    bits.push(false);
    Packed::from_bits(bits)
}

/// A right-leaning spine of zero leaves with one `2^b − 1` tail leaf:
/// depth `d`, `4d + 2b + 3` bits.
///
/// Layout: `d` × (`1 · gamma(0) · 0 · gamma(0)`) — each spine node's
/// zero-base flag and its zero left leaf — then the bottom node's wide
/// right leaf `0 · gamma(2^b − 1)`. Preorder leaf heights are
/// `0, 0, …, 0, 2^b − 1`: every proper subtree of the spine nets
/// `+(2^b − 1)` from entry to exit while all minima stay at zero, so any
/// per-level bookkeeping that materializes subtree nets (rather than
/// carrying them relative to a shared anchor) re-touches the tail's
/// width once per level. Crossed with [`nested_left_full_id`], every
/// level is additionally a left-full pre-scan site.
///
/// # Panics
///
/// Panics if `b == 0` or `d == 0`.
pub fn wide_tail(b: usize, d: usize) -> Packed {
    assert!(b >= 1, "wide tail needs a nonzero magnitude");
    assert!(d >= 1, "wide tail needs a nonzero spine depth");
    let mut bits = Bits::with_capacity(4 * d + 2 * b + 3);
    for _ in 0..d {
        bits.push(true); // spine node flag ...
        codec::encode_int(&mut bits, &Base::from(0u8)); // ... base 0
        ev_leaf(&mut bits, 0); // its zero left leaf
    }
    ev_leaf_wide(&mut bits, &pow2_minus_1(b)); // the bottom's wide tail
    Packed::from_bits(bits)
}

/// The descending staircase `D(d)`: the dense left spine whose preorder
/// leaf heights descend `d, d − 1, …, 0` by unit deltas; `~5d` bits.
///
/// Layout: the root `1 · gamma(0)`, then `d − 1` × (`1 · gamma(1)`)
/// (each deeper spine node lifts its subtree's minimum by one), the
/// bottom pair `0 · gamma(1) · 0 · gamma(0)`, then `d − 1` right-sibling
/// zero leaves (`0 · gamma(0)`), innermost first. Min-lifted normal form
/// holds at every node (each node's right leaf sits exactly at its
/// subtree's minimum), and no leaf pair is equal. Every preorder leaf
/// undercuts every leaf before it, so under an id that pairs internal
/// down the whole spine (`id_spine`), every consumed leaf is a
/// full-penetration minimum update through all open ranges — the shape
/// that separates per-level minimum bookkeeping (quadratic) from
/// run-compressed propagation (linear), independent of value width.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn staircase(d: usize) -> Packed {
    assert!(d >= 1, "the staircase needs at least one internal node");
    let mut bits = Bits::with_capacity(5 * d + 8);
    bits.push(true); // the root: base 0 (the whole tree's minimum)
    codec::encode_int(&mut bits, &Base::from(0u8));
    for _ in 1..d {
        bits.push(true); // each deeper spine node ...
        codec::encode_int(&mut bits, &Base::from(1u8)); // ... lifts by 1
    }
    ev_leaf(&mut bits, 1); // bottom-left leaf: the staircase's top
    ev_leaf(&mut bits, 0); // bottom-right leaf: one step down
    for _ in 1..d {
        ev_leaf(&mut bits, 0); // each ancestor's right leaf: its floor
    }
    Packed::from_bits(bits)
}

/// The memo-chain event `Q(k, distinct)`: a right-leaning spine of `k`
/// single-leaf left-full sites, `~(14k + 9)` bits distinct (γ(j) codes),
/// `13k + 9` shared.
///
/// Layout: the root `1 · γ(0)` with left leaf `0 · γ(0)`, then per level
/// `j = 1..=k` the spine node `1 · γ(0)` over the site node
/// `1 · γ(0) · 0 · γ(0) · 0 · γ(v_j)` (leaves 0 and `v_j`), terminated by
/// `0 · γ(0)`. With `distinct`, `v_j = j`; else every `v_j = 1`. Crossed
/// with [`memo_chain_id`], the root is one covering left-full site and
/// every `(0, 0, v_j)` node an interior left-full site whose range is the
/// single leaf `v_j` — `k` consumption-sibling memo records in one fresh
/// scan, minima `v_j`. Distinct minima make every recorded difference
/// nonzero; the shared twin's differences are all zero (the unstored
/// case), so the pair separates per-record bookkeeping from work that
/// scales with the differences the records carry. Normal form holds
/// everywhere: every node's subtree minimum is its left leaf's 0, and the
/// only leaf pairs are `(0, v_j)` with `v_j ≥ 1`.
///
/// # Panics
///
/// Panics if `k == 0`.
pub fn memo_chain(k: usize, distinct: bool) -> Packed {
    assert!(k >= 1, "the memo chain needs at least one interior site");
    let mut bits = Bits::with_capacity(14 * k + 9);
    bits.push(true); // the root: the covering site's node
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the covering site's collapsed left leaf
    for j in 1..=k {
        bits.push(true); // spine node
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(true); // the interior site's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf(&mut bits, 0); // its collapsed left leaf
        ev_leaf(&mut bits, if distinct { j as u64 } else { 1 }); // its range: one leaf, the site minimum
    }
    ev_leaf(&mut bits, 0); // the spine terminal
    Packed::from_bits(bits)
}

/// The memo-chain id over [`memo_chain`]: `(1, ·)` at the root and at
/// every interior site, `10k + 8` bits.
///
/// Layout: the root tag `11` with full left terminal `00`, then per level
/// the spine tag `11`, the site id `(1, (1, 0))` (`11 · 00 · 10 · 00`),
/// terminated by `(1, 0)` (`10 · 00`). Normal form: no `(1, 1)` node —
/// every full child's sibling is internal or absent.
///
/// # Panics
///
/// Panics if `k == 0`.
pub fn memo_chain_id(k: usize) -> Packed {
    assert!(k >= 1, "the memo-chain id needs at least one interior site");
    let mut bits = Bits::with_capacity(10 * k + 8);
    bits.push(true); // the root: full left child ...
    bits.push(true); // ... over the spine
    bits.push(false); // the full left terminal
    bits.push(false);
    for _ in 0..k {
        bits.push(true); // spine tag: the site id, then the spine
        bits.push(true);
        bits.push(true); // the site id: full left child ...
        bits.push(true); // ... and an internal right
        bits.push(false); // the full left terminal
        bits.push(false);
        bits.push(true); // the site's range id: `(1, 0)`
        bits.push(false);
        bits.push(false);
        bits.push(false);
    }
    bits.push(true); // the spine terminus: `(1, 0)`
    bits.push(false);
    bits.push(false);
    bits.push(false);
    Packed::from_bits(bits)
}

/// The memo-comb event `B(d)`: `d` alternating levels of a single-leaf
/// site and a covering site, `~(18d + 2·γlen(d))` bits.
///
/// Layout: the root `1 · γ(0)` with left leaf `0 · γ(0)`, then per level
/// `i = 1..=d`: `1 · γ(0)` (the covering site `X_{i+1}`'s range root)
/// over `1 · γ(0) · 0 · γ(0) · 0 · γ(i)` (the single-leaf site `A_i`,
/// minimum `i`) and `1 · γ(0) · 0 · γ(0)` (the next covering site's
/// node), terminated by the leaf `0 · γ(d + 1)`. Crossed with
/// [`memo_comb_id`], one fresh scan records `2d + 1` sites whose ranges
/// interleave shallow (`A_i`, closing early) with covering (`X_i`,
/// closing late): recording order runs `A_1..A_d` then `X_{d+1}..X_1`
/// while the walk consumes `X_1, A_1, X_2, A_2, …` — every consecutive
/// consumption is Θ(d) apart in recording order, with ascending site
/// minima (`m(A_i) = m(X_i) = i`, the tail `d + 1`) keeping ~2d recorded
/// differences nonzero. Any resolution that walks recorded differences
/// between consecutively consumed sites — against the enclosing site or
/// the previously consumed one alike — re-reads Θ(d) of them per site;
/// per-site records anchored to the walk's own live state read O(1).
/// Normal form: every node's subtree minimum is 0 via its zero leaves,
/// and no equal leaf pair exists.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn memo_comb(d: usize) -> Packed {
    assert!(d >= 1, "the memo comb needs at least one level");
    let mut bits = Bits::with_capacity(20 * d + 24);
    bits.push(true); // the root: the outermost covering site's node
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // its collapsed left leaf
    for i in 1..=d {
        bits.push(true); // the covering range's root
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(true); // the single-leaf site's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf(&mut bits, 0); // its collapsed left leaf
        ev_leaf(&mut bits, i as u64); // its range: minimum `i`
        bits.push(true); // the next covering site's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf(&mut bits, 0); // its collapsed left leaf
    }
    ev_leaf(&mut bits, d as u64 + 1); // the innermost range: one leaf
    Packed::from_bits(bits)
}

/// The memo-comb id over [`memo_comb`]: a covering `(1, ·)` site per
/// level interleaved with the single-leaf sites' `(1, (1, 0))`,
/// `14d + 12` bits.
///
/// Layout: the root tag `11 · 00`, then per level `11` (the covering
/// range's node) · `11 · 00 · 10 · 00` (the single-leaf site) ·
/// `11 · 00` (the next covering site), terminated by `10 · 00`. Normal
/// form: no `(1, 1)` node.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn memo_comb_id(d: usize) -> Packed {
    assert!(d >= 1, "the memo-comb id needs at least one level");
    let mut bits = Bits::with_capacity(14 * d + 12);
    bits.push(true); // the root: full left child over the comb
    bits.push(true);
    bits.push(false); // the full left terminal
    bits.push(false);
    for _ in 0..d {
        bits.push(true); // the covering range's node
        bits.push(true);
        bits.push(true); // the single-leaf site: full left ...
        bits.push(true); // ... over its one-leaf range
        bits.push(false); // the full left terminal
        bits.push(false);
        bits.push(true); // the range id: `(1, 0)`
        bits.push(false);
        bits.push(false);
        bits.push(false);
        bits.push(true); // the next covering site: full left ...
        bits.push(true); // ... over the rest
        bits.push(false); // the full left terminal
        bits.push(false);
    }
    bits.push(true); // the innermost range id: `(1, 0)`
    bits.push(false);
    bits.push(false);
    bits.push(false);
    Packed::from_bits(bits)
}

/// The memo fan-out event `F(k, b)`: the memo-chain skeleton with one
/// `2^b − 1` minimum shared by all `k` sites over the covering site's
/// zero floor, `~(13k + 2kb + 9)` bits.
///
/// Layout: [`memo_chain`]'s skeleton with every site's range leaf at
/// `2^b − 1` and its collapsed left leaf at `2^b − 2` — the stream
/// climbs to the wide plateau once and steps by units across all `k`
/// sites, so the input pays the width exactly once (unlike
/// [`memo_oscillating`], whose input re-pays it per site). Crossed
/// with [`memo_chain_id`], the sites all share the wide minimum while
/// the covering site's own minimum is the zero terminal: the sibling
/// links are all zero (unstored) and exactly one ledger quantity (the
/// first site's deferred link against the covering minimum) carries
/// the width — paid once, independent of `k`. A recording discipline
/// that anchors each site to the covering floor instead materializes
/// `k` wide records; the pinned absolute touch ceiling is what such a
/// fan-out blows. Normal form: leaf pairs `(2^b − 2, 2^b − 1)`, every
/// subtree minimum 0 via the zero terminal under the root.
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
pub fn memo_fanout(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the memo fan-out needs at least one site");
    assert!(b >= 1, "the memo fan-out needs a nonzero magnitude");
    let wide = pow2_minus_1(b);
    let below = wide.clone() - &Base::from(1u8);
    let mut bits = Bits::with_capacity(13 * k + 4 * b + 9);
    bits.push(true); // the root: the covering site's node
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the covering site's collapsed left leaf
    for _ in 0..k {
        bits.push(true); // spine node
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(true); // the interior site's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf_wide(&mut bits, &below); // its collapsed left leaf, one below the plateau
        ev_leaf_wide(&mut bits, &wide); // its range: the shared wide minimum
    }
    ev_leaf(&mut bits, 0); // the spine terminal: the covering minimum
    Packed::from_bits(bits)
}

/// The oscillating-siblings event `O(k, b)`: the memo-chain skeleton
/// with site minima alternating `1` and `2^b − 1`, `~(13k + kb + 9)`
/// bits.
///
/// Layout: [`memo_chain`]'s exactly, with `v_j = 2^b − 1` for odd `j`
/// and `1` for even. Crossed with [`memo_chain_id`], every sibling
/// ledger link is wide — but each site's range leaf codes the same
/// width in the input, so the links are funded one-for-one by the
/// oscillation the input already paid for: the control for the
/// funding argument (flat touches per input byte, unlike the fan-out,
/// whose input pays its width once). Normal form: as
/// [`memo_chain`]'s.
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
pub fn memo_oscillating(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the oscillating siblings need at least one site");
    assert!(b >= 1, "the oscillating siblings need a nonzero magnitude");
    let wide = pow2_minus_1(b);
    let one = Base::from(1u8);
    let mut bits = Bits::with_capacity(13 * k + k * b + 9);
    bits.push(true); // the root: the covering site's node
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the covering site's collapsed left leaf
    for j in 0..k {
        bits.push(true); // spine node
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(true); // the interior site's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf(&mut bits, 0); // its collapsed left leaf
                               // the range: minima oscillating wide/narrow, funded by the
                               // input codes that store them
        ev_leaf_wide(&mut bits, if j % 2 == 0 { &wide } else { &one });
    }
    ev_leaf(&mut bits, 0); // the spine terminal
    Packed::from_bits(bits)
}

/// The memo-churn event `U(d)`: `d` sibling single-leaf sites, then a
/// descending run undercutting every open range minimum, `~(18d + 13)`
/// bits.
///
/// Layout: the root `1 · γ(0)` (the covering site) with left leaf
/// `0 · γ(0)`, then per level `i = 1..=d` the nested carrier
/// `1 · γ(0)` over the site `1 · γ(0) · 0 · γ(0) · 0 · γ(i + 1)`
/// (minimum `i + 1`), bottoming in [`staircase`]`(2d)`'s subtree
/// (preorder heights `2d, 2d − 1, …, 0`). Crossed with
/// [`memo_churn_id`], each site's record is live on the ledger head
/// while the run's every leaf undercuts every open range — `~2d`
/// full-penetration minimum drops with `d` recorded minima in flight.
/// One live head follows them at one fold per drop; a discipline that
/// keeps one live record per open level folds all `d` per drop
/// (quadratic), the refuted live-anchored followers' tombstone.
/// Normal form: leaf pairs `(0, i + 1)`, the run's unit-step descent,
/// and every subtree minimum 0 via the run's bottom.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn memo_churn(d: usize) -> Packed {
    assert!(d >= 1, "the memo churn needs at least one site");
    let mut bits = Bits::with_capacity(18 * d + 10 * (2 * d) + 20);
    bits.push(true); // the root: the covering site's node
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the covering site's collapsed left leaf
    for i in 1..=d {
        bits.push(true); // the nested carrier node
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(true); // the site's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf(&mut bits, 0); // its collapsed left leaf
        ev_leaf(&mut bits, i as u64 + 1); // its range: minimum i + 1
    }
    // The descending run: staircase(2d)'s subtree, heights 2d .. 0 —
    // above every site minimum at entry, below them all at exit.
    let run = 2 * d;
    bits.push(true); // the run's root: base 0
    codec::encode_int(&mut bits, &Base::from(0u8));
    for _ in 1..run {
        bits.push(true); // each deeper run node lifts by one
        codec::encode_int(&mut bits, &Base::from(1u8));
    }
    ev_leaf(&mut bits, 1); // bottom-left leaf: the run's top
    ev_leaf(&mut bits, 0); // bottom-right leaf: one step down
    for _ in 1..run {
        ev_leaf(&mut bits, 0); // each ancestor's right leaf: its floor
    }
    Packed::from_bits(bits)
}

/// The memo-churn id over [`memo_churn`]: a covering `(1, ·)` root,
/// per level the site's `(1, (1, 0))` under a carrier whose right arm
/// continues, and an absent id over the descending run, `14d + 6`
/// bits.
///
/// Layout: the root tag `11 · 00`, then per level `11` (the carrier)
/// · `11 · 00 · 10 · 00` (the site), with the last carrier's tag `10`
/// (right absent: the run is walked as `fill(0, e) = e`, its
/// emissions undercutting through every open frame). Normal form: no
/// `(1, 1)` node.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn memo_churn_id(d: usize) -> Packed {
    assert!(d >= 1, "the memo-churn id needs at least one site");
    let mut bits = Bits::with_capacity(14 * d + 6);
    bits.push(true); // the root: full left child over the carriers
    bits.push(true);
    bits.push(false); // the full left terminal
    bits.push(false);
    for i in 1..=d {
        bits.push(true); // the carrier: the site ...
        bits.push(i != d); // ... then deeper (absent at the last: the run)
        bits.push(true); // the site: full left child ...
        bits.push(true); // ... over its one-leaf range
        bits.push(false); // the full left terminal
        bits.push(false);
        bits.push(true); // the range id: `(1, 0)`
        bits.push(false);
        bits.push(false);
        bits.push(false);
    }
    Packed::from_bits(bits)
}

/// The descending-raises event `W(d)`: a floor realized high, then
/// `d` sibling sites whose minima step down from it, `~(13d + 26)`
/// bits.
///
/// Layout: the root `1 · γ(0)` (the covering site) with left leaf
/// `0 · γ(0)`, then `1 · γ(0)` whose left leaf `0 · γ(d + 2)` arms
/// the frame high before any site, over the [`memo_chain`]-style
/// spine with `v_j = d + 2 − j` — so every site's raise lands BELOW
/// the frame's minimum at its own consume, and each consume's arm
/// moves the tracked minimum the ledger relation must survive.
/// The one family whose raises exercise the decide-then-emit
/// ordering: a relation read after the raise emission is stale by
/// exactly the arm's delta, and the oracle differential catches the
/// wrong values. Normal form: leaf pairs `(0, d + 2 − j)` with
/// `j ≤ d`, every subtree minimum 0 via the zero terminal.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn descending_raises(d: usize) -> Packed {
    assert!(d >= 1, "the descending raises need at least one site");
    let mut bits = Bits::with_capacity(13 * d + 30);
    bits.push(true); // the root: the covering site's node
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the covering site's collapsed left leaf
    bits.push(true); // the floor carrier
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, d as u64 + 2); // the floor: armed before any site
    for j in 1..=d {
        bits.push(true); // spine node
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(true); // the interior site's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf(&mut bits, 0); // its collapsed left leaf
        ev_leaf(&mut bits, (d as u64 + 2) - j as u64); // its range: below the floor so far
    }
    ev_leaf(&mut bits, 0); // the spine terminal
    Packed::from_bits(bits)
}

/// The descending-raises id over [`descending_raises`]: the covering
/// `(1, ·)` root, an absent left over the floor leaf, then the
/// memo-chain site ids, `10d + 10` bits.
///
/// Layout: the root tag `11 · 00`, the floor carrier's `01` (left
/// absent: the floor leaf stays), then per site `11` (spine) ·
/// `11 · 00 · 10 · 00`, terminated by `(1, 0)`. Normal form: no
/// `(1, 1)` node.
///
/// # Panics
///
/// Panics if `d == 0`.
pub fn descending_raises_id(d: usize) -> Packed {
    assert!(d >= 1, "the descending-raises id needs at least one site");
    let mut bits = Bits::with_capacity(10 * d + 10);
    bits.push(true); // the root: full left child over the rest
    bits.push(true);
    bits.push(false); // the full left terminal
    bits.push(false);
    bits.push(false); // the floor carrier: left absent ...
    bits.push(true); // ... spine continues right
    for _ in 0..d {
        bits.push(true); // spine tag: the site id, then the spine
        bits.push(true);
        bits.push(true); // the site id: full left child ...
        bits.push(true); // ... and an internal right
        bits.push(false); // the full left terminal
        bits.push(false);
        bits.push(true); // the site's range id: `(1, 0)`
        bits.push(false);
        bits.push(false);
        bits.push(false);
    }
    bits.push(true); // the spine terminus: `(1, 0)`
    bits.push(false);
    bits.push(false);
    bits.push(false);
    Packed::from_bits(bits)
}

/// The reveal-comb event `R(k, b)`: one covering site over a
/// left-leaning comb of `k` sibling sites sharing one wide minimum
/// `2^b` above a zero floor, `~(k(4b + 8) + 6)` bits.
///
/// Layout: the root `1 · γ(0)` (the covering site) with left leaf
/// `0 · γ(0)`, then `k` comb nodes `1 · γ(0)` leaning left
/// (`a_i = node(0, a_{i−1}, site_i)`), the floor `0 · γ(0)` at the
/// deepest left, then per site `1 · γ(0) · 0 · γ(2^b − 1) · 0 · γ(2^b)`
/// (leaves one apart at the shared wide plateau). The input pays the
/// width once — the stream climbs to the plateau at the first site and
/// steps by units after — and every site's fill collapses to the equal
/// pair's leaf, so the output is unit deltas too. Crossed with
/// [`reveal_comb_id`], each site is a left-full pre-scan site whose
/// consume arms the tracked minimum `2^b` above the floor, and the
/// left-leaning spine closes the site's node frame back into the
/// 0-floor frame between consecutive consumes: the width-`b` boundary
/// difference is minted at every consume and popped at every close —
/// per-object-legal moves circulating one width with no input delta,
/// no output code, and no undercut descent funding any hop. Normal
/// form: no equal leaf pair exists (site pairs are `(2^b − 1, 2^b)`),
/// and every comb node's subtree minimum is 0 via the floor.
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
pub fn reveal_comb(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the reveal comb needs at least one site");
    assert!(b >= 1, "the reveal comb needs a nonzero magnitude");
    let wide = pow2(b);
    let below = pow2_minus_1(b);
    let mut bits = Bits::with_capacity(k * (4 * b + 8) + 6);
    bits.push(true); // the root: the covering site's node
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the covering site's collapsed left leaf
    for _ in 0..k {
        bits.push(true); // comb node a_i, i = k..1
        codec::encode_int(&mut bits, &Base::ZERO);
    }
    ev_leaf(&mut bits, 0); // the floor: a_1's left child
    for _ in 0..k {
        bits.push(true); // site_i's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf_wide(&mut bits, &below); // its collapsed left leaf: 2^b − 1
        ev_leaf_wide(&mut bits, &wide); // its range: the shared wide minimum
    }
    Packed::from_bits(bits)
}

/// [`reveal_comb`] with the floor raised to `2^b − 2`: identical site
/// forest, identical close-reveal cycle, consume-time gap 2,
/// `~(k(4b + 8) + 2b + 4)` bits.
///
/// Layout: [`reveal_comb`]'s exactly, with the floor leaf at
/// `0 · γ(2^b − 2)`. The tracked minimum at every site consume sits 2
/// below the site's minimum instead of `2^b` below, so the boundary
/// difference the cycle circulates is O(1) wide: the control that
/// separates the wide *gap* (the cost driver) from the forest shape
/// and the deferral cycle (shared with the red family). Normal form:
/// as [`reveal_comb`]'s, the floor now one below the site pairs.
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
pub fn reveal_comb_hifloor(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the reveal comb needs at least one site");
    assert!(b >= 1, "the reveal comb needs a nonzero magnitude");
    let wide = pow2(b);
    let below = pow2_minus_1(b);
    let floor = wide.clone() - &Base::from(2u8);
    let mut bits = Bits::with_capacity(k * (4 * b + 8) + 2 * b + 4);
    bits.push(true); // the root: the covering site's node
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the covering site's collapsed left leaf
    for _ in 0..k {
        bits.push(true); // comb node a_i, i = k..1
        codec::encode_int(&mut bits, &Base::ZERO);
    }
    ev_leaf_wide(&mut bits, &floor); // the raised floor: 2^b − 2
    for _ in 0..k {
        bits.push(true); // site_i's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf_wide(&mut bits, &below); // its collapsed left leaf: 2^b − 1
        ev_leaf_wide(&mut bits, &wide); // its range: the shared wide minimum
    }
    Packed::from_bits(bits)
}

/// The reveal-comb id over [`reveal_comb`]: the covering `(1, ·)`
/// root over per-comb-level `(b_{i−1}, site)` tags with the site ids
/// `(1, (1, 0))`, `10k + 4` bits.
///
/// Layout: the root tag `11 · 00`, then `k − 1` comb tags `11`
/// (deeper comb left, site right), the deepest comb tag `01` (left
/// absent: the floor stays), then per site `11 · 00 · 10 · 00` — the
/// site blocks trail the comb tags because each site is its comb
/// node's *right* child and the comb leans left. Normal form: no
/// `(1, 1)` node.
///
/// # Panics
///
/// Panics if `k == 0`.
pub fn reveal_comb_id(k: usize) -> Packed {
    assert!(k >= 1, "the reveal-comb id needs at least one site");
    let mut bits = Bits::with_capacity(10 * k + 4);
    bits.push(true); // the root: full left child ...
    bits.push(true); // ... over the comb
    bits.push(false); // the full left terminal
    bits.push(false);
    for _ in 1..k {
        bits.push(true); // b_i: the deeper comb left ...
        bits.push(true); // ... and site_i right
    }
    bits.push(false); // b_1: left absent (the floor stays) ...
    bits.push(true); // ... site_1 right
    for _ in 0..k {
        bits.push(true); // the site id: full left child ...
        bits.push(true); // ... and an internal right
        bits.push(false); // the full left terminal
        bits.push(false);
        bits.push(true); // the site's range id: `(1, 0)`
        bits.push(false);
        bits.push(false);
        bits.push(false);
    }
    Packed::from_bits(bits)
}

/// The pure-comb event `L(k, b)`: [`reveal_comb`]'s left-leaning comb
/// with a bare `2^b` leaf per level and NO covering site,
/// `~(k(2b + 4) + 2)` bits.
///
/// Layout: `k` comb nodes `1 · γ(0)` leaning left, the floor
/// `0 · γ(0)`, then `k` leaves `0 · γ(2^b)` (each comb node's right
/// child). Crossed with [`pure_comb_id`], no left-full site exists
/// anywhere — no memo, no pre-scan, no site consume: each wide leaf is
/// walked in its own leaf-under-internal-id frame, whose first
/// emission arms it `2^b` above the floor and whose close pops the
/// width-`b` boundary difference back — the base watermark stack's own
/// arm-move + close-pop cycle, isolated from the pre-scan's frame
/// ledger. Normal form: every comb node's subtree minimum is 0 via
/// the floor, and no two sibling leaves are equal (`2^b` pairs with an
/// internal node or the floor).
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
pub fn pure_comb(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the pure comb needs at least one level");
    assert!(b >= 1, "the pure comb needs a nonzero magnitude");
    let wide = pow2(b);
    let mut bits = Bits::with_capacity(k * (2 * b + 4) + 2);
    for _ in 0..k {
        bits.push(true); // comb node a_i, i = k..1
        codec::encode_int(&mut bits, &Base::ZERO);
    }
    ev_leaf(&mut bits, 0); // the floor: a_1's left child
    for _ in 0..k {
        ev_leaf_wide(&mut bits, &wide); // a_i's right leaf: 2^b
    }
    Packed::from_bits(bits)
}

/// The pure-comb id over [`pure_comb`]: per-comb-level
/// `(b_{i−1}, (1, 0))` tags, `6k` bits.
///
/// Layout: `k − 1` comb tags `11` (deeper comb left, the leaf's id
/// right), the deepest comb tag `01` (left absent: the floor stays),
/// then `k` × `10 · 00` — each level's `(1, 0)` node id over its wide
/// leaf, the leaf-under-internal-id frame shape. Normal form: no
/// `(1, 1)` node.
///
/// # Panics
///
/// Panics if `k == 0`.
pub fn pure_comb_id(k: usize) -> Packed {
    assert!(k >= 1, "the pure-comb id needs at least one level");
    let mut bits = Bits::with_capacity(6 * k);
    for _ in 1..k {
        bits.push(true); // b_i: the deeper comb left ...
        bits.push(true); // ... and the leaf's id right
    }
    bits.push(false); // b_1: left absent (the floor stays) ...
    bits.push(true); // ... the leaf's id right
    for _ in 0..k {
        bits.push(true); // the leaf's id: `(1, 0)` ...
        bits.push(false);
        bits.push(false); // ... whose left child is the terminal
        bits.push(false);
    }
    Packed::from_bits(bits)
}

/// The ascending cliff `A(k, b)`: a right spine of `k` ascending wide
/// left leaves `2^b + i` over a terminal 0-cliff, `k(2b + 4) + 2` bits.
///
/// Layout: `k` spine nodes `1 · γ(0)`, each with left leaf
/// `0 · γ(2^b + i)` (`i = 1..=k`, ascending inward), the deepest
/// node's right child the cliff `0 · γ(0)`. Crossed with
/// [`ascend_cliff_id`], each ascending unit step arms its own node's
/// frame one above the enclosing frame's minimum — `k − 1` nonzero
/// unit boundary differences with no zero runs anywhere — and the
/// cliff's single wide undercut (residue `2^b + k`) then propagates
/// through all of them: the family whose cascade prices the fold
/// *direction* of every hop, one wide residue against `k − 1` narrow
/// dying differences. The version's stored skyline stream pays the
/// width in O(1) codes (the first climb and the terminal drop) and
/// unit deltas between, so the input is Θ(k + b) and the tick's
/// output is the input with the cliff grown to `(0, 1, 0)` — Θ(k + b)
/// too, so a residue-width fold per hop survives the input+output
/// denominator. Normal form: every spine node's subtree minimum is 0
/// via the cliff, and no two sibling leaves exist (each wide leaf
/// pairs with an internal node; the deepest pair is `(2^b + k, 0)`).
///
/// # Panics
///
/// Panics if `k == 0`, `b == 0`, or `k + 2 > 2^b` (the ascent must
/// stay inside the width-`b` gamma-code band the closed form counts).
pub fn ascend_cliff(k: usize, b: usize) -> Packed {
    ascend_spine(k, b, true)
}

/// [`ascend_cliff`] with every wide leaf leveled at `2^b + 1`:
/// identical spine, identical cliff undercut, all boundary
/// differences zero, `k(2b + 4) + 2` bits.
///
/// Layout: [`ascend_cliff`]'s exactly, `i` pinned to 1. Every frame
/// arms at the shared minimum, so the difference stack is one
/// compressed zero run and the cliff's wide undercut passes it whole
/// in O(1): the control separating the cascade's *hop count* (the
/// cost driver under a per-hop width fold) from the spine shape, the
/// arming schedule, and the undercut itself, all shared with the
/// ascending family. Normal form: as [`ascend_cliff`]'s (each wide
/// leaf pairs with an internal node; the deepest pair is
/// `(2^b + 1, 0)`).
///
/// # Panics
///
/// Panics if `k == 0`, `b == 0`, or `k + 2 > 2^b`.
pub fn ascend_cliff_plateau(k: usize, b: usize) -> Packed {
    ascend_spine(k, b, false)
}

/// The shared ascending-cliff layout: ascending leaves or the
/// leveled control.
fn ascend_spine(k: usize, b: usize, ascend: bool) -> Packed {
    assert!(k >= 1, "the ascending cliff needs at least one spine node");
    assert!(b >= 1, "the ascending cliff needs a nonzero magnitude");
    // Every leaf's gamma code must stay 2b + 1 bits: γ(n) codes
    // m = n + 1, so the deepest leaf needs 2^b + k + 1 < 2^(b+1).
    assert!(
        b >= usize::BITS as usize || (k + 2) >> b == 0,
        "the ascent must stay inside the width-b code band"
    );
    let wide = pow2(b);
    let mut bits = Bits::with_capacity(k * (2 * b + 4) + 2);
    for i in 1..=k {
        bits.push(true); // spine node S_i, i = 1..=k
        codec::encode_int(&mut bits, &Base::ZERO);
        let step = if ascend { i as u64 } else { 1 };
        ev_leaf_wide(&mut bits, &(&wide + step)); // its wide left leaf
    }
    ev_leaf(&mut bits, 0); // the cliff: S_k's right child
    Packed::from_bits(bits)
}

/// The ascending-cliff id over [`ascend_cliff`]: a right-descent
/// `(0, ·)` chain bottoming in `(1, 0)` over the cliff, `2k + 4` bits.
///
/// Layout: `k` tags `01` (left absent — the wide leaves stay), then
/// `10 · 00` — the `(1, 0)` node over the cliff, whose owned left
/// half makes the cliff the tick's one grow site. Normal form: no
/// `(1, 1)` node.
///
/// # Panics
///
/// Panics if `k == 0`.
pub fn ascend_cliff_id(k: usize) -> Packed {
    assert!(
        k >= 1,
        "the ascending-cliff id needs at least one spine node"
    );
    let mut bits = Bits::with_capacity(2 * k + 4);
    for _ in 0..k {
        bits.push(false); // S_i's tag: left absent (the wide leaf stays) ...
        bits.push(true); // ... right present (the descent continues)
    }
    bits.push(true); // the cliff's id: `(1, 0)` ...
    bits.push(false);
    bits.push(false); // ... whose left child is the terminal
    bits.push(false);
    Packed::from_bits(bits)
}

/// The base `2^b − 1`, whose gamma code is `0^b · 1 · 0^b`.
fn pow2_minus_1(b: usize) -> Base {
    pow2(b) - &Base::from(1u8)
}

/// The base `2^b`.
fn pow2(b: usize) -> Base {
    let b = u32::try_from(b).expect("magnitude bit count fits u32");
    Base::from(1u8) << b
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
/// hashing; a widening left shift records its output width, operand plus
/// shifted-in limbs, so a shift-and-discard loop cannot read near-zero)
/// plus one value-width record
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

/// The packed-stream bits scanned and written since the last
/// [`reset_scan_bits`].
///
/// The deterministic stand-in for traversal work over the packed forms,
/// which every other meter can miss at once: an id-tree fold allocates
/// little (no heap delta), loops rather than recurses (no segments), and
/// does no `Base` arithmetic (no limb operations) — the work is *reading
/// and writing stream bits*, and this counter records exactly those, at
/// the packed-stream primitives (id tag reads and skip steps, id-builder
/// bit writes and splice lengths, event topology cursor advances and gamma
/// code-skips, every sequential decoder/validator bit read). Unit: bits.
/// Process-global, same isolation requirement as [`stack_segments`]; only
/// compiled under the `scan-meter` feature, which adds the counting to the
/// primitives themselves.
#[cfg(feature = "scan-meter")]
pub fn scan_bits() -> u64 {
    crate::codec::scan::scan_bits()
}

/// Reset the scanned-bits counter behind [`scan_bits`] to zero.
#[cfg(feature = "scan-meter")]
pub fn reset_scan_bits() {
    crate::codec::scan::reset()
}

#[cfg(test)]
mod tests;
