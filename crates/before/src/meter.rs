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
//! part of a production build. (The proptest strategies over *arbitrary*
//! inputs are a different instrument and live in the test-only
//! `testing::generators` module; the shapes here are hand-derived
//! worst cases.)
//!
//! A shape lands in one of two enforcement homes, and most take only one:
//! every shape gets its envelope rows in `tests/meter.rs` — the enforced
//! per-operation record — and a shape additionally earns a column on the
//! amplification board ([`board`](crate::meter::board)) only when it is a whole-surface
//! adversary rather than a kernel-seam probe (the criterion, and the
//! luck-proof touch list, sit on the board's `FAMILIES` roster).
//!
//! Every generator output is strict normal form: it round-trips through
//! [`Party::decode`](crate::Party::decode)/[`Version::decode`](crate::Version::decode)
//! and re-encodes byte-identically,
//! and its exact bit length is a closed formula in the parameters (pinned by
//! this module's tests). Normal form is also the one shaping constraint —
//! equal sibling leaves collapse, so a plateau is never spelled as an
//! equal leaf pair: the shapes spell one as unit-apart leaf values
//! ([`reveal_comb`](crate::meter::reveal_comb)) or as bare leaves under internal nodes
//! ([`pure_comb`](crate::meter::pure_comb)). Event shapes are built in the generators'
//! construction language — per node, a flag bit (`1` internal, `0` leaf,
//! this language's own convention) plus the Elias-gamma code of its base
//! (`gamma(n)` codes `m = n + 1`) — which the skyline transcoder
//! (the [`skyline`](crate::meter::skyline) module's `encode_bits`)
//! turns into the stored wire
//! coding. Id
//! shapes are the crate codec directly: a 2-bit child-presence tag per
//! node, absent children occupying no bits.
//!
//! Designing a new shape, two decided axes are worth finding before any
//! bits: whether the input pays the adversarial width once or per site —
//! the funding argument, argued at [`memo_fanout`](crate::meter::memo_fanout) versus
//! [`memo_oscillating`](crate::meter::memo_oscillating) — and, for pair shapes, whether the pair is two
//! packed streams ([`jump_pair`](crate::meter::jump_pair)) or organically built [`Version`]s
//! ([`concurrent_pair`](crate::meter::concurrent_pair), which argues the choice).

pub mod board;
pub mod tier2;

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
/// The node-count and tree-depth maximizer; drives every per-node and
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

/// The freeze-position exponent of the wide drop in
/// [`freeze_position`].
///
/// `2^288` is a ten-base-2^32-digit value, so a block's drift exceeds
/// the following unit code's one digit by more than the query folds'
/// eight-digit freeze allowance, and every block fires one freeze.
const FREEZE_POSITION_DROP_BITS: usize = 288;

/// The freeze-position spine `FP(k)`: a right spine of `2k` descending
/// wide left leaves whose consecutive drops alternate `2^288` and one,
/// over a terminal 0 leaf.
///
/// Exactly `4k(L + 2) + 2` bits for the one shared leaf-width band
/// `L = 289 + bitlen(k)`.
///
/// Layout: `2k` spine nodes `1 · γ(0)` leaning right, node `j`'s left
/// leaf the `j`-th value of the descent from `2^L + k(2^288 + 1)`
/// (alternately dropping `2^288` and `1`), the deepest node's right
/// child the terminal `0 · γ(0)`. Each block's wide drop re-arms live
/// drift over the query folds' freeze allowance and the following unit
/// code fires the freeze, so a query fold freezes `Θ(k)` times, at
/// stream positions whose written span grows with every block — the
/// many-freezes genre: an accounting that reads an absolute position
/// (or re-reads any whole-history state) per freeze goes quadratic
/// here, while every committed comb fires O(1) freezes. The descent
/// consumes `k(2^288 + 1) < 2^L`, so every leaf shares the one
/// `(L + 1)`-bit width and the size formula is exact.
/// `min_ticks(FP(k))` is the leaf sum `2k·2^L + k(k−1)(2^288 + 1) + k`
/// (every node minimum is 0 via the terminal leaf). Normal form:
/// values strictly descend (no equal siblings), every base is 0, and
/// every subtree minimum is 0.
///
/// # Panics
///
/// Panics if `k == 0`.
pub fn freeze_position(k: usize) -> Packed {
    assert!(k >= 1, "the freeze-position spine needs at least one block");
    let band = FREEZE_POSITION_DROP_BITS + 1 + bitlen(k);
    let wide = suanpan::UBig::ONE << FREEZE_POSITION_DROP_BITS;
    let unit = suanpan::UBig::ONE;
    let descent = (&wide + &unit) * suanpan::UBig::from(k as u64);
    let mut value = (suanpan::UBig::ONE << band) + descent;
    let mut bits = Bits::with_capacity(4 * k * (band + 2) + 2);
    for _ in 0..k {
        for drop in [&wide, &unit] {
            bits.push(true); // spine node: base 0, leaf left, spine right
            codec::encode_int(&mut bits, &Base::ZERO);
            value -= drop;
            ev_leaf_wide(&mut bits, &Base::from(value.clone()));
        }
    }
    ev_leaf(&mut bits, 0); // the terminal leaf: every ancestor's minimum
    Packed::from_bits(bits)
}

/// The bit length of `k` (`k >= 1`): the freeze-position band's
/// headroom exponent.
fn bitlen(k: usize) -> usize {
    (usize::BITS - k.leading_zeros()) as usize
}

/// The promotion re-arm arming exponent in [`promotion_rearm`].
///
/// `2^608` spans 20 base-2^32 digits: more than the query folds'
/// eight-digit freeze allowance above the settling drop's ten
/// ([`PROMOTION_REARM_SETTLE_BITS`]), so every block's second freeze
/// finds the parked component over-wide and promotes it.
const PROMOTION_REARM_ARM_BITS: usize = 608;

/// The promotion re-arm settling exponent in [`promotion_rearm`].
///
/// `2^288` spans 10 digits: wide enough that the following unit code
/// trips the freeze trigger (10 > 1 + 8), narrow enough that the parked
/// arming drift exceeds it by more than the allowance (20 > 10 + 8).
const PROMOTION_REARM_SETTLE_BITS: usize = 288;

/// Span-building spine levels per block in [`promotion_rearm`]: the
/// phase-1 run of `32p` levels puts a `Θ(p)`-digit floor under the
/// consumed-mass span the blocks then re-arm across, at ~5 stored bits
/// per level.
const PROMOTION_REARM_LEVELS_PER_BLOCK: usize = 32;

/// The promotion re-arm spine `PR(p)`: `32p` span-building levels down
/// a right spine, then `p` four-node re-arm blocks, over a terminal 1
/// leaf.
///
/// Exactly `1972p + 4` bits. Layout: `32p` spine nodes `(0, 1, ·)` /
/// `(0, 0, ·)` alternating (base 0, leaf heights 1, 0, 1, 0, … — 10
/// bits per pair), then per block the node bases `2^608, 1, 2^288, 1`
/// on the 0-leaf shape (1,220 + 6 + 580 + 6 bits), closing in the
/// leaf `1` (4 bits). The prefix's ±1 oscillation never freezes while
/// its interval masses' depths grow the consumed span one digit per 32
/// levels, and its running range minima are all zero, so the min-ticks
/// web rides the whole prefix as one compressed zero run (an ascending
/// prefix would instead arm `Θ(p)` distinct nested minima — the
/// ascend-cliff heap genre, deliberately avoided: this family's
/// adversarial payload is the promotion schedule, not the web). Each
/// block's `2^608` climb re-arms parked drift over the query folds'
/// freeze allowance (the following unit fires the freeze that parks
/// it), and its `2^288` climb re-freezes at a drift the parked
/// component exceeds by more than the allowance — one promotion per
/// block, `Θ(p)` promotions at O(1) stored codes each, so any
/// promotion accounting that re-reads whole-history state per arming
/// goes quadratic here while the family's suffix masses compact to
/// O(1) balanced terms. Every stored code is a delta the fold must
/// consume, and `min_ticks(PR(p)) = Σ bases = 16p + p(2^608 + 2^288 +
/// 2) + 1` is the closed-form semantic leg. Normal form: every prefix
/// node reaches its subtree minimum 0 through a later prefix 0 leaf,
/// every block node's minimum is its own 0 leaf, and no sibling leaf
/// pair is equal.
///
/// # Panics
///
/// Panics if `p == 0`.
pub fn promotion_rearm(p: usize) -> Packed {
    assert!(
        p >= 1,
        "the promotion re-arm spine needs at least one block"
    );
    let arm = pow2(PROMOTION_REARM_ARM_BITS);
    let settle = pow2(PROMOTION_REARM_SETTLE_BITS);
    let zero = Base::ZERO;
    let one = Base::from(1u8);
    let mut bits = Bits::with_capacity(1972 * p + 4);
    for level in 0..PROMOTION_REARM_LEVELS_PER_BLOCK * p {
        bits.push(true); // span-builder node: alternating leaf left
        codec::encode_int(&mut bits, &zero);
        ev_leaf(&mut bits, u64::from(level % 2 == 0)); // 1, 0, 1, 0, …
    }
    for _ in 0..p {
        for base in [&arm, &one, &settle, &one] {
            bits.push(true); // block node: 0-leaf left, spine right
            codec::encode_int(&mut bits, base);
            ev_leaf(&mut bits, 0);
        }
    }
    ev_leaf(&mut bits, 1); // the terminal leaf: the last unit climb
    Packed::from_bits(bits)
}

/// The promotion re-arm mate `PRM(p)`: the small twin of
/// [`promotion_rearm`] — the same `36p`-node right-spine topology with
/// the 1, 0, 1, 0, … leaf alternation running the whole spine.
///
/// Exactly `180p + 4` bits, and `min_ticks(PRM(p)) = 18p + 1`.
/// Overlaid against `PR(p)` it is the two-operand re-arm genre: the
/// heights agree leaf for leaf along the whole span-building prefix
/// (the difference folds to zero, boundary by boundary), and every
/// block boundary folds a unit from this operand against the other's
/// wide climb — so the co-sweep's freezes and promotions fire at
/// boundaries where this operand's cheap codes set the funded width,
/// moving drift only the other operand's wide codes deposited. `PR(p)`
/// dominates it pointwise (equal on the prefix, `≥ 2^608` against
/// `≤ 1` in the blocks), so the pair measures collapse to exact rank
/// identities. Normal form: as [`promotion_rearm`]'s prefix, closing
/// in the unequal leaf pair `(0, 1)`.
///
/// # Panics
///
/// Panics if `p == 0`.
pub fn promotion_rearm_mate(p: usize) -> Packed {
    assert!(p >= 1, "the re-arm mate needs at least one block's worth");
    let zero = Base::ZERO;
    // The spine matches PR(p) node for node: 32p span-builder levels
    // plus the 4p block levels, the alternation running through both.
    let levels = (PROMOTION_REARM_LEVELS_PER_BLOCK + 4) * p;
    let mut bits = Bits::with_capacity(180 * p + 4);
    for level in 0..levels {
        bits.push(true); // spine node: alternating leaf left
        codec::encode_int(&mut bits, &zero);
        ev_leaf(&mut bits, u64::from(level % 2 == 0)); // 1, 0, 1, 0, …
    }
    ev_leaf(&mut bits, 1); // the terminal leaf: the unequal closer
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

/// Shared-spine levels per isolated position digit in [`jump_pair`].
///
/// Each right-descent turn sets one isolated bit of every absolute
/// position below it, and a 33-level stride keeps successive bits more
/// than a full base-2^32 digit apart, so the balanced signed-digit
/// compaction (which cancels only ones-runs) can never merge two of
/// them into one term: every absolute position inside the comb carries
/// `d` incompressible digits.
const JUMP_PAIR_DIGIT_STRIDE: usize = 33;

/// The two-operand jump comb `JP(k, m, d)`: a version pair whose
/// height difference crests `k` bits wide at every one of `m` comb
/// levels, deep under a spine that makes every absolute position
/// `d` digits dense.
///
/// Each operand is certified-linear alone; the shape exists only in
/// the two-operand composition.
///
/// Both operands share a descent spine of `33d` zero-base levels that
/// turns right every 33rd level (one 0-leaf consumed *before* the comb
/// per turn — the `d` isolated position bits the stride constant
/// derives) and left elsewhere (those 0-leaf siblings trail after), then
/// diverge in an `m`-level right-leaning comb at the spine's bottom.
/// Per comb level, over a quarter-interval *tooth* and a quarter-interval
/// *gap*:
///
/// - the **teeth operand** `A` stores a bare wide tooth leaf `2^k + 3`
///   and a gap pair `(1, 0)` — its skyline oscillates `0 ↔ 2^k + 3`, so
///   consecutive wide folds cancel adjacently and `A`'s own rank fold
///   never freezes (bounded oscillation, paid by its own codes);
/// - the **band operand** `B` rides a plateau: a two-leaf band
///   `(2^k + 2, 2^k + 1)` across the tooth and a `2^k + 1` gap leaf,
///   with the width hoisted once into the comb root's stored base — one
///   wide code in `B`'s whole stream, unit deltas after.
///
/// Their overlay interleaves them: `|h_a − h_b|` sits at 1–2 inside
/// every tooth and `~2^k` across every gap, so per level a wide crest
/// funded by `A`'s tooth code rides over cheap codes from the *other*
/// operand (`k ≥ 320` bits clears the freeze allowance, so the drift is
/// parked at `B`'s first cheap boundary — `2m` freezes per distance,
/// each fired by the operand that did not pay for the drift). The spine
/// makes every absolute position `d` incompressible digits while the
/// per-crest *segment* masses compact to O(1) digits: an accounting
/// that multiplies parked drift by absolute positions pays
/// `Θ(m · d · k)` limb work against a `Θ(m·k + d)`-bit input, and the
/// anchored-segment co-sweep (`version/skyline/query.rs`'s
/// pair-co-sweep section) settles each crest against its own segment
/// and stays linear — the separation the `skyline_flatness` band test
/// and the board cell hold. The **join** (the band shades every gap)
/// collapses to
/// unit steps around one climb, and either operand's own rank is flat:
/// both inputs are individually innocuous, and the shape exists only in
/// the two-operand composition.
///
/// Layout, shared spine (level `ℓ = 0..33d`): `1 · γ(0)`, with the
/// 0-leaf `0 · γ(0)` emitted before the descent at right turns
/// (`ℓ ≡ 32 mod 33`) and queued after it otherwise — 4 bits per level.
/// Comb level `i = 1..=m`, teeth operand: `1 · γ(0)` (spine `c_i`),
/// `1 · γ(0)` (the tooth/gap pair node), `0 · γ(2^k + 3)` (the tooth),
/// `1 · γ(0) · 0 · γ(1) · 0 · γ(0)` (the gap pair) — `2k + 14` bits.
/// Band operand: `1 · γ(2^k + 1)` at `c_1` (the hoisted plateau base,
/// `2k + 2` bits) and `1 · γ(0)` below, `1 · γ(0)` (the pair node),
/// `1 · γ(0) · 0 · γ(1) · 0 · γ(0)` (the band pair, relative), `0 · γ(0)`
/// (the gap leaf) — 14 bits per level plus the one wide root code. Both
/// end in the comb terminal `0 · γ(0)` and the trailing left-turn
/// siblings. Totals: `132d + m(2k + 14) + 2` bits (teeth) and
/// `132d + 14m + 2k + 2` bits (band). Normal form holds everywhere:
/// every spine and pair node has a 0-leaf or 0-min child in reach, the
/// band comb's plateau lift sits on `c_1`'s own base, and no two sibling
/// leaves are equal (`(1, 0)` pairs; every wide leaf pairs with an
/// internal node).
///
/// # Panics
///
/// Panics if `k < 3` (the closed form needs `γ(2^k + 3)` at `2k + 1`
/// bits), `m == 0`, or `d == 0`.
pub fn jump_pair(k: usize, m: usize, d: usize) -> (Packed, Packed) {
    (
        jump_pair_operand(k, m, d, false),
        jump_pair_operand(k, m, d, true),
    )
}

/// One [`jump_pair`] operand: the teeth stream, or the band stream with
/// `band`.
fn jump_pair_operand(k: usize, m: usize, d: usize, band: bool) -> Packed {
    assert!(k >= 3, "the jump pair needs a wide tooth magnitude");
    assert!(m >= 1, "the jump pair needs at least one comb level");
    assert!(d >= 1, "the jump pair needs at least one position digit");
    let depth = JUMP_PAIR_DIGIT_STRIDE * d;
    let tooth = &pow2(k) + 3u64;
    let plateau = &pow2(k) + 1u64;
    let zero = Base::ZERO;
    let mut bits = Bits::with_capacity(132 * d + m * (2 * k + 14) + 2);
    // The shared descent spine: right turns every 33rd level consume
    // their 0-leaf before the comb (the freeze-position bits), left
    // turns queue theirs after it.
    let mut trailing = 0usize;
    for level in 0..depth {
        bits.push(true); // spine node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
        if (level + 1) % JUMP_PAIR_DIGIT_STRIDE == 0 {
            ev_leaf(&mut bits, 0); // right turn: the leaf leads the descent
        } else {
            trailing += 1; // left turn: the leaf trails the whole subtree
        }
    }
    // The comb: c_i = node(pair_i, c_{i+1}), terminal leaf under c_m.
    for i in 0..m {
        bits.push(true); // comb spine node c_i
        codec::encode_int(
            &mut bits,
            // The band stream's one wide code: the plateau lift, hoisted
            // to the comb root by min-lifted normal form.
            if band && i == 0 { &plateau } else { &zero },
        );
        bits.push(true); // the tooth/gap pair node
        codec::encode_int(&mut bits, &Base::ZERO);
        if band {
            bits.push(true); // the band node across the tooth interval
            codec::encode_int(&mut bits, &Base::ZERO);
            ev_leaf(&mut bits, 1); // band leaf 2^k + 2, relative 1
            ev_leaf(&mut bits, 0); // band leaf 2^k + 1, relative 0
            ev_leaf(&mut bits, 0); // gap leaf 2^k + 1, relative 0
        } else {
            ev_leaf_wide(&mut bits, &tooth); // the tooth: 2^k + 3
            bits.push(true); // the gap pair node
            codec::encode_int(&mut bits, &Base::ZERO);
            ev_leaf(&mut bits, 1); // gap leaf 1
            ev_leaf(&mut bits, 0); // gap leaf 0
        }
    }
    ev_leaf(&mut bits, 0); // the comb terminal: c_m's right child
    for _ in 0..trailing {
        ev_leaf(&mut bits, 0); // the left turns' siblings, innermost first
    }
    Packed::from_bits(bits)
}

/// The concurrent pair `CP(n)`: two organically built versions over one
/// balanced fork of `n` parties, ticked so every adjacent region flips
/// which operand dominates — the emit side-switch population.
///
/// The seed party forks balanced to `n` single-leaf owners (leaf `i` of
/// the depth-`log2 n` fork tree). Each version joins `n` independent
/// per-party histories — party `i`'s empty version ticked to its target
/// alone, then all `n` merged through `join_all`'s balanced fold, so
/// construction is `O(n log n)` and every tick lands exactly one height
/// unit (an isolated history has no higher neighbor for the tick's fill
/// leg to lift toward). The targets make the winner alternate by leaf
/// parity with no two adjacent plateaus ever equal: leaf `i` reaches
/// `3 + (i mod 3)` ticks on the dominant side and `1 + (i mod 3)` on
/// the other, the dominant side even-`i` for the first version and
/// odd-`i` for the second. The join's plateau sequence is `3 + (i mod 3)` and
/// the meet's `1 + (i mod 3)` — each adjacent-distinct, so neither
/// emission ever collapses a boundary and **every one of the `n − 1`
/// overlay boundaries is a side switch, in the join and the meet
/// alike** (the corpus pairing `w = v + one seed tick` reaches at most
/// one). All heights are word-scale: the family prices the switch
/// machinery's density, not width. Every leaf's dominant and dominated
/// heights differ by exactly 2, so `distance = Σᵢ 2/n` — the integer
/// rank 2 at every `n`, which the generator's test pins as the semantic
/// witness that the schedule realized the heights it claims.
///
/// # Panics
///
/// Panics if `n` is not a power of two at least 2 (the balanced fork
/// and the parity schedule both need it).
pub fn concurrent_pair(n: usize) -> (crate::Version, crate::Version) {
    assert!(
        n >= 2 && n.is_power_of_two(),
        "the concurrent pair needs a power-of-two party count"
    );
    let mut parties = vec![crate::Party::seed()];
    while parties.len() < n {
        let mut next = Vec::with_capacity(parties.len() * 2);
        for mut p in parties {
            let q = p.fork();
            next.push(p);
            next.push(q);
        }
        parties = next;
    }
    let history = |p: &crate::Party, ticks: u64| {
        let mut h = crate::Version::new();
        for _ in 0..ticks {
            h.tick(p);
        }
        h
    };
    let mut v_parts = Vec::with_capacity(n);
    let mut w_parts = Vec::with_capacity(n);
    for (i, p) in parties.iter().enumerate() {
        let dominant = 3 + (i % 3) as u64;
        let other = 1 + (i % 3) as u64;
        let (v_ticks, w_ticks) = if i % 2 == 0 {
            (dominant, other)
        } else {
            (other, dominant)
        };
        // Independent per-party histories: each operand advances on
        // every party, the schedule alone decides who dominates where.
        v_parts.push(history(p, v_ticks));
        w_parts.push(history(p, w_ticks));
    }
    (
        crate::Version::join_all(v_parts),
        crate::Version::join_all(w_parts),
    )
}

/// The masked-comparison correlated triple `MT(k, n)`: a boundary comb, a
/// mask owning every other tooth, and a wide plateau — the three-stream
/// fused comparison's adversary, each operand benign alone.
///
/// Returns `(event, id, event)`: [`cliff_comb`]`(k, n)` (the masked
/// operand), [`scattered_id`]`(n / 2)` (the mask, whose owned fragments
/// sit exactly at the comb's even tooth positions), and a single-leaf
/// plateau at `2^k` (the unmasked right operand, `2k + 2` bits). The
/// correlation is the point — every operand is a certified-linear genre
/// by itself (the comb, the scattered id, a hugeleaf-class plateau), and
/// the heat exists only in the composition: comparing
/// `(comb / mask) ⋚ plateau` toggles ownership at every tooth boundary,
/// so the walk alternates between reading the difference `D = h_comb −
/// 2^k` inside owned teeth — a near-zero value spelled by cancelling
/// wide digits, oscillating across the `2^k` carry boundary behind 3-bit
/// stored deltas — and the zero-check `sign(h_plateau)` on unowned
/// intervals. An integrator that materializes either read pays `Θ(k)`
/// limb work per 3-bit code; the balanced signed-digit accumulator
/// answers both in amortized O(1) touches (the envelope rows and the
/// flatness band in `tests/meter.rs` hold it there). The verdict is
/// `Less` — the projected comb sits under the plateau everywhere and
/// strictly under it outside the mask — so the walk never exits early
/// and the measurement prices the whole overlay.
///
/// # Panics
///
/// Panics if `k == 0`, or `n` is not an even count of at least 2 (the
/// mask owns every other tooth).
pub fn mask_drift_triple(k: usize, n: usize) -> (Packed, Packed, Packed) {
    assert!(k >= 1, "the mask-drift triple needs a nonzero magnitude");
    assert!(
        n >= 2 && n.is_multiple_of(2),
        "the mask-drift triple needs an even tooth count"
    );
    let mut plateau = Bits::with_capacity(2 * k + 2);
    ev_leaf_wide(&mut plateau, &pow2(k));
    (
        cliff_comb(k, n),
        scattered_id(n / 2),
        Packed::from_bits(plateau),
    )
}

/// The masked-comparison correlated quadruple `MQ(k, n)`: two comb/mask
/// pairs whose ownership parities interleave — the four-stream fused
/// comparison's adversary, each operand benign alone.
///
/// Returns `((event₁, id₁), (event₂, id₂))`: the sparse comb (teeth at
/// odd levels only, plain zero leaves at even levels,
/// `(n/2)(2k + 14) + 2` bits) under [`scattered_id`]`(n / 2)` (owning
/// the even levels — exactly where its event is zero), against the full
/// [`cliff_comb`]`(k, n)` under the offset mask (owning the odd levels —
/// exactly where its event's teeth stand). Every tooth boundary is a
/// double mask toggle with the parities out of phase, so the walk
/// rotates through its ownership cases: even-level teeth read
/// `sign(h₁)` — the trichotomy's zero-check on a height that is zero
/// *semantically* but spelled by cancelling `2^k`-wide digits, each
/// odd tooth's climb and drop funded by its own wide codes — and
/// odd-level teeth read `sign(h₂)` mid-oscillation across the carry
/// boundary. The projected verdict is `Less` (view₁ is semantically
/// empty; view₂ keeps its teeth), so the walk never exits early. The
/// envelope rows and the flatness band in `tests/meter.rs` hold the
/// composition linear.
///
/// # Panics
///
/// Panics if `k == 0`, or `n` is not an even count of at least 2.
pub fn mask_drift_quadruple(k: usize, n: usize) -> ((Packed, Packed), (Packed, Packed)) {
    assert!(k >= 1, "the mask-drift quadruple needs a nonzero magnitude");
    assert!(
        n >= 2 && n.is_multiple_of(2),
        "the mask-drift quadruple needs an even tooth count"
    );
    (
        (sparse_cliff_comb(k, n), scattered_id(n / 2)),
        (cliff_comb(k, n), scattered_id_offset(n / 2)),
    )
}

/// The sparse boundary comb: [`cliff_comb`]'s spine with teeth at odd
/// levels only and a plain zero leaf at each even level,
/// `(n/2)(2k + 14) + 2` bits.
///
/// Layout per level pair: `"11" · "01"` (even level: spine node,
/// zero left leaf), then `"11" · "1" · gamma(2^k − 1) · "01" · "0010"`
/// (odd level: spine node and the comb's tooth), after all `n` levels
/// `"01"` (the terminal spine leaf). Normal form holds as the comb's:
/// every spine node's zero-base leaf child carries its subtree minimum,
/// and the only sibling leaf pairs are the teeth's `(0, 1)`.
fn sparse_cliff_comb(k: usize, n: usize) -> Packed {
    debug_assert!(k >= 1 && n >= 2 && n.is_multiple_of(2));
    let mut bits = Bits::with_capacity((n / 2) * (2 * k + 14) + 2);
    let tooth = pow2_minus_1(k);
    for level in 0..n {
        bits.push(true); // spine node flag
        codec::encode_int(&mut bits, &Base::ZERO); // gamma(0) = "1"
        if level % 2 == 1 {
            bits.push(true); // tooth node flag
            codec::encode_int(&mut bits, &tooth);
            ev_leaf(&mut bits, 0); // tooth's left leaf: value 2^k − 1
            ev_leaf(&mut bits, 1); // tooth's right leaf: value 2^k
        } else {
            ev_leaf(&mut bits, 0); // the even level's plain zero leaf
        }
    }
    ev_leaf(&mut bits, 0); // terminal spine leaf
    Packed::from_bits(bits)
}

/// The offset scattered id: [`scattered_id`]'s alternation shifted one
/// level down — a gap level, then an owned left subtree, repeated —
/// `6e + 4` bits.
///
/// Layout, repeated `e` times: `01` (a right-only gap level), `11` (both
/// children present), `00` (the owned left leaf); terminated by `01 · 00`
/// (a final gap level whose right child is the owned tip). Owns exactly
/// the *odd* levels' left subtrees of a right-leaning spine — the
/// complement, tooth for tooth, of [`scattered_id`]'s even-level
/// fragments. Normal form: no node has two fully-owned children (each
/// `11` node's right child is a gap node or the final gap level) and no
/// node has two absent children.
fn scattered_id_offset(e: usize) -> Packed {
    debug_assert!(e >= 1);
    let mut bits = Bits::with_capacity(6 * e + 4);
    for _ in 0..e {
        bits.push(false); // gap node: left child absent ...
        bits.push(true); // ... the spine continues right
        bits.push(true); // fragment node: left child present ...
        bits.push(true); // ... and the spine continues right
        bits.push(false); // the owned left leaf: terminal tag "00"
        bits.push(false);
    }
    bits.push(false); // a final gap level ...
    bits.push(true);
    bits.push(false); // ... whose right child is the owned tip
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
