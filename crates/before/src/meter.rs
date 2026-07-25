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
