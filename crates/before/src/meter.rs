//! Adversarial input generators and deterministic resource meters.
//!
//! Public under the `meter` feature so the metering test binaries can drive
//! them.
//!
//! This module is the measurement half of the crate's resource-proportionality
//! work: transient cost — peak heap, stack segments, big-integer limb work,
//! packed-stream scan work — as a function of packed input size, with no bound
//! on value magnitude, tree depth, or encoded size. The generators below build
//! the canonical packed encodings that maximize each cost against its input
//! size; the meters read the deterministic counters the envelopes are pinned
//! against. Public under the `meter` feature so the metering test binaries (and
//! benches) can reach it; never part of a production build. (The proptest
//! strategies over *arbitrary* inputs are a different instrument and live in
//! the test-only `testing::generators` module; the shapes here are hand-derived
//! worst cases.)
//!
//! The generators themselves are private: every instrument mints its shapes
//! through the family registry ([`registry`]), whose roster is the single
//! source of truth for adversarial families — the registry module doc states
//! the invariant and the compiler ties that hold it. A shape lands in one of
//! two enforcement homes, and most take only one: every shape gets its envelope
//! rows in `tests/meter.rs` — the enforced per-operation record — and a shape
//! additionally earns a column on the amplification board ([`board`]) only when
//! it is a whole-surface adversary rather than a kernel-seam probe (the
//! criterion, each family's coverage answer, and the luck-proof touch list sit
//! on the registry's [`FamilyId`](crate::meter::registry::FamilyId)).
//!
//! Every generator output is strict normal form: it round-trips through
//! [`Party::decode`](crate::Party::decode)/[`Version::decode`](crate::Version::decode)
//! and re-encodes byte-identically, and its exact bit length is a closed
//! formula in the parameters (pinned by this module's tests). Normal form is
//! also the one shaping constraint — equal sibling leaves collapse, so a
//! plateau is never spelled as an equal leaf pair: the shapes spell one as
//! unit-apart leaf values
//! ([`Shape::RevealComb`](crate::meter::registry::Shape::RevealComb)) or as
//! bare leaves under internal nodes
//! ([`Shape::PureComb`](crate::meter::registry::Shape::PureComb)). Event shapes
//! are built in the generators' construction language — per node, a flag bit
//! (`1` internal, `0` leaf, this language's own convention) plus the
//! Elias-gamma code of its base (`gamma(n)` codes `m = n + 1`) — which the
//! skyline transcoder (the [`skyline`] module's `encode_bits`) turns into the
//! stored wire coding. Id shapes are the crate codec directly: a 2-bit
//! child-presence tag per node, absent children occupying no bits.
//!
//! Designing a new shape, two decided axes are worth finding before any bits:
//! whether the input pays the adversarial width once or per site — the funding
//! argument, argued at
//! [`Shape::MemoFanout`](crate::meter::registry::Shape::MemoFanout) versus
//! [`Shape::MemoOscillating`](crate::meter::registry::Shape::MemoOscillating) —
//! and, for pair shapes, whether the pair is two packed streams
//! ([`Shape::JumpPair`](crate::meter::registry::Shape::JumpPair)) or
//! organically built [`Version`](crate::Version)s
//! ([`Shape::ConcurrentPair`](crate::meter::registry::Shape::ConcurrentPair),
//! which argues the choice).
//!
//! One construction convention for new families: when a family is a
//! *geometrically coupled pair* — two operands whose adversarial effect depends
//! on shared structure (aligned boundaries, mirrored spikes, lockstep walks) —
//! one generator builds and returns the pair
//! ([`Shape::ToothTail`](crate::meter::registry::Shape::ToothTail) is the form
//! of record), never two parallel generators whose coupling is maintained by
//! keeping their bodies in sync by hand. Existing paired generators keep their
//! committed shapes as-is: migrating them would churn pinned envelopes and
//! provenance for no behavioral gain; the convention binds new families.

pub mod board;
pub mod registry;
pub mod tier2;

/// The skyline transcoding codec, re-exported so the resource-envelope suite
/// can pin its validator's transient state and limb behavior.
pub use crate::version::skyline;

/// The pair-hull rung snapshot, re-exported beside its readers
/// ([`span_traffic`]/[`reset_span_traffic`]).
pub use crate::version::hull_traffic::SpanTraffic;

/// The watermark web's domination-decision snapshot, re-exported beside its
/// readers ([`emit_traffic`]/[`reset_emit_traffic`]).
pub use crate::version::skyline::web_traffic::EmitTraffic;

use crate::codec::{self, Base, BitsMut};

/// A generator's output: canonical packed bytes plus the exact bit length.
///
/// `bytes` is what `decode` accepts and `encode` reproduces
/// (marker-padded to a byte boundary); `bits` is the live bit length
/// before that padding, so tests can pin the closed-form size of each
/// shape.
#[derive(Debug, Clone)]
pub struct Packed {
    /// The canonical packed bytes, marker-padded to a byte boundary.
    pub bytes: Vec<u8>,
    /// The exact number of live bits in `bytes` before the padding.
    pub bits: usize,
}

impl Packed {
    /// Canonicalize a built bit stream: seal the marker padding, keeping
    /// the live length.
    fn from_bits(mut bits: BitsMut) -> Self {
        let len = bits.len();
        codec::seal_padding(&mut bits);
        Packed {
            bytes: bits.into_vec(),
            bits: len,
        }
    }

    /// The generator's live bits, borrowed.
    ///
    /// A borrowed bit view, sized for the instrument surface: the shape
    /// generators emit parameter-scale streams far below the view
    /// encoding's cap (`usize::MAX >> 3` bits), where the byte decode
    /// doors — which admit larger buffers on 32-bit targets — walk raw
    /// bytes instead.
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
fn ev_leaf(bits: &mut BitsMut, n: u64) {
    ev_leaf_wide(bits, &Base::from(n));
}

/// Append an event leaf with an arbitrary-width stored base: flag `0`, then
/// `gamma(base)`.
fn ev_leaf_wide(bits: &mut BitsMut, base: &Base) {
    bits.push(false);
    codec::encode_int(bits, base);
}

/// Append the dense event spine body: `d` zero-base internal nodes leaning
/// left, each with a 0-leaf right sibling, bottoming out in `(0, 0, 1)`.
///
/// Layout: `"11" × d` (internal flag + `gamma(0)`), `"01"` (bottom-left leaf
/// 0), `"0010"` (bottom-right leaf 1), `"01" × (d − 1)` (each ancestor's right
/// sibling). Exactly `4d + 4` bits for `2d + 1` nodes at depth `d` — the
/// densest shape normal form admits (~2 bits per node, depth ~n/4 for `n`
/// bits), maximizing node count and recursion depth simultaneously. Normal form
/// holds everywhere: each internal node's spine child has base 0, and the only
/// leaf pair is `(0, 1)`.
fn ev_spine(bits: &mut BitsMut, d: usize) {
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
fn dense(d: usize) -> Packed {
    assert!(d >= 1, "dense spine needs at least one internal node");
    let mut bits = BitsMut::with_capacity(4 * d + 4);
    ev_spine(&mut bits, d);
    Packed::from_bits(bits)
}

/// A root with base `2^b − 1` over `S(d)` and a 0-leaf: `2b + 4d + 8` bits.
///
/// Layout: `"1" · gamma(2^b − 1) · S(d) · "01"`, where `gamma(2^b − 1) = 0^b ·
/// 1 · 0^b` (`2b + 1` bits). Puts a `b`-bit magnitude on every root-to-node
/// path sum while keeping paths long — the shape that makes owned per-frame
/// path sums quadratic in the input.
///
/// # Panics
///
/// Panics if `b == 0` or `d == 0`.
fn bigroot(b: usize, d: usize) -> Packed {
    assert!(b >= 1, "bigroot needs a nonzero root magnitude");
    assert!(d >= 1, "bigroot needs a nonzero spine depth");
    let mut bits = BitsMut::with_capacity(2 * b + 4 * d + 8);
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
fn hugeleaf(b: usize) -> Packed {
    assert!(b >= 1, "hugeleaf needs a nonzero magnitude");
    let mut bits = BitsMut::with_capacity(2 * b + 2);
    bits.push(false); // leaf flag
    codec::encode_int(&mut bits, &pow2_minus_1(b));
    Packed::from_bits(bits)
}

/// The boundary comb `C(k, n)`: `n` cliff teeth, `n(2k + 10) + 2` bits.
///
/// A zero-base spine leaning right, each spine node's left child a *tooth*
/// `(2^k − 1, 0, 1)` — an internal node with base `2^k − 1` over leaves 0 and 1
/// — terminated by a leaf 0. Its preorder leaf values oscillate `2^k − 1 ↔
/// 2^k`: every consecutive-leaf difference is `±1` sitting exactly on the `2^k`
/// carry boundary, so any sweep that maintains a running leaf value (or a
/// running difference of leaf values) pays a full `k`-bit carry or borrow per
/// crossing. In this coding each tooth stores its own `gamma(2^k − 1)` — `2k +
/// 1` bits — so every crossing is paid for by a comparably-wide input code and
/// operations stay linear per input bit; a delta coding of the same tree stores
/// 3-bit `±1` codes per crossing instead, which is what makes this the
/// separating family for the leaf-delta representation question.
///
/// Layout per tooth: `"11"` (spine node, `gamma(0)`), `"1" · gamma(2^k − 1)`
/// (tooth node), `"01"` (leaf 0), `"0011"` (leaf 1); after all `n` teeth,
/// `"01"` (the terminal leaf 0). `2k + 10` bits per tooth plus 2, over `4n + 1`
/// nodes of which `2n + 1` are leaves. Normal form holds everywhere: each spine
/// node's right child has base 0, each tooth's left leaf has base 0, and the
/// only leaf pairs are `(0, 1)`.
///
/// # Panics
///
/// Panics if `k == 0` or `n == 0`.
fn cliff_comb(k: usize, n: usize) -> Packed {
    assert!(k >= 1, "cliff comb needs a nonzero tooth magnitude");
    assert!(n >= 1, "cliff comb needs at least one tooth");
    let mut bits = BitsMut::with_capacity(n * (2 * k + 10) + 2);
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

/// The jump comb `J(k, n)`: one low tooth, then `n − 1` cliff teeth, `(n −
/// 1)(2k + 10) + 14` bits.
///
/// The boundary comb with its first tooth lowered to `(1, 0, 1)`: preorder leaf
/// values run `1, 2`, jump to `2^k − 1`, then oscillate `2^k − 1 ↔ 2^k`. In a
/// delta coding the jump is the one wide leaf-to-leaf code, arriving mid-stream
/// with only 3-bit codes behind it — the stale-drift shape: a sweep that keeps
/// running height state must move the jump out of its cheap-delta path exactly
/// once, paid by the jump's own code, or pay the jump's width again on every
/// 3-bit delta that follows. The wide-tooth comb prices bounded wide
/// oscillation (state that must *stay* live); this family prices the eviction
/// (state that must *leave*), so together they pin a height split from both
/// sides.
///
/// Layout: one tooth `"11" · "1" · gamma(1) · "01" · "0010"` (12 bits), then `n
/// − 1` [`cliff_comb`] teeth at `2k + 10` bits, then the terminal `"01"`.
/// Normal form holds everywhere by the comb's own argument: every spine node's
/// right child has base 0, every tooth's left leaf has base 0, and the only
/// leaf pairs are `(0, 1)`.
///
/// # Panics
///
/// Panics if `k == 0` or `n < 2`: the jump needs a low tooth and at least one
/// cliff tooth to jump between.
fn jump_comb(k: usize, n: usize) -> Packed {
    assert!(k >= 1, "jump comb needs a nonzero cliff magnitude");
    assert!(n >= 2, "jump comb needs a low tooth and a cliff tooth");
    let mut bits = BitsMut::with_capacity((n - 1) * (2 * k + 10) + 14);
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
/// The boundary comb's wide-delta sibling: the same zero-base spine, each tooth
/// `(2^k − 2^w, 0, 2^w)` — an internal node with base `2^k − 2^w` over leaves 0
/// and `2^w` — terminated by a leaf 0. Its preorder leaf values oscillate `2^k
/// − 2^w ↔ 2^k`: every consecutive-leaf difference is `±2^w`, and applying it
/// carries or borrows across the `k − w` bits up to the `2^k` boundary.
/// Machine-word deltas are what a fixed-width lazy window absorbs, so this
/// family prices the deltas *wider than any such window*: a two-zone
/// accumulator (normalized prefix plus fixed-width buffer) is forced through
/// its normalized prefix on every tooth, while a representation with no
/// normalized region pays O(delta limbs). Each tooth stores `gamma(2^k − 2^w)`
/// — `2k − 1` bits — so under today's coding every crossing is paid for by a
/// comparably-wide input code.
///
/// Layout per tooth: `"11"` (spine node, `gamma(0)`), `"1" · gamma(2^k − 2^w)`
/// (tooth node), `"01"` (leaf 0), `"0" · gamma(2^w)` (leaf `2^w`); after all
/// `n` teeth, `"01"` (the terminal leaf 0). `2k + 2w + 6` bits per tooth plus
/// 2. Normal form holds everywhere: each spine node's right child has base 0,
/// each tooth's left leaf has base 0, and the only leaf pairs are `(0, 2^w)`.
///
/// # Panics
///
/// Panics if `w == 0`, `w ≥ k`, or `n == 0`.
fn wide_tooth_comb(k: usize, w: usize, n: usize) -> Packed {
    assert!(w >= 1, "wide-tooth comb needs a nonzero tooth width");
    assert!(
        w < k,
        "wide-tooth comb needs its cliff above its tooth width"
    );
    assert!(n >= 1, "wide-tooth comb needs at least one tooth");
    let mut bits = BitsMut::with_capacity(n * (2 * k + 2 * w + 6) + 2);
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
/// A root with base `2^k − 1` whose left child is a zero-base fan spine of `n`
/// teeth `(1, 0, 1)` — an internal node with base 1 over leaves 0 and 1 —
/// terminated by a leaf 0, with the root's required 0-leaf on the right. The
/// root-to-node path sum sits at `2^k − 1` across the whole fan, so a walk that
/// maintains a running path sum (enter: add the stored base; leave: subtract
/// it) crosses the `2^k` carry boundary *twice per tooth* — and each tooth
/// costs only 12 stored bits. One comparably-coded magnitude (the root's, paid
/// once) funds `n` crossings: the excursions are siblings, not nested, so no
/// Dyck-structure argument bounds them, and any accumulator that materializes
/// each crossing as a full-width carry does Θ(nk) limb work in a Θ(n + k)-bit
/// input. Consecutive-leaf *values* stay cliff-free (`2^k ↔ 2^k + 1`): the fan
/// prices entry/exit accumulation, the boundary comb prices leaf deltas.
///
/// Layout: `"1" · gamma(2^k − 1)` (root), then per tooth `"11"` (spine node),
/// `"1" · gamma(1)` (tooth node), `"01"` (leaf 0), `"0010"` (leaf 1); after all
/// `n` teeth, `"01"` (terminal fan leaf), `"01"` (the root's right leaf).
/// Normal form holds everywhere: the root's right leaf and each spine node's
/// non-tooth child have base 0, and the only leaf pairs are `(0, 1)`.
///
/// # Panics
///
/// Panics if `k == 0` or `n == 0`.
fn cliff_fan(k: usize, n: usize) -> Packed {
    assert!(k >= 1, "cliff fan needs a nonzero root magnitude");
    assert!(n >= 1, "cliff fan needs at least one tooth");
    let mut bits = BitsMut::with_capacity(12 * n + 2 * k + 6);
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

/// The cancelling-prefix chain `P(k, n)`: `n` peak-to-1 drops, `n(2k + 10) + 2`
/// bits.
///
/// The boundary comb's shape with the wide magnitude moved onto the *left*
/// leaf: teeth `(1, 2^k − 1, 0)` off a zero-base spine, terminated by a leaf 0,
/// so preorder leaf values oscillate `2^k ↔ 1`. Each drop from the peak leaves
/// a running-value accumulator holding a tiny value spelled with wide digits —
/// a high positive digit cancelled by a trail of negative ones — so the next
/// sign check cannot decide at the top digit and must scan (and collapse) the
/// whole cancelling prefix. Every drop is paid by its own `gamma(2^k − 1)`
/// input code, so the family prices deep sign scans against the wide writes
/// that immediately precede them. It does not exercise the collapse: a scan
/// funded by an adjacent write is linear whether or not the fold rewrites what
/// it scanned. The collapse-is-load-bearing case — a cancelling prefix built
/// once, then read many times — is a delta-stream shape, not a packed input,
/// and is pinned by the accumulator envelope suite's static-prefix stream.
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
fn cancelling_chain(k: usize, n: usize) -> Packed {
    assert!(k >= 1, "cancelling chain needs a nonzero peak magnitude");
    assert!(n >= 1, "cancelling chain needs at least one tooth");
    let mut bits = BitsMut::with_capacity(n * (2 * k + 10) + 2);
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
/// `d` zero-base internal nodes leaning left, each with a 1-leaf right sibling,
/// bottoming out in a 0-leaf: level `i`'s leaf contributes area `1/2^i`, so the
/// whole tree's rank telescopes to `(2^d − 1)/2^d` — the closed form this
/// module's tests pin. The numerator is the all-ones `d`-bit odd integer, so
/// the rank fold's running numerator is as wide as the depth already walked at
/// *every* level: any fold that re-shifts its accumulated numerator per level
/// does `Θ(d²)` limb work against `Θ(d)` input bits, which is what makes this
/// the separating family for the rank/distance/lag delta algebra. [`dense`] is
/// the control: same density, but its single 1-leaf keeps the fold's numerator
/// one bit wide.
///
/// Layout: `"11" × d` (internal flag + `gamma(0)`), `"01"` (the bottom 0-leaf),
/// then `"0010" × d` (each level's 1-leaf right sibling, innermost first).
/// Exactly `6d + 2` bits for `2d + 1` nodes at depth `d`. Normal form holds
/// everywhere: each internal node's left child stores base 0, and the only
/// sibling leaf pair is the bottom `(0, 1)`.
///
/// # Panics
///
/// Panics if `d == 0`: the spine needs at least one internal node.
fn harmonic(d: usize) -> Packed {
    assert!(d >= 1, "harmonic spine needs at least one internal node");
    let mut bits = BitsMut::with_capacity(6 * d + 2);
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
/// nodes whose single internal child sits *left at even depths and right at odd
/// depths*, each with a 0-leaf sibling, bottoming out in `(0, 0, 1)`. Same
/// density as [`dense`] (~2 bits per node, depth ~n/4 for `n` bits), but the
/// root-to-bottom route changes direction every level, so any per-level saved
/// state — walk frames, route bits, resume records — is maximally non-uniform:
/// nothing about a frame can be inferred from its neighbors, which makes this
/// the frame-count adversary for iterative walks that keep per-level records (a
/// fixed 16-byte frame per level costs ~32 bytes per input byte here).
///
/// Layout: at each level, `"11"` (internal node, `gamma(0)`), preceded by
/// `"01"` when the internal child sits right (the leaf sibling is emitted first
/// in preorder); the bottom node is `"01" · "0010"` (leaves 0, 1); unwinding,
/// each level whose internal child sat left emits its trailing `"01"` sibling.
/// Normal form holds everywhere: every internal node has a base-0 child, and
/// the only leaf pair is `(0, 1)`.
///
/// # Panics
///
/// Panics if `d == 0`: the spine needs at least one internal node.
fn alt_spine(d: usize) -> Packed {
    assert!(d >= 1, "alternating spine needs at least one internal node");
    let mut bits = BitsMut::with_capacity(4 * d + 4);
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

/// The scattered id `Z(e)`: `e` owned left subtrees at alternating depths of a
/// right-leaning spine, `6e + 2` bits.
///
/// The operand cross's id side for output-dominated projection: the party owns
/// the whole left child at every other level of a right-leaning spine — the
/// positions where [`cliff_comb`]'s teeth hang — so projecting a comb through
/// it keeps every second tooth. `e` disjoint owned fragments scatter across the
/// id tree at `Θ(1)` stored bits each, so the *input* is linear in `e` while
/// every kept fragment boundary forces a fresh wide magnitude into the
/// projected *output*.
///
/// Layout, repeated `e` times: `11` (both children present), `00` (the owned
/// left leaf), `01` (a right-only gap level); terminated by `00` (the owned
/// tip). 6 bits per fragment plus 2. Normal form: no node has two fully-owned
/// children (each `11` node's right child is a gap node) and no node has two
/// absent children.
///
/// # Panics
///
/// Panics if `e == 0`.
fn scattered_id(e: usize) -> Packed {
    assert!(e >= 1, "scattered id needs at least one owned fragment");
    let mut bits = BitsMut::with_capacity(6 * e + 2);
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
/// `divert`, the last unary node is right-only (`01`) instead, so `I(d, false)`
/// and `I(d, true)` share their first `d − 1` levels and own disjoint regions —
/// the pair shape that drives two-operand id walks to full lockstep depth.
/// Normal form: no `(1, 1)` node anywhere.
///
/// # Panics
///
/// Panics if `d == 0`.
fn id_spine(d: usize, divert: bool) -> Packed {
    assert!(d >= 1, "id spine needs at least one unary node");
    let mut bits = BitsMut::with_capacity(2 * d + 2);
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

/// The nested-full-sibling id `N(d)`: `(x, 1)` repeated down a left spine,
/// `4d + 4` bits.
///
/// Layout: `d` both-children tags (`11`) descending left, then the left-only
/// terminus `(1, 0)` (`10 · 00`; a `(1, 1)` terminus would break normal form),
/// then the `d` right-child terminals (`00`), innermost first — preorder closes
/// the spine's right children in reverse. Every level is a right-full shortcut
/// site over a matching event spine: the deepest stacking of the fill walk's
/// deferred right-full decisions and per-level raise bookkeeping per input bit.
///
/// # Panics
///
/// Panics if `d == 0`.
fn nested_full_id(d: usize) -> Packed {
    assert!(d >= 1, "nested-full id needs at least one shortcut level");
    let mut bits = BitsMut::with_capacity(4 * d + 4);
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

/// The mirror nested-full id `M(d)`: `(1, x)` repeated down a right spine,
/// `4d + 4` bits.
///
/// Layout: `d` both-children tags (`11`) descending right, each followed
/// immediately by its full left terminal (`00` — preorder visits the left child
/// first, so the terminals interleave with the spine tags instead of trailing
/// them), then the right-only terminus `(0, 1)` (`01 · 00`; a `(1, 1)` terminus
/// would break normal form). Every level is a left-full shortcut site over a
/// right-leaning event spine: the raised leaf precedes the range its minimum
/// comes from at every level, so the walk's memoized pre-scan (and the
/// pre-scan's own per-level bookkeeping) runs at maximal nesting per input bit.
///
/// # Panics
///
/// Panics if `d == 0`.
fn nested_left_full_id(d: usize) -> Packed {
    assert!(
        d >= 1,
        "nested-left-full id needs at least one shortcut level"
    );
    let mut bits = BitsMut::with_capacity(4 * d + 4);
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

/// A right-leaning spine of zero leaves with one `2^b − 1` tail leaf: depth
/// `d`, `4d + 2b + 3` bits.
///
/// Layout: `d` × (`1 · gamma(0) · 0 · gamma(0)`) — each spine node's zero-base
/// flag and its zero left leaf — then the bottom node's wide right leaf `0 ·
/// gamma(2^b − 1)`. Preorder leaf heights are `0, 0, …, 0, 2^b − 1`: every
/// proper subtree of the spine nets `+(2^b − 1)` from entry to exit while all
/// minima stay at zero, so any per-level bookkeeping that materializes subtree
/// nets (rather than carrying them relative to a shared anchor) re-touches the
/// tail's width once per level. Crossed with [`nested_left_full_id`], every
/// level is additionally a left-full pre-scan site.
///
/// # Panics
///
/// Panics if `b == 0` or `d == 0`.
fn wide_tail(b: usize, d: usize) -> Packed {
    assert!(b >= 1, "wide tail needs a nonzero magnitude");
    assert!(d >= 1, "wide tail needs a nonzero spine depth");
    let mut bits = BitsMut::with_capacity(4 * d + 2 * b + 3);
    for _ in 0..d {
        bits.push(true); // spine node flag ...
        codec::encode_int(&mut bits, &Base::from(0u8)); // ... base 0
        ev_leaf(&mut bits, 0); // its zero left leaf
    }
    ev_leaf_wide(&mut bits, &pow2_minus_1(b)); // the bottom's wide tail
    Packed::from_bits(bits)
}

/// The descending staircase `D(d)`: the dense left spine whose preorder leaf
/// heights descend `d, d − 1, …, 0` by unit deltas; `~5d` bits.
///
/// Layout: the root `1 · gamma(0)`, then `d − 1` × (`1 · gamma(1)`) (each
/// deeper spine node lifts its subtree's minimum by one), the bottom pair `0 ·
/// gamma(1) · 0 · gamma(0)`, then `d − 1` right-sibling zero leaves (`0 ·
/// gamma(0)`), innermost first. Min-lifted normal form holds at every node
/// (each node's right leaf sits exactly at its subtree's minimum), and no leaf
/// pair is equal. Every preorder leaf undercuts every leaf before it, so under
/// an id that pairs internal down the whole spine (`id_spine`), every consumed
/// leaf is a full-penetration minimum update through all open ranges — the
/// shape that separates per-level minimum bookkeeping (quadratic) from
/// run-compressed propagation (linear), independent of value width.
///
/// # Panics
///
/// Panics if `d == 0`.
fn staircase(d: usize) -> Packed {
    assert!(d >= 1, "the staircase needs at least one internal node");
    let mut bits = BitsMut::with_capacity(5 * d + 8);
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

/// Append one *hole region*: a right-leaning staircase of `m + 1` leaves
/// descending `m, m − 1, …, 0` by unit steps, entered through `lead − 1`
/// zero-base wrapper levels so its first leaf sits at depth exactly `lead`.
///
/// The engagement unit of the sub-scan hole pairs ([`collapse_hole`],
/// [`copy_hole`], [`raise_hole`]): the fused fill's sub-scans route a range
/// on its first descent's depth — per-leaf below depth 2, one block summary
/// at or above it — and this region is built to make the block side the
/// only cheap answer. The staircase body undercuts at every step, so a
/// per-leaf pass pays the walk's full register freight (and, on the
/// emitting scans, one watermark undercut) per leaf where the block summary
/// folds the whole range into one net movement and one streaming extremum;
/// the `lead` knob (2 or 3) places the routing decision on each side of the
/// depth-2 boundary, so the pairs alternating it keep every reroute of the
/// boundary — in either direction — measurable. In the stored skyline
/// coding the staircase's steps are unit deltas: the region's cost currency
/// scales with its leaf count, not its value widths.
///
/// Layout: `lead − 1` × (`1 · γ(0)`) (the wrappers), `1 · γ(0)` (the
/// staircase root), `0 · γ(m)` (the top step), `m − 1` × (`1 · γ(0) · 0 ·
/// γ(v)`) for `v = m − 1, …, 1` (the descending spine), `0 · γ(0)` (the
/// terminal step), then `lead − 1` × (`0 · γ(0)`) (each wrapper's trailing
/// floor leaf). `4·lead + 2(m − 1) + Σ_{v=1}^{m} 2·bitlen(v + 1)` bits.
/// Min-lifted normal form holds at every node (every subtree bottoms at
/// its zero floor), and no sibling leaf pair is equal.
fn hole_region(bits: &mut BitsMut, lead: usize, m: usize) {
    debug_assert!(lead >= 2, "a hole region's block routing needs depth 2+");
    debug_assert!(m >= 1, "a hole region needs at least one descending step");
    for _ in 0..lead - 1 {
        bits.push(true); // wrapper node: its floor leaf trails the region
        codec::encode_int(bits, &Base::ZERO);
    }
    bits.push(true); // the staircase root
    codec::encode_int(bits, &Base::ZERO);
    ev_leaf(bits, m as u64); // the top step: the region's first leaf
    for v in (1..m).rev() {
        bits.push(true); // each spine node ...
        codec::encode_int(bits, &Base::ZERO);
        ev_leaf(bits, v as u64); // ... steps its left leaf one down
    }
    ev_leaf(bits, 0); // the terminal step: the region minimum
    for _ in 0..lead - 1 {
        ev_leaf(bits, 0); // each wrapper's trailing floor leaf
    }
}

/// The collapse-hole pair `CH(k, m)`: `k` left-full sites with absent
/// right siblings down a right spine, each collapsing one deep hole
/// region at the walk's descend arm.
///
/// Returns `(event, id)`. Each unit's id node fully owns its left child —
/// a [`hole_region`] (leads alternating 2, 3 across units) — and lacks
/// its right one, so the walk's consuming max scan crosses the deep range
/// exactly once and nothing else does: an absent sibling launches no
/// pre-scan, and no covering left-full site exists anywhere. The pair
/// that concentrates `FillWalk::scan_max_consuming`'s routing at the
/// descend arm on deep ranges, where the committed tick families feed it
/// only leaf-scale ones (the `tick_collapse_hole` envelope pins the
/// readings); [`raise_hole`] is its ascend-arm dual.
///
/// Event layout, per unit: `1 · γ(0)` (spine node), `1 · γ(0)` (site
/// node), the hole region, `0 · γ(0)` (the absent-side sibling leaf);
/// after all `k` units, `0 · γ(0)` (the trailing no-stake leaf). Id
/// layout, per unit: `11` (`10` at the last unit's spine node), `10 · 00`
/// (the site: full left child, absent right). `6k` id bits.
///
/// # Panics
///
/// Panics if `k` is not an even count of at least 2 (the lead alternation
/// needs equal halves), or if `m == 0`.
fn collapse_hole(k: usize, m: usize) -> (Packed, Packed) {
    assert!(
        k >= 2 && k.is_multiple_of(2),
        "the collapse hole needs an even unit count"
    );
    assert!(m >= 1, "the collapse hole needs a nonzero region size");
    let mut ev = BitsMut::new();
    for i in 0..k {
        ev.push(true); // spine node
        codec::encode_int(&mut ev, &Base::ZERO);
        ev.push(true); // the unit's site node
        codec::encode_int(&mut ev, &Base::ZERO);
        hole_region(&mut ev, 2 + (i % 2), m); // the collapse range
        ev_leaf(&mut ev, 0); // the site's absent-side sibling leaf
    }
    ev_leaf(&mut ev, 0); // the spine's trailing no-stake leaf
    let mut id = BitsMut::new();
    for i in 0..k {
        id.push(true); // spine node: the unit hangs left ...
        id.push(i + 1 < k); // ... and the spine continues (absent at the end)
        id.push(true); // the unit's site: left full ...
        id.push(false); // ... with its right sibling absent
        id.push(false); // the full collapse child
        id.push(false);
    }
    (Packed::from_bits(ev), Packed::from_bits(id))
}

/// The copy-hole pair `CO(k, m)`: `k` absent-child hole regions down a
/// right spine, inside one covering pre-scan.
///
/// Returns `(event, id)`. The id's root is a left-full site over a
/// single-leaf collapse range (launching the covering fresh pre-scan);
/// each unit's id node then has an absent left child over a deep
/// [`hole_region`] (leads alternating 2, 3), terminated by an owned tip
/// over one tail leaf. Per unit the pre-scan copies the untouched range
/// once — its virtual emissions are the recorded currency — so the pair
/// concentrates `PreScan::copy_range` on deep ranges (the walk's own copy
/// of the same ranges rides the block regime the `tick_ownership_hole`
/// envelope already pins; the `tick_copy_hole` envelope pins this pair's
/// readings). The fill here is the identity, so the tick is the walk plus
/// one grow splice.
///
/// Event layout: `1 · γ(0) · 0 · γ(0)` (the root site and its collapsed
/// leaf), then per unit `1 · γ(0)` (spine node) and the hole region as its
/// left child; after all `k` units, `0 · γ(0)` (the owned tail leaf). Id
/// layout: `11 · 00` (the root site), `01` per unit (left absent, spine
/// continues right), `00` (the owned tail tip). `2k + 6` id bits.
///
/// # Panics
///
/// Panics if `k` is not an even count of at least 2, or if `m == 0`.
fn copy_hole(k: usize, m: usize) -> (Packed, Packed) {
    assert!(
        k >= 2 && k.is_multiple_of(2),
        "the copy hole needs an even unit count"
    );
    assert!(m >= 1, "the copy hole needs a nonzero region size");
    let mut ev = BitsMut::new();
    ev.push(true); // the root site's node
    codec::encode_int(&mut ev, &Base::ZERO);
    ev_leaf(&mut ev, 0); // its collapsed left leaf
    for i in 0..k {
        ev.push(true); // spine node
        codec::encode_int(&mut ev, &Base::ZERO);
        hole_region(&mut ev, 2 + (i % 2), m); // the absent-child range
    }
    ev_leaf(&mut ev, 0); // the owned tail leaf
    let mut id = BitsMut::new();
    id.push(true); // the root site: left full ...
    id.push(true); // ... over the spine
    id.push(false); // the full collapsed child
    id.push(false);
    for _ in 0..k {
        id.push(false); // left absent over the hole region ...
        id.push(true); // ... the spine continues right
    }
    id.push(false); // the owned tail tip
    id.push(false);
    (Packed::from_bits(ev), Packed::from_bits(id))
}

/// The raise-hole pair `RH(k, m)`: `k` right-full sites up a left chain,
/// each raising over one deep hole region at the walk's ascend arm.
///
/// Returns `(event, id)`. A left-leaning chain of `k` id nodes each
/// carries a fully-owned *right* child over a deep [`hole_region`] (leads
/// alternating 2, 3), so the right-full shortcut arm fires on the way
/// back up at every level and the walk's consuming max scan crosses each
/// deep range exactly once — no left-full site exists anywhere, so no
/// pre-scan runs and nothing else reads the regions. The pair that
/// concentrates `FillWalk::scan_max_consuming`'s routing at the ascend
/// arm on deep ranges (the `tick_raise_hole` envelope pins the readings);
/// [`collapse_hole`] is its descend-arm dual.
///
/// Event layout: `k` × (`1 · γ(0)`) (the left chain), `0 · γ(0)` (the
/// chain's bottom leaf), then the `k` hole regions, innermost first (each
/// chain node's right child, closed in preorder on the way out). Id
/// layout: `11` per chain node, `10 · 00` (the bottom's minimal non-full
/// id), then `00` per chain node's full right child, innermost first.
/// `4k + 4` id bits.
///
/// # Panics
///
/// Panics if `k` is not an even count of at least 2, or if `m == 0`.
fn raise_hole(k: usize, m: usize) -> (Packed, Packed) {
    assert!(
        k >= 2 && k.is_multiple_of(2),
        "the raise hole needs an even unit count"
    );
    assert!(m >= 1, "the raise hole needs a nonzero region size");
    let mut ev = BitsMut::new();
    for _ in 0..k {
        ev.push(true); // each chain node: its region trails in preorder
        codec::encode_int(&mut ev, &Base::ZERO);
    }
    ev_leaf(&mut ev, 0); // the chain's bottom leaf
    for i in (0..k).rev() {
        hole_region(&mut ev, 2 + (i % 2), m); // each node's raised right child
    }
    let mut id = BitsMut::new();
    for _ in 0..k {
        id.push(true); // chain node: the chain continues left ...
        id.push(true); // ... and its right child is full
    }
    id.push(true); // the bottom's minimal non-full id ...
    id.push(false);
    id.push(false); // ... its owned tip
    id.push(false);
    for _ in 0..k {
        id.push(false); // each chain node's full right child, innermost first
        id.push(false);
    }
    (Packed::from_bits(ev), Packed::from_bits(id))
}

/// The site-hole pair `SH(k, m)`: `k` interior left-full sites down a
/// right spine, each collapsing one deep hole region inside one covering
/// pre-scan.
///
/// Returns `(event, id)`. The id's root is a left-full site over a
/// single-leaf collapse range (launching the covering fresh pre-scan, as
/// [`copy_hole`]'s root does); each unit's id node then hangs a left-full
/// site over a deep [`hole_region`] (leads alternating 2, 3) with its
/// right sibling absent, so the pre-scan consumes each deep range through
/// `PreScan::skip_collapse` — the height movement is the only quantity
/// the range owes the scan (an absent sibling records no ledger link) —
/// and the walk's own consuming max scan crosses it once more at the
/// site's consume. The pair that concentrates the pre-scan's collapse
/// skip on deep ranges, where the committed tick families feed it only
/// leaf-scale ones (the `tick_site_hole` envelope pins the readings);
/// [`copy_hole`] is its untouched-range dual.
///
/// Event layout: `1 · γ(0) · 0 · γ(0)` (the root site and its collapsed
/// leaf), then per unit `1 · γ(0)` (spine node), `1 · γ(0)` (the site
/// node), the hole region, and `0 · γ(0)` (the site's absent-side sibling
/// leaf); after all `k` units, `0 · γ(0)` (the trailing no-stake leaf).
/// Id layout: `11 · 00` (the root site), then per unit `11` (`10` at the
/// last unit's spine node) and `10 · 00` (the site: full left child,
/// absent right). `6k + 4` id bits.
///
/// # Panics
///
/// Panics if `k` is not an even count of at least 2 (the lead alternation
/// needs equal halves), or if `m == 0`.
fn site_hole(k: usize, m: usize) -> (Packed, Packed) {
    assert!(
        k >= 2 && k.is_multiple_of(2),
        "the site hole needs an even unit count"
    );
    assert!(m >= 1, "the site hole needs a nonzero region size");
    let mut ev = BitsMut::new();
    ev.push(true); // the root site's node
    codec::encode_int(&mut ev, &Base::ZERO);
    ev_leaf(&mut ev, 0); // its collapsed left leaf
    for i in 0..k {
        ev.push(true); // spine node
        codec::encode_int(&mut ev, &Base::ZERO);
        ev.push(true); // the unit's site node
        codec::encode_int(&mut ev, &Base::ZERO);
        hole_region(&mut ev, 2 + (i % 2), m); // the collapse range
        ev_leaf(&mut ev, 0); // the site's absent-side sibling leaf
    }
    ev_leaf(&mut ev, 0); // the spine's trailing no-stake leaf
    let mut id = BitsMut::new();
    id.push(true); // the root site: left full ...
    id.push(true); // ... over the spine
    id.push(false); // the full collapsed child
    id.push(false);
    for i in 0..k {
        id.push(true); // spine node: the unit hangs left ...
        id.push(i + 1 < k); // ... and the spine continues (absent at the end)
        id.push(true); // the unit's site: left full ...
        id.push(false); // ... with its right sibling absent
        id.push(false); // the full collapse child
        id.push(false);
    }
    (Packed::from_bits(ev), Packed::from_bits(id))
}

/// The memo-chain event `Q(k, distinct)`: a right-leaning spine of `k`
/// single-leaf left-full sites, `~(14k + 9)` bits distinct (γ(j) codes), `13k +
/// 9` shared.
///
/// Layout: the root `1 · γ(0)` with left leaf `0 · γ(0)`, then per level `j =
/// 1..=k` the spine node `1 · γ(0)` over the site node `1 · γ(0) · 0 · γ(0) · 0
/// · γ(v_j)` (leaves 0 and `v_j`), terminated by `0 · γ(0)`. With `distinct`,
/// `v_j = j`; else every `v_j = 1`. Crossed with [`memo_chain_id`], the root is
/// one covering left-full site and every `(0, 0, v_j)` node an interior
/// left-full site whose range is the single leaf `v_j` — `k`
/// consumption-sibling memo records in one fresh scan, minima `v_j`. Distinct
/// minima make every recorded difference nonzero; the shared twin's differences
/// are all zero (the unstored case), so the pair separates per-record
/// bookkeeping from work that scales with the differences the records carry.
/// Normal form holds everywhere: every node's subtree minimum is its left
/// leaf's 0, and the only leaf pairs are `(0, v_j)` with `v_j ≥ 1`.
///
/// # Panics
///
/// Panics if `k == 0`.
fn memo_chain(k: usize, distinct: bool) -> Packed {
    assert!(k >= 1, "the memo chain needs at least one interior site");
    let mut bits = BitsMut::with_capacity(14 * k + 9);
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

/// The memo-chain id over [`memo_chain`]: `(1, ·)` at the root and at every
/// interior site, `10k + 8` bits.
///
/// Layout: the root tag `11` with full left terminal `00`, then per level the
/// spine tag `11`, the site id `(1, (1, 0))` (`11 · 00 · 10 · 00`), terminated
/// by `(1, 0)` (`10 · 00`). Normal form: no `(1, 1)` node — every full child's
/// sibling is internal or absent.
///
/// # Panics
///
/// Panics if `k == 0`.
fn memo_chain_id(k: usize) -> Packed {
    assert!(k >= 1, "the memo-chain id needs at least one interior site");
    let mut bits = BitsMut::with_capacity(10 * k + 8);
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

/// The memo-comb event `B(d)`: `d` alternating levels of a single-leaf site and
/// a covering site, `~(18d + 2·γlen(d))` bits.
///
/// Layout: the root `1 · γ(0)` with left leaf `0 · γ(0)`, then per level `i =
/// 1..=d`: `1 · γ(0)` (the covering site `X_{i+1}`'s range root) over `1 · γ(0)
/// · 0 · γ(0) · 0 · γ(i)` (the single-leaf site `A_i`, minimum `i`) and `1 ·
/// γ(0) · 0 · γ(0)` (the next covering site's node), terminated by the leaf `0
/// · γ(d + 1)`. Crossed with [`memo_comb_id`], one fresh scan records `2d + 1`
/// sites whose ranges interleave shallow (`A_i`, closing early) with covering
/// (`X_i`, closing late): recording order runs `A_1..A_d` then `X_{d+1}..X_1`
/// while the walk consumes `X_1, A_1, X_2, A_2, …` — every consecutive
/// consumption is Θ(d) apart in recording order, with ascending site minima
/// (`m(A_i) = m(X_i) = i`, the tail `d + 1`) keeping ~2d recorded differences
/// nonzero. Any resolution that walks recorded differences between
/// consecutively consumed sites — against the enclosing site or the previously
/// consumed one alike — re-reads Θ(d) of them per site; per-site records
/// anchored to the walk's own live state read O(1). Normal form: every node's
/// subtree minimum is 0 via its zero leaves, and no equal leaf pair exists.
///
/// # Panics
///
/// Panics if `d == 0`.
fn memo_comb(d: usize) -> Packed {
    assert!(d >= 1, "the memo comb needs at least one level");
    let mut bits = BitsMut::with_capacity(20 * d + 24);
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

/// The memo-comb id over [`memo_comb`]: a covering `(1, ·)` site per level
/// interleaved with the single-leaf sites' `(1, (1, 0))`, `14d + 12` bits.
///
/// Layout: the root tag `11 · 00`, then per level `11` (the covering range's
/// node) · `11 · 00 · 10 · 00` (the single-leaf site) · `11 · 00` (the next
/// covering site), terminated by `10 · 00`. Normal form: no `(1, 1)` node.
///
/// # Panics
///
/// Panics if `d == 0`.
fn memo_comb_id(d: usize) -> Packed {
    assert!(d >= 1, "the memo-comb id needs at least one level");
    let mut bits = BitsMut::with_capacity(14 * d + 12);
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

/// The memo fan-out event `F(k, b)`: the memo-chain skeleton with one `2^b − 1`
/// minimum shared by all `k` sites over the covering site's zero floor,
/// `~(13k + 2kb + 9)` bits.
///
/// Layout: [`memo_chain`]'s skeleton with every site's range leaf at `2^b − 1`
/// and its collapsed left leaf at `2^b − 2` — the stream climbs to the wide
/// plateau once and steps by units across all `k` sites, so the input pays the
/// width exactly once (unlike [`memo_oscillating`], whose input re-pays it per
/// site). Crossed with [`memo_chain_id`], the sites all share the wide minimum
/// while the covering site's own minimum is the zero terminal: the sibling
/// links are all zero (unstored) and exactly one ledger quantity (the first
/// site's deferred link against the covering minimum) carries the width — paid
/// once, independent of `k`. A recording discipline that anchors each site to
/// the covering floor instead materializes `k` wide records; the pinned
/// absolute touch ceiling is what such a fan-out blows. Normal form: leaf pairs
/// `(2^b − 2, 2^b − 1)`, every subtree minimum 0 via the zero terminal under
/// the root.
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
fn memo_fanout(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the memo fan-out needs at least one site");
    assert!(b >= 1, "the memo fan-out needs a nonzero magnitude");
    let wide = pow2_minus_1(b);
    let below = wide.clone() - &Base::from(1u8);
    let mut bits = BitsMut::with_capacity(13 * k + 4 * b + 9);
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

/// The oscillating-siblings event `O(k, b)`: the memo-chain skeleton with site
/// minima alternating `1` and `2^b − 1`, `~(13k + kb + 9)` bits.
///
/// Layout: [`memo_chain`]'s exactly, with `v_j = 2^b − 1` for odd `j` and `1`
/// for even. Crossed with [`memo_chain_id`], every sibling ledger link is wide
/// — but each site's range leaf codes the same width in the input, so the links
/// are funded one-for-one by the oscillation the input already paid for: the
/// control for the funding argument (flat touches per input byte, unlike the
/// fan-out, whose input pays its width once). Normal form: as [`memo_chain`]'s.
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
fn memo_oscillating(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the oscillating siblings need at least one site");
    assert!(b >= 1, "the oscillating siblings need a nonzero magnitude");
    let wide = pow2_minus_1(b);
    let one = Base::from(1u8);
    let mut bits = BitsMut::with_capacity(13 * k + k * b + 9);
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
/// descending run undercutting every open range minimum, `~(18d + 13)` bits.
///
/// Layout: the root `1 · γ(0)` (the covering site) with left leaf `0 · γ(0)`,
/// then per level `i = 1..=d` the nested carrier `1 · γ(0)` over the site `1 ·
/// γ(0) · 0 · γ(0) · 0 · γ(i + 1)` (minimum `i + 1`), bottoming in
/// [`staircase`]`(2d)`'s subtree (preorder heights `2d, 2d − 1, …, 0`). Crossed
/// with [`memo_churn_id`], each site's record is live on the ledger head while
/// the run's every leaf undercuts every open range — `~2d` full-penetration
/// minimum drops with `d` recorded minima in flight. One live head follows them
/// at one fold per drop; a discipline that keeps one live record per open level
/// folds all `d` per drop (quadratic), the refuted live-anchored followers'
/// tombstone. Normal form: leaf pairs `(0, i + 1)`, the run's unit-step
/// descent, and every subtree minimum 0 via the run's bottom.
///
/// # Panics
///
/// Panics if `d == 0`.
fn memo_churn(d: usize) -> Packed {
    assert!(d >= 1, "the memo churn needs at least one site");
    let mut bits = BitsMut::with_capacity(18 * d + 10 * (2 * d) + 20);
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

/// The memo-churn id over [`memo_churn`]: a covering `(1, ·)` root, per level
/// the site's `(1, (1, 0))` under a carrier whose right arm continues, and an
/// absent id over the descending run, `14d + 6` bits.
///
/// Layout: the root tag `11 · 00`, then per level `11` (the carrier) · `11 · 00
/// · 10 · 00` (the site), with the last carrier's tag `10` (right absent: the
/// run is walked as `fill(0, e) = e`, its emissions undercutting through every
/// open frame). Normal form: no `(1, 1)` node.
///
/// # Panics
///
/// Panics if `d == 0`.
fn memo_churn_id(d: usize) -> Packed {
    assert!(d >= 1, "the memo-churn id needs at least one site");
    let mut bits = BitsMut::with_capacity(14 * d + 6);
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

/// The descending-raises event `W(d)`: a floor realized high, then `d` sibling
/// sites whose minima step down from it, `~(13d + 26)` bits.
///
/// Layout: the root `1 · γ(0)` (the covering site) with left leaf `0 · γ(0)`,
/// then `1 · γ(0)` whose left leaf `0 · γ(d + 2)` arms the frame high before
/// any site, over the [`memo_chain`]-style spine with `v_j = d + 2 − j` — so
/// every site's raise lands BELOW the frame's minimum at its own consume, and
/// each consume's arm moves the tracked minimum the ledger relation must
/// survive. The one family whose raises exercise the decide-then-emit ordering:
/// a relation read after the raise emission is stale by exactly the arm's
/// delta, and the oracle differential catches the wrong values. Normal form:
/// leaf pairs `(0, d + 2 − j)` with `j ≤ d`, every subtree minimum 0 via the
/// zero terminal.
///
/// # Panics
///
/// Panics if `d == 0`.
fn descending_raises(d: usize) -> Packed {
    assert!(d >= 1, "the descending raises need at least one site");
    let mut bits = BitsMut::with_capacity(13 * d + 30);
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

/// The descending-raises id over [`descending_raises`]: the covering `(1, ·)`
/// root, an absent left over the floor leaf, then the memo-chain site ids,
/// `10d + 10` bits.
///
/// Layout: the root tag `11 · 00`, the floor carrier's `01` (left absent: the
/// floor leaf stays), then per site `11` (spine) · `11 · 00 · 10 · 00`,
/// terminated by `(1, 0)`. Normal form: no `(1, 1)` node.
///
/// # Panics
///
/// Panics if `d == 0`.
fn descending_raises_id(d: usize) -> Packed {
    assert!(d >= 1, "the descending-raises id needs at least one site");
    let mut bits = BitsMut::with_capacity(10 * d + 10);
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

/// The reveal-comb event `R(k, b)`: one covering site over a left-leaning comb
/// of `k` sibling sites sharing one wide minimum `2^b` above a zero floor,
/// `~(k(4b + 8) + 6)` bits.
///
/// Layout: the root `1 · γ(0)` (the covering site) with left leaf `0 · γ(0)`,
/// then `k` comb nodes `1 · γ(0)` leaning left (`a_i = node(0, a_{i−1},
/// site_i)`), the floor `0 · γ(0)` at the deepest left, then per site `1 · γ(0)
/// · 0 · γ(2^b − 1) · 0 · γ(2^b)` (leaves one apart at the shared wide
/// plateau). The input pays the width once — the stream climbs to the plateau
/// at the first site and steps by units after — and every site's fill collapses
/// to the equal pair's leaf, so the output is unit deltas too. Crossed with
/// [`reveal_comb_id`], each site is a left-full pre-scan site whose consume
/// arms the tracked minimum `2^b` above the floor, and the left-leaning spine
/// closes the site's node frame back into the 0-floor frame between consecutive
/// consumes: the width-`b` boundary difference is minted at every consume and
/// popped at every close — per-object-legal moves circulating one width with no
/// input delta, no output code, and no undercut descent funding any hop. Normal
/// form: no equal leaf pair exists (site pairs are `(2^b − 1, 2^b)`), and every
/// comb node's subtree minimum is 0 via the floor.
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
fn reveal_comb(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the reveal comb needs at least one site");
    assert!(b >= 1, "the reveal comb needs a nonzero magnitude");
    let wide = pow2(b);
    let below = pow2_minus_1(b);
    let mut bits = BitsMut::with_capacity(k * (4 * b + 8) + 6);
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

/// [`reveal_comb`] with the floor raised to `2^b − 2`: identical site forest,
/// identical close-reveal cycle, consume-time gap 2, `~(k(4b + 8) + 2b + 4)`
/// bits.
///
/// Layout: [`reveal_comb`]'s exactly, with the floor leaf at `0 · γ(2^b − 2)`.
/// The tracked minimum at every site consume sits 2 below the site's minimum
/// instead of `2^b` below, so the boundary difference the cycle circulates is
/// O(1) wide: the control that separates the wide *gap* (the cost driver) from
/// the forest shape and the deferral cycle (shared with the red family). Normal
/// form: as [`reveal_comb`]'s, the floor now one below the site pairs.
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
fn reveal_comb_hifloor(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the reveal comb needs at least one site");
    assert!(b >= 1, "the reveal comb needs a nonzero magnitude");
    let wide = pow2(b);
    let below = pow2_minus_1(b);
    let floor = wide.clone() - &Base::from(2u8);
    let mut bits = BitsMut::with_capacity(k * (4 * b + 8) + 2 * b + 4);
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

/// The reveal-comb id over [`reveal_comb`]: the covering `(1, ·)` root over
/// per-comb-level `(b_{i−1}, site)` tags with the site ids `(1, (1, 0))`,
/// `10k + 4` bits.
///
/// Layout: the root tag `11 · 00`, then `k − 1` comb tags `11` (deeper comb
/// left, site right), the deepest comb tag `01` (left absent: the floor stays),
/// then per site `11 · 00 · 10 · 00` — the site blocks trail the comb tags
/// because each site is its comb node's *right* child and the comb leans left.
/// Normal form: no `(1, 1)` node.
///
/// # Panics
///
/// Panics if `k == 0`.
fn reveal_comb_id(k: usize) -> Packed {
    assert!(k >= 1, "the reveal-comb id needs at least one site");
    let mut bits = BitsMut::with_capacity(10 * k + 4);
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

/// The pure-comb event `L(k, b)`: [`reveal_comb`]'s left-leaning comb with a
/// bare `2^b` leaf per level and NO covering site, `~(k(2b + 4) + 2)` bits.
///
/// Layout: `k` comb nodes `1 · γ(0)` leaning left, the floor `0 · γ(0)`, then
/// `k` leaves `0 · γ(2^b)` (each comb node's right child). Crossed with
/// [`pure_comb_id`], no left-full site exists anywhere — no memo, no pre-scan,
/// no site consume: each wide leaf is walked in its own leaf-under-internal-id
/// frame, whose first emission arms it `2^b` above the floor and whose close
/// pops the width-`b` boundary difference back — the watermark web's own
/// arm-move + close-pop cycle, isolated from the pre-scan's frame ledger.
/// Normal form: every comb node's subtree minimum is 0 via the floor, and no
/// two sibling leaves are equal (`2^b` pairs with an internal node or the
/// floor).
///
/// # Panics
///
/// Panics if `k == 0` or `b == 0`.
fn pure_comb(k: usize, b: usize) -> Packed {
    assert!(k >= 1, "the pure comb needs at least one level");
    assert!(b >= 1, "the pure comb needs a nonzero magnitude");
    let wide = pow2(b);
    let mut bits = BitsMut::with_capacity(k * (2 * b + 4) + 2);
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

/// The pure-comb id over [`pure_comb`]: per-comb-level `(b_{i−1}, (1, 0))`
/// tags, `6k` bits.
///
/// Layout: `k − 1` comb tags `11` (deeper comb left, the leaf's id right), the
/// deepest comb tag `01` (left absent: the floor stays), then `k` × `10 · 00` —
/// each level's `(1, 0)` node id over its wide leaf, the leaf-under-internal-id
/// frame shape. Normal form: no `(1, 1)` node.
///
/// # Panics
///
/// Panics if `k == 0`.
fn pure_comb_id(k: usize) -> Packed {
    assert!(k >= 1, "the pure-comb id needs at least one level");
    let mut bits = BitsMut::with_capacity(6 * k);
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

/// The ascending cliff `A(k, b)`: a right spine of `k` ascending wide left
/// leaves `2^b + i` over a terminal 0-cliff, `k(2b + 4) + 2` bits.
///
/// Layout: `k` spine nodes `1 · γ(0)`, each with left leaf `0 · γ(2^b + i)` (`i
/// = 1..=k`, ascending inward), the deepest node's right child the cliff `0 ·
/// γ(0)`. Crossed with [`ascend_cliff_id`], each ascending unit step arms its
/// own node's frame one above the enclosing frame's minimum — `k − 1` nonzero
/// unit boundary differences with no zero runs anywhere — and the cliff's
/// single wide undercut (residue `2^b + k`) then propagates through all of
/// them: the family whose cascade prices the fold *direction* of every hop, one
/// wide residue against `k − 1` narrow dying differences. The version's stored
/// skyline stream pays the width in O(1) codes (the first climb and the
/// terminal drop) and unit deltas between, so the input is Θ(k + b) and the
/// tick's output is the input with the cliff grown to `(0, 1, 0)` — Θ(k + b)
/// too, so a residue-width fold per hop survives the input+output denominator.
/// Normal form: every spine node's subtree minimum is 0 via the cliff, and no
/// two sibling leaves exist (each wide leaf pairs with an internal node; the
/// deepest pair is `(2^b + k, 0)`).
///
/// # Panics
///
/// Panics if `k == 0`, `b == 0`, or `k + 2 > 2^b` (the ascent must
/// stay inside the width-`b` gamma-code band the closed form counts).
fn ascend_cliff(k: usize, b: usize) -> Packed {
    ascend_spine(k, b, true)
}

/// [`ascend_cliff`] with every wide leaf leveled at `2^b + 1`: identical spine,
/// identical cliff undercut, all boundary differences zero, `k(2b + 4) + 2`
/// bits.
///
/// Layout: [`ascend_cliff`]'s exactly, `i` pinned to 1. Every frame arms at the
/// shared minimum, so the difference stack is one compressed zero run and the
/// cliff's wide undercut passes it whole in O(1): the control separating the
/// cascade's *hop count* (the cost driver under a per-hop width fold) from the
/// spine shape, the arming schedule, and the undercut itself, all shared with
/// the ascending family. Normal form: as [`ascend_cliff`]'s (each wide leaf
/// pairs with an internal node; the deepest pair is `(2^b + 1, 0)`).
///
/// # Panics
///
/// Panics if `k == 0`, `b == 0`, or `k + 2 > 2^b`.
fn ascend_cliff_plateau(k: usize, b: usize) -> Packed {
    ascend_spine(k, b, false)
}

/// The shared ascending-cliff layout: ascending leaves or the
/// leveled control.
fn ascend_spine(k: usize, b: usize, ascend: bool) -> Packed {
    assert!(k >= 1, "the ascending cliff needs at least one spine node");
    assert!(b >= 1, "the ascending cliff needs a nonzero magnitude");
    // Every leaf's gamma code must stay 2b + 1 bits: γ(n) codes
    // m = n + 1, so the deepest leaf needs 2^b + k + 1 < 2^(b+1),
    // i.e. k + 1 < 2^b.
    assert!(
        b >= usize::BITS as usize || (k + 1) >> b == 0,
        "the ascent must stay inside the width-b code band"
    );
    let wide = pow2(b);
    let mut bits = BitsMut::with_capacity(k * (2 * b + 4) + 2);
    for i in 1..=k {
        bits.push(true); // spine node S_i, i = 1..=k
        codec::encode_int(&mut bits, &Base::ZERO);
        let step = if ascend { i as u64 } else { 1 };
        ev_leaf_wide(&mut bits, &(&wide + step)); // its wide left leaf
    }
    ev_leaf(&mut bits, 0); // the cliff: S_k's right child
    Packed::from_bits(bits)
}

/// The exponent of the wide drop in [`freeze_position`] and of the in-pair drop
/// in [`freeze_parade`].
///
/// `2^288` is a ten-base-2^32-digit value, so a block's drift exceeds the
/// following unit code's one digit by more than the query folds' eight-digit
/// freeze allowance, and every block fires one freeze.
const FREEZE_POSITION_DROP_BITS: usize = 288;

/// The freeze-position spine `FP(k)`: a right spine of `2k` descending wide
/// left leaves whose consecutive drops alternate `2^288` and one, over a
/// terminal 0 leaf.
///
/// Exactly `4k(L + 2) + 2` bits for the one shared leaf-width band `L = 289 +
/// bitlen(k)`.
///
/// Layout: `2k` spine nodes `1 · γ(0)` leaning right, node `j`'s left leaf the
/// `j`-th value of the descent from `2^L + k(2^288 + 1)` (alternately dropping
/// `2^288` and `1`), the deepest node's right child the terminal `0 · γ(0)`.
/// Each block's wide drop re-arms live drift over the query folds' freeze
/// allowance and the following unit code fires the freeze, so a query fold
/// freezes `Θ(k)` times, at stream positions whose written span grows with
/// every block — the many-freezes genre: an accounting that reads an absolute
/// position (or re-reads any whole-history state) per freeze goes quadratic
/// here, while every committed comb fires O(1) freezes. The descent consumes
/// `k(2^288 + 1) < 2^L`, so every leaf shares the one `(L + 1)`-bit width and
/// the size formula is exact. `min_ticks(FP(k))` is the leaf sum `2k·2^L +
/// k(k−1)(2^288 + 1) + k` (every node minimum is 0 via the terminal leaf).
/// Normal form: values strictly descend (no equal siblings), every base is 0,
/// and every subtree minimum is 0.
///
/// # Panics
///
/// Panics if `k == 0`.
fn freeze_position(k: usize) -> Packed {
    assert!(k >= 1, "the freeze-position spine needs at least one block");
    let band = FREEZE_POSITION_DROP_BITS + 1 + bitlen(k);
    let wide = suanpan::UBig::ONE << FREEZE_POSITION_DROP_BITS;
    let unit = suanpan::UBig::ONE;
    let descent = (&wide + &unit) * suanpan::UBig::from(k as u64);
    let mut value = (suanpan::UBig::ONE << band) + descent;
    let mut bits = BitsMut::with_capacity(4 * k * (band + 2) + 2);
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
/// `2^608` spans 20 base-2^32 digits: more than the query folds' eight-digit
/// freeze allowance above the settling drop's ten
/// ([`PROMOTION_REARM_SETTLE_BITS`]), so every block's second freeze finds the
/// parked component over-wide and promotes it.
const PROMOTION_REARM_ARM_BITS: usize = 608;

/// The promotion re-arm settling exponent in [`promotion_rearm`].
///
/// `2^288` spans 10 digits: wide enough that the following unit code trips the
/// freeze trigger (10 > 1 + 8), narrow enough that the parked arming drift
/// exceeds it by more than the allowance (20 > 10 + 8).
const PROMOTION_REARM_SETTLE_BITS: usize = 288;

/// Span-building spine levels per block in [`promotion_rearm`]: the phase-1 run
/// of `32p` levels puts a `Θ(p)`-digit floor under the consumed-mass span the
/// blocks then re-arm across, at ~5 stored bits per level.
const PROMOTION_REARM_LEVELS_PER_BLOCK: usize = 32;

/// The promotion re-arm spine `PR(p)`: `32p` span-building levels down a right
/// spine, then `p` four-node re-arm blocks, over a terminal 1 leaf.
///
/// Exactly `1972p + 4` bits. Layout: `32p` spine nodes `(0, 1, ·)` / `(0, 0,
/// ·)` alternating (base 0, leaf heights 1, 0, 1, 0, … — 10 bits per pair),
/// then per block the node bases `2^608, 1, 2^288, 1` on the 0-leaf shape
/// (1,220 + 6 + 580 + 6 bits), closing in the leaf `1` (4 bits). The prefix's
/// ±1 oscillation never freezes while its interval masses' depths grow the
/// consumed span one digit per 32 levels, and its running range minima are all
/// zero, so the min-ticks web rides the whole prefix as one compressed zero run
/// (an ascending prefix would instead arm `Θ(p)` distinct nested minima — the
/// ascend-cliff heap genre, deliberately avoided: this family's adversarial
/// payload is the promotion schedule, not the web). Each block's `2^608` climb
/// re-arms parked drift over the query folds' freeze allowance (the following
/// unit fires the freeze that parks it), and its `2^288` climb re-freezes at a
/// drift the parked component exceeds by more than the allowance — one
/// promotion per block, `Θ(p)` promotions at O(1) stored codes each, so any
/// promotion accounting that re-reads whole-history state per arming goes
/// quadratic here while the family's suffix masses compact to O(1) balanced
/// terms. Every stored code is a delta the fold must consume, and
/// `min_ticks(PR(p)) = Σ bases = 16p + p(2^608 + 2^288 + 2) + 1` is the
/// closed-form semantic leg. Normal form: every prefix node reaches its subtree
/// minimum 0 through a later prefix 0 leaf, every block node's minimum is its
/// own 0 leaf, and no sibling leaf pair is equal.
///
/// # Panics
///
/// Panics if `p == 0`.
fn promotion_rearm(p: usize) -> Packed {
    assert!(
        p >= 1,
        "the promotion re-arm spine needs at least one block"
    );
    let arm = pow2(PROMOTION_REARM_ARM_BITS);
    let settle = pow2(PROMOTION_REARM_SETTLE_BITS);
    let zero = Base::ZERO;
    let one = Base::from(1u8);
    let mut bits = BitsMut::with_capacity(1972 * p + 4);
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

/// The promotion re-arm mate `PRM(p)`: the small twin of [`promotion_rearm`] —
/// the same `36p`-node right-spine topology with the 1, 0, 1, 0, … leaf
/// alternation running the whole spine.
///
/// Exactly `180p + 4` bits, and `min_ticks(PRM(p)) = 18p + 1`. Overlaid against
/// `PR(p)` it is the two-operand re-arm genre: the heights agree leaf for leaf
/// along the whole span-building prefix (the difference folds to zero, boundary
/// by boundary), and every block boundary folds a unit from this operand
/// against the other's wide climb — so the co-sweep's freezes and promotions
/// fire at boundaries where this operand's cheap codes set the funded width,
/// moving drift only the other operand's wide codes deposited. `PR(p)`
/// dominates it pointwise (equal on the prefix, `≥ 2^608` against `≤ 1` in the
/// blocks), so the pair measures collapse to exact rank identities. Normal
/// form: as [`promotion_rearm`]'s prefix, closing in the unequal leaf pair `(0,
/// 1)`.
///
/// # Panics
///
/// Panics if `p == 0`.
fn promotion_rearm_mate(p: usize) -> Packed {
    assert!(p >= 1, "the re-arm mate needs at least one block's worth");
    let zero = Base::ZERO;
    // The spine matches PR(p) node for node: 32p span-builder levels
    // plus the 4p block levels, the alternation running through both.
    let levels = (PROMOTION_REARM_LEVELS_PER_BLOCK + 4) * p;
    let mut bits = BitsMut::with_capacity(180 * p + 4);
    for level in 0..levels {
        bits.push(true); // spine node: alternating leaf left
        codec::encode_int(&mut bits, &zero);
        ev_leaf(&mut bits, u64::from(level % 2 == 0)); // 1, 0, 1, 0, …
    }
    ev_leaf(&mut bits, 1); // the terminal leaf: the unequal closer
    Packed::from_bits(bits)
}

/// Spine levels per suffix digit in [`dense_suffix`] and [`wide_arming`].
///
/// Each right-descent turn removes one isolated interval from the trailing run,
/// and a 33-level stride keeps successive gaps more than a full base-2^32 digit
/// apart, so the balanced signed-digit compaction (which cancels only
/// ones-runs) can never merge two of them into one term: the trailing interval
/// mass carries `d` incompressible digits.
const DENSE_SUFFIX_DIGIT_STRIDE: usize = 33;

/// The dense-suffix re-arm family `DS(p, d)`: a gap spine of `33d` levels, then
/// `p` four-node re-arm blocks at its bottom, over a terminal 1 leaf.
///
/// Exactly `134d + 1812p + 4` bits. The spine turns right every 33rd level —
/// the turn's 1-leaf is swept *before* the blocks, so its interval is absent
/// from the trailing mass — and left elsewhere, those right-sibling 0-leaves
/// swept *after* the blocks. The trailing mass is an all-ones run punctured by
/// `d` isolated gaps a full digit apart (the 33-level stride constant's
/// derivation): the interval mass behind every block, `Θ(d)` balanced digits
/// however it is assembled. Each block is [`promotion_rearm`]'s verbatim — a
/// `2^608` climb, a unit (the freeze that parks the wide drift), a `2^288`
/// climb, and a unit (the freeze whose promotion arms the query folds' ledger)
/// — one promotion per block at O(1) stored codes, so `Θ(p)` armings all owe
/// their debt across the same `Θ(d)`-dense trailing mass: a ledger settle that
/// walks the suffix once per arming (or re-reads a promoted prefix once per
/// window) goes quadratic here, and the balanced product-tree settle reads
/// flat. `min_ticks(DS(p, d)) = Σ bases = d + p(2^608 + 2^288 + 2) + 1` is the
/// closed-form semantic leg (the `d` term is the turn leaves, so a spine-less
/// generator fails it). Normal form: every spine node reaches its subtree
/// minimum 0 through a trailing 0-leaf, every block node's minimum is its own
/// 0-leaf, and no sibling leaf pair is equal.
///
/// # Panics
///
/// Panics if `p == 0` or `d == 0`.
fn dense_suffix(p: usize, d: usize) -> Packed {
    assert!(p >= 1, "the dense-suffix family needs at least one block");
    assert!(d >= 1, "the dense-suffix family needs at least one gap");
    let arm = pow2(PROMOTION_REARM_ARM_BITS);
    let settle = pow2(PROMOTION_REARM_SETTLE_BITS);
    let one = Base::from(1u8);
    let mut bits = BitsMut::with_capacity(134 * d + 1812 * p + 4);
    let trailing = gap_spine(&mut bits, d);
    for _ in 0..p {
        for base in [&arm, &one, &settle, &one] {
            bits.push(true); // block node: 0-leaf left, chain right
            codec::encode_int(&mut bits, base);
            ev_leaf(&mut bits, 0);
        }
    }
    ev_leaf(&mut bits, 1); // the block terminal: the last unit climb
    for _ in 0..trailing {
        ev_leaf(&mut bits, 0); // the left turns' siblings, innermost first
    }
    Packed::from_bits(bits)
}

/// The dense-suffix mate `DSM(p, d)`: the small twin of [`dense_suffix`] — the
/// same topology with every block node's base 1.
///
/// Exactly `134d + 24p + 4` bits, and `min_ticks(DSM(p, d)) = d + 4p + 1`.
/// Overlaid against `DS(p, d)`, heights agree leaf for leaf along the spine and
/// the trailing run (the difference folds to zero) and every block boundary
/// folds this operand's unit codes against the other's wide climbs, so the
/// co-sweep's freezes and promotions fire on drift only the wide operand
/// deposited — and the ledger's every arming owes its debt across the same
/// dense trailing mass. `DS(p, d)` dominates it pointwise (equal outside the
/// blocks, `≥ 2^608` against `≤ 4p` inside), so the pair measures collapse to
/// exact rank identities. Normal form: as [`dense_suffix`]'s.
///
/// # Panics
///
/// Panics if `p == 0` or `d == 0`.
fn dense_suffix_mate(p: usize, d: usize) -> Packed {
    assert!(p >= 1, "the dense-suffix mate needs at least one block");
    assert!(d >= 1, "the dense-suffix mate needs at least one gap");
    let one = Base::from(1u8);
    let mut bits = BitsMut::with_capacity(134 * d + 24 * p + 4);
    let trailing = gap_spine(&mut bits, d);
    for _ in 0..4 * p {
        bits.push(true); // block node: 0-leaf left, chain right
        codec::encode_int(&mut bits, &one);
        ev_leaf(&mut bits, 0);
    }
    ev_leaf(&mut bits, 1); // the block terminal
    for _ in 0..trailing {
        ev_leaf(&mut bits, 0);
    }
    Packed::from_bits(bits)
}

/// The wide-arming family `WA(w, d)`: the gap spine of [`dense_suffix`] over a
/// *single* re-arm block whose arming climb is `2^(32w)`.
///
/// One promotion whose parked mass is as wide as the input, owing its debt
/// across a trailing mass as dense as the input. Exactly `134d + 64w + 600`
/// bits. The one block climbs `2^(32w)` (parked at its unit), climbs `2^288`
/// (whose unit's freeze finds the parked component over-wide and promotes it —
/// the one ledger arming), and the sweep then consumes the `Θ(d)`-dense
/// trailing mass and descends, cancelling the plateau only after the ledger
/// entry is sealed. The exact debt embeds one `Θ(w)`-digit × `Θ(d)`-digit
/// product whose factors the input funds separately (`w` digits of arming code,
/// `d` spine turns), and the cancelling descent lands outside the ledger, so no
/// seam cancellation can dodge it: the settle's one aggregate product is the
/// ledger's wide × dense multiplication genre at its purest, priced at the
/// multiplication bound by the query fold's `integral` module doc's settle
/// bound — where a
/// per-digit schoolbook charge pays `Θ(w · d)` digit work against a `Θ(w +
/// d)`-bit operand, quadratic at `w = d`, the reading the committed schoolbook
/// kernel keeps failing beside the `ledger_wide_arming` flatness band.
/// `min_ticks(WA(w, d)) = d + 2^(32w) + 2^288 + 2 + 1` is the closed-form
/// semantic leg. Normal form: as [`dense_suffix`]'s.
///
/// # Panics
///
/// Panics if `w < 10` (the parked component must clear the settling drift's ten
/// digits by more than the freeze allowance) or `d == 0`.
fn wide_arming(w: usize, d: usize) -> Packed {
    assert!(
        w >= 10,
        "the wide arming must out-span the settling drift plus the allowance"
    );
    assert!(d >= 1, "the wide-arming family needs at least one gap");
    let arm = pow2(32 * w);
    let settle = pow2(PROMOTION_REARM_SETTLE_BITS);
    let one = Base::from(1u8);
    let mut bits = BitsMut::with_capacity(134 * d + 64 * w + 600);
    let trailing = gap_spine(&mut bits, d);
    for base in [&arm, &one, &settle, &one] {
        bits.push(true); // the one block: 0-leaf left, chain right
        codec::encode_int(&mut bits, base);
        ev_leaf(&mut bits, 0);
    }
    ev_leaf(&mut bits, 1); // the block terminal
    for _ in 0..trailing {
        ev_leaf(&mut bits, 0);
    }
    Packed::from_bits(bits)
}

/// The hoisted-window family `HW(w, d, t)`: [`wide_arming`]'s gap spine and
/// single re-arm block, with the block terminal deepened into the dense tail
/// spine `S(t)`.
///
/// The tail knob is the family's whole point. Its leaves ride the block's
/// plateau at unit deltas, its consumed interval mass is one contiguous run
/// whose balanced spelling compacts to O(1) digits, and its stored bases sum
/// to the 1 the terminal it replaces carried — so it funds no window density,
/// no settle width, and no freeze. What it does move is depth: the overlay
/// deepens by `t` levels, so the absolute digit position of every settle
/// cluster — the trailing window's punctured `Θ(d)`-digit run, the block's
/// own banked mass — rises by `~t/32` base-2^32 digits while every cluster's
/// *span* stays put. Work priced by cluster spans (the walk, the folds, the
/// settle products, the images the settle densifies) therefore reads flat
/// across a tail doubling, and work priced by a cluster's absolute position
/// scales with the tail — the axis the `hoisted_window` band in
/// `tests/meter.rs` prices through the densify column. Exactly `134d + 64w +
/// 4t + 600` bits; `min_ticks(HW(w, d, t)) = d + 2^(32w) + 2^288 + 2 + 1`,
/// independent of `t`. Normal form: as [`wide_arming`]'s, the tail by
/// [`dense`]'s own argument.
///
/// # Panics
///
/// Panics if `w < 10` (the parked component must clear the settling drift by
/// more than the freeze allowance), `d == 0`, or `t < 32(w + 2)`: the tail
/// must hoist the trailing window past every settle factor's cluster gap
/// limit, or the tail mass's own compacted digits merge into the trailing
/// cluster and the family stops separating span from position.
fn hoisted_window(w: usize, d: usize, t: usize) -> Packed {
    assert!(
        w >= 10,
        "the wide arming must out-span the settling drift plus the allowance"
    );
    assert!(d >= 1, "the hoisted-window family needs at least one gap");
    assert!(
        t >= 32 * (w + 2),
        "the tail must hoist the window past the settle factors' gap limit"
    );
    let arm = pow2(32 * w);
    let settle = pow2(PROMOTION_REARM_SETTLE_BITS);
    let one = Base::from(1u8);
    let mut bits = BitsMut::with_capacity(134 * d + 64 * w + 4 * t + 600);
    let trailing = gap_spine(&mut bits, d);
    for base in [&arm, &one, &settle, &one] {
        bits.push(true); // the one block: 0-leaf left, chain right
        codec::encode_int(&mut bits, base);
        ev_leaf(&mut bits, 0);
    }
    ev_spine(&mut bits, t); // the block terminal, deepened into the tail
    for _ in 0..trailing {
        ev_leaf(&mut bits, 0);
    }
    Packed::from_bits(bits)
}

/// Append the dense-suffix gap spine: `33d` zero-base levels turning right
/// every 33rd and left elsewhere.
///
/// A turn's 1-leaf is emitted before the descent; the return value is the count
/// of trailing 0-leaf siblings the caller must emit innermost-first after the
/// spine's terminal content.
fn gap_spine(bits: &mut BitsMut, d: usize) -> usize {
    let mut trailing = 0usize;
    for level in 0..DENSE_SUFFIX_DIGIT_STRIDE * d {
        bits.push(true); // spine node flag
        codec::encode_int(bits, &Base::ZERO);
        if level % DENSE_SUFFIX_DIGIT_STRIDE == 0 {
            ev_leaf(bits, 1); // the turn: its leaf leads the descent
        } else {
            trailing += 1; // the lean: its 0-leaf trails the subtree
        }
    }
    trailing
}

/// Append the parked-unit spine shared by [`weight_comb`] and
/// [`freeze_parade`]: `s` zero-base levels leaning left, with the *root's*
/// right child left to the caller.
///
/// The innermost node is `(0, 1, 0)` and every other level's right sibling is a
/// unit leaf. The spine's sole job is depth: the caller's block hangs as the
/// root's right child at position weight `2^(s − 1)`, bought once with `Θ(s)`
/// one-time topology bits, and the innermost right leaf's 0 drop parks one
/// digit-0 unit under everything that follows, so no value-emptiness or
/// write-watermark shortcut can stand in for the gap the block's events must
/// cross. Exactly `6s − 2` bits.
///
/// # Panics
///
/// Panics if `s < 2` (the innermost node and the root are distinct).
fn parked_unit_spine(bits: &mut BitsMut, s: usize) {
    assert!(s >= 2, "the parked-unit spine needs at least two levels");
    for _ in 0..s {
        bits.push(true); // spine node flag, base 0
        codec::encode_int(bits, &Base::ZERO);
    }
    ev_leaf(bits, 1); // the innermost node's left leaf
    ev_leaf(bits, 0); // its right leaf: the parked digit-0 unit
    for _ in 0..s - 2 {
        ev_leaf(bits, 1); // each middle level's right sibling, innermost first
    }
}

/// The weight-comb family `WC(n)`: the parked-unit spine at depth `32n`, then
/// one complete subtree of `2n` leaves alternating heights 0 and 2 as the
/// root's right child.
///
/// Exactly `202n − 4` bits. The rank integral deposits each leaf's live
/// component at its position weight `2^(S − depth)`, so the shallow block's ±1
/// oscillation lands alternating signs at one digit position `Θ(n)` digits
/// above the spine's parked unit — for O(1) stored bits per leaf, the position
/// weight being topology, not code. Every even-numbered block leaf cancels the
/// digit and the accumulator's top must settle back across the never-written
/// gap; every odd-numbered leaf re-raises it in one write — the many-jumps
/// genre: a settlement scan that steps the gap digit by digit pays `Θ(n)`
/// unfunded touches per event (`Θ(n²)` on linear input), and the parked digit-0
/// unit forecloses value-emptiness and write-watermark shortcuts, so consuming
/// one zero-run certificate per jumped run is what holds the cost flat (the
/// `skyline_flatness` weight-comb band in `tests/meter.rs` carries both
/// readings). `min_ticks(WC(n))` is the stored-base sum `34n − 1` (the spine's
/// `32n − 1` unit leaves plus the block's `n` twos). Normal form: the innermost
/// leaf pair is `(1, 0)`, every block pair is `(0, 2)`, and every subtree
/// minimum is 0.
///
/// # Panics
///
/// Panics if `n` is not a power of two (the block is one complete subtree).
fn weight_comb(n: usize) -> Packed {
    assert!(
        n.is_power_of_two(),
        "the weight-comb block is one complete subtree"
    );
    let mut bits = BitsMut::with_capacity(202 * n - 4);
    parked_unit_spine(&mut bits, 32 * n);
    // The block: a complete subtree over 2n leaves alternating 0 and 2,
    // every internal base 0.
    fn block(bits: &mut BitsMut, width: usize) {
        bits.push(true); // block node flag, base 0
        codec::encode_int(bits, &Base::ZERO);
        if width == 2 {
            ev_leaf(bits, 0);
            ev_leaf(bits, 2);
        } else {
            block(bits, width / 2);
            block(bits, width / 2);
        }
    }
    block(&mut bits, 2 * n);
    Packed::from_bits(bits)
}

/// The freeze-parade family `FZ(k)`: the parked-unit spine at depth `64k`, then
/// one complete subtree of `k` freeze blocks — wide leaf pairs dropping `2^288`
/// inside each pair and one across pairs — as the root's right child.
///
/// Exactly `1546k − 2` bits. Each pair's wide in-pair drop re-arms live drift
/// over the query folds' eight-digit freeze allowance (`2^288` spans ten
/// base-2^32 digits, the same width argument as [`freeze_position`]'s drop) and
/// the cheap cross-pair code fires the freeze, so a query fold freezes `Θ(k)`
/// times, every freeze settling its segment through the accumulator's scaled
/// read — and the segment's interval masses sit at the block's position weight,
/// `Θ(k)` digits above digit 0 (the blocks are shallow; the deep spine only
/// sets the scale). The write watermark is what lets each scaled read start at
/// the written span; a read that starts at digit 0 walks the `Θ(k)`-digit
/// never-written prefix per freeze — `Θ(k²)` touches on linear input, the
/// zero-padded magnitudes dragging the limb column with it (the
/// `skyline_flatness` freeze-parade band in `tests/meter.rs` carries both
/// readings). The block is min-lifted over a strictly descending run, every
/// node's minimum its last leaf, so right children code base 0 and left
/// children the difference of the halves' minima; `min_ticks(FZ(k))` is the
/// printed-base sum, re-derived in closed form by this module's tests and the
/// band. Normal form: values strictly descend (no equal siblings) and every
/// subtree minimum is 0 through the spine's parked unit.
///
/// # Panics
///
/// Panics if `k` is not a power of two (the parade is one complete
/// subtree).
fn freeze_parade(k: usize) -> Packed {
    assert!(
        k.is_power_of_two(),
        "the freeze parade is one complete subtree"
    );
    let wide = suanpan::UBig::ONE << FREEZE_POSITION_DROP_BITS;
    // One shared width band for the 2k descending values: the descent consumes
    // k(2^288 + 1) < 2^(289 + bitlen(k)), so the top value's width bounds them
    // all.
    let band = FREEZE_POSITION_DROP_BITS + 2 + bitlen(k);
    let mut values = Vec::with_capacity(2 * k);
    let mut v = suanpan::UBig::ONE << band;
    for _ in 0..k {
        values.push(v.clone());
        v -= &wide;
        values.push(v.clone());
        v -= suanpan::UBig::ONE;
    }
    let mut bits = BitsMut::with_capacity(1546 * k - 2);
    parked_unit_spine(&mut bits, 64 * k);
    // The min-lifted complete subtree over the descending run.
    fn block(bits: &mut BitsMut, vals: &[suanpan::UBig], parent_min: &suanpan::UBig) {
        if let [leaf] = vals {
            ev_leaf_wide(bits, &Base::from(leaf - parent_min));
            return;
        }
        let my_min = vals.last().expect("the parade block is nonempty");
        bits.push(true); // block node flag
        codec::encode_int(bits, &Base::from(my_min - parent_min));
        let (l, r) = vals.split_at(vals.len() / 2);
        block(bits, l, my_min);
        block(bits, r, my_min);
    }
    block(&mut bits, &values, &suanpan::UBig::ZERO);
    Packed::from_bits(bits)
}

/// The height of the lone-freeze plateau: one freeze-allowance-clearing drop
/// above the low tail, so the family's single mid-stream drop is the sweep's
/// one freeze.
///
/// `2^288 + 2` (ten base-2^32 digits): the drop from the plateau to the low
/// block exceeds the query folds' eight-digit freeze allowance over the
/// following unit code, the same width argument as [`freeze_position`]'s drop.
const LONE_FREEZE_PLATEAU_BITS: usize = 288;

/// The lone-freeze spine `LF(pre, post)`: `pre` unit-oscillation levels on a
/// wide plateau, one freeze-firing drop, then `post` unit-oscillation levels
/// near the floor, over a terminal 0 leaf.
///
/// Exactly `580·pre + 6·post + 14` bits. Layout: a right spine of `pre + post +
/// 2` nodes `1 · γ(0)`, left leaves in preorder at heights `H, H + 1, H, H + 1,
/// …` (`pre` leaves, `H = 2^288 + 2`), then `2` (the drop: one ten-digit delta,
/// within the freeze allowance of its own code), then `1` (the unit whose fold
/// fires the sweep's one freeze), then `2, 1, 2, 1, …` (`post` leaves), closing
/// in the terminal `0 · γ(0)`.
///
/// The family straddles the query folds' first-freeze gate from both sides, one
/// dial per axis:
///
/// - **`pre` (the late-freeze axis)**: the whole prefix runs strictly
///   before the sweep's first freeze — unit drift, no eviction — so
///   any per-interval deposit toward the settle machinery made before
///   drift exists to settle (the segment feed the gate holds shut)
///   scales with `pre` while the family's one settle never reads it.
/// - **`post` (the frozen-tail axis)**: the whole tail runs with the
///   gate open and the parked ten-digit drift live — every tail
///   interval feeds the segment mass the close's one `P · segment`
///   settle then reads at its watermark — so a tail feed or a close
///   read that is not amortized O(1) per interval scales with `post`
///   against O(1) funded wide codes.
///
/// Exactly one freeze fires (the tail's oscillation never re-trips the trigger)
/// and no promotion ever does (nothing is parked before the one freeze), so the
/// family also pins the settle's smallest nonempty configuration: one parked
/// drift against one final segment. `min_ticks(LF(pre, post))` is the leaf sum
/// `pre·(2^288 + 2) + pre/2 + 3·post/2 + 3` (every node minimum is 0 via the
/// terminal leaf). Normal form: every base is 0, every subtree reaches the
/// terminal 0, and the only sibling leaf pair is the deepest `(1, 0)` or `(2,
/// 0)`.
///
/// # Panics
///
/// Panics if `pre` or `post` is zero or odd (the closed forms count
/// whole oscillation pairs).
fn lone_freeze(pre: usize, post: usize) -> Packed {
    assert!(
        pre >= 2 && pre.is_multiple_of(2),
        "the lone freeze needs a whole-pair plateau prefix"
    );
    assert!(
        post >= 2 && post.is_multiple_of(2),
        "the lone freeze needs a whole-pair low tail"
    );
    let plateau = (suanpan::UBig::ONE << LONE_FREEZE_PLATEAU_BITS) + suanpan::UBig::from(2u8);
    let mut bits = BitsMut::with_capacity(580 * pre + 6 * post + 14);
    let leaf = |bits: &mut BitsMut, value: suanpan::UBig| {
        bits.push(true); // spine node: base 0, leaf left, spine right
        codec::encode_int(bits, &Base::ZERO);
        ev_leaf_wide(bits, &Base::from(value));
    };
    for j in 0..pre {
        leaf(&mut bits, &plateau + suanpan::UBig::from((j % 2) as u64));
    }
    leaf(&mut bits, suanpan::UBig::from(2u8)); // the drop: one wide delta
    leaf(&mut bits, suanpan::UBig::ONE); // the unit that fires the freeze
    for i in 0..post {
        leaf(&mut bits, suanpan::UBig::from((2 - i % 2) as u64)); // 2, 1, …
    }
    ev_leaf(&mut bits, 0); // the terminal leaf: every ancestor's minimum
    Packed::from_bits(bits)
}

/// The tooth-tail pair `TT(g, m)`: two same-shape right spines of `m` flat unit
/// leaves over a terminal 0, whose second leaves spike by `2^(32g)` in both
/// operands.
///
/// `b` runs one tick above `a` everywhere except the shared terminal. Exactly
/// `6m + 64g` bits per operand. The comparison sweep folds both spikes into one
/// difference at the same boundary — the spike cancels exactly, leaving the
/// difference at −1 spelled in one digit under a buffer `g` digits tall — and
/// then reads `sign(D)` once per boundary for the remaining `Θ(m)` boundaries
/// with no intervening write. Exact-`top` maintenance prices each read at the
/// value's own width; a high-water bound re-walks the spike's `g` dead digits
/// per read — `Θ(m·g)` on `Θ(m + g)` input, the cost the spike's own code paid
/// once (the `skyline_flatness` tooth-tail band in `tests/meter.rs` carries
/// both readings). `min_ticks` is the stored-base sum `m·h + 2^(32g)` per
/// operand (`h` the operand's unit height, 1 for `a` and 2 for `b`), and `a`
/// precedes `b` in the causal order. Normal form: every chain node's subtree
/// minimum is 0 through the terminal, and no sibling leaf pair is equal.
///
/// # Panics
///
/// Panics if `g == 0` (no spike) or `m < 2` (the spike rides the
/// second leaf).
fn tooth_tail(g: usize, m: usize) -> (Packed, Packed) {
    assert!(g >= 1, "the tooth-tail spike needs at least one digit");
    assert!(m >= 2, "the tooth-tail spike rides the second leaf");
    let spike = pow2(32 * g);
    let build = |base_h: u64| -> Packed {
        let mut bits = BitsMut::with_capacity(6 * m + 64 * g);
        for i in 0..m {
            bits.push(true); // chain node: leaf left, chain right, base 0
            codec::encode_int(&mut bits, &Base::ZERO);
            if i == 1 {
                ev_leaf_wide(&mut bits, &(&spike + Base::from(base_h)));
            } else {
                ev_leaf(&mut bits, base_h);
            }
        }
        ev_leaf(&mut bits, 0); // the shared terminal: every minimum
        Packed::from_bits(bits)
    };
    (build(1), build(2))
}

/// The puncture-product embedding `V(x, y)`: a gap spine whose turn leaves all
/// sit on the plateau `x` and whose turn positions spell the mass `2y`, over a
/// bottom 1 leaf and the trailing 0-leaves.
///
/// The arbitrary-product reduction behind the `Ω(M(·))` floor: the exact rank
/// of `V(x, y)` is `(2·x·y + 1) / 2^L` with `L = bits(2y)`, for *any* positive
/// integers `x` and `y` — one spine level per bit of `2y`, turning (its plateau
/// leaf leads the descent, at interval mass `2^(L − 1 − ℓ)` for level `ℓ`)
/// exactly where the mass bit is set and leaning (a trailing 0-leaf) elsewhere,
/// so the turns' interval masses sum to exactly `2y` and the rank integral is
/// `x · 2y` plus the bottom leaf's unit. The stored version is `Θ(bits(x) +
/// bits(y))` bits — the deltas collapse to one climb and one plunge, and each
/// mass bit rides one or two topology bits — so a fold that answers its rank
/// exactly computes the arbitrary product `x · y` (one subtraction and one
/// shift recover it from the numerator) at linear overhead: no exact fold beats
/// one integer multiplication of input-funded factors. Doubling the mass keeps
/// the last level a lean, which roots every subtree's minimum at 0 whatever
/// `y`'s low bit is. Normal form: every node's base is 0 and every subtree
/// reaches a trailing 0-leaf, and the only sibling leaf pair is the bottom `(1,
/// 0)`.
///
/// The packed construction spells the plateau once per turn, so the *packed*
/// size is `Θ(popcount(y) · bits(x) + bits(y))` even though the stored version
/// is `Θ(bits(x) + bits(y))`.
///
/// # Panics
///
/// Panics if `x` or `y` is zero.
fn puncture_product(x: &suanpan::UBig, y: &suanpan::UBig) -> Packed {
    use dashu_int::ops::BitTest;
    assert!(
        *x != suanpan::UBig::ZERO,
        "the plateau factor must be positive"
    );
    assert!(
        *y != suanpan::UBig::ZERO,
        "the mass factor must be positive"
    );
    let mass = y.clone() << 1usize;
    let levels = mass.bit_len();
    let turns = (0..levels).filter(|&b| mass.bit(b)).count();
    let plateau = Base::from(x.clone());
    let mut bits = BitsMut::with_capacity(4 * levels + turns * 2 * (x.bit_len() + 1) + 8);
    let mut trailing = 0usize;
    for level in 0..levels {
        bits.push(true); // spine node flag
        codec::encode_int(&mut bits, &Base::ZERO);
        if mass.bit(levels - 1 - level) {
            ev_leaf_wide(&mut bits, &plateau); // the turn: on the plateau
        } else {
            trailing += 1; // the lean: its 0-leaf trails the subtree
        }
    }
    ev_leaf(&mut bits, 1); // the bottom leaf: the plunge's near edge
    for _ in 0..trailing {
        ev_leaf(&mut bits, 0);
    }
    Packed::from_bits(bits)
}

/// Digit `i` of the deterministic pseudorandom content stream `seed`: the
/// SplitMix64 finalizer over `seed ⊕ i`, truncated to one base-2^32 digit, zero
/// remapped so every digit is live.
///
/// The committed incompressible-factor families draw their content here, so the
/// digits are reproducible, structureless (no runs, no arithmetic progression
/// for the balanced signed-digit compaction or a closed-form shortcut to grip),
/// and independent across seeds.
pub fn factor_digit(seed: u64, i: u64) -> u32 {
    let mut z = seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let digit = ((z ^ (z >> 31)) >> 16) as u32;
    if digit == 0 {
        0x9E37_79B9
    } else {
        digit
    }
}

/// A dense pseudorandom magnitude of exactly `32·digits` bits: every base-2^32
/// digit nonzero from [`factor_digit`]'s stream `seed`.
///
/// The top bit is forced set (exact width), the top digit's bit 30 forced clear
/// (never an all-ones digit), and bit 0 forced clear (so `+ 1` never carries
/// past digit 0 — the gamma code of the value stays at its closed-form width).
///
/// # Panics
///
/// Panics if `digits == 0`.
pub fn dense_factor(seed: u64, digits: usize) -> suanpan::UBig {
    assert!(digits >= 1, "a dense factor needs at least one digit");
    let mut bytes = vec![0u8; 4 * digits];
    for i in 0..digits {
        let mut digit = factor_digit(seed, i as u64);
        if i == 0 {
            digit = (digit & !1) | 2;
        }
        if i == digits - 1 {
            digit = (digit | 0x8000_0000) & !0x4000_0000;
        }
        bytes[4 * i..4 * i + 4].copy_from_slice(&digit.to_le_bytes());
    }
    suanpan::UBig::from_le_bytes(&bytes)
}

/// Content-stream seed for the plateau-puncture plateau factor.
const PLATEAU_PUNCTURE_X_SEED: u64 = 0x5054_5058; // "PTPX"

/// Content-stream seed for the plateau-puncture turn-position jitter.
const PLATEAU_PUNCTURE_J_SEED: u64 = 0x5054_504A; // "PTPJ"

/// The committed factors of the plateau-puncture family `PP(w, d)`: the
/// incompressible plateau `x` and the jittered punctured mass `y` whose product
/// the family's exact rank embeds.
///
/// - `x = dense_factor(w)`: exactly `32w` bits of pseudorandom digit
///   content, so neither `x` nor the plunge the fold parks (`x − 1`,
///   which differs only inside digit 0) compacts below `Θ(w)`
///   balanced signed digits.
/// - `y = Σᵢ₌₁ᵈ 2^(66(i−1) + 33 + jᵢ)`: `d` isolated bits at
///   pseudorandom jitters `jᵢ ∈ 0..32` (`j_d = 31` fixed, so the
///   width is the closed form `66d − 1` bits), successive bits
///   `35..=97` positions apart — always more than a full base-2^32
///   digit, so the balanced compaction can never merge two of them
///   and the mass's spelling is exactly `d` terms — and never an
///   arithmetic progression, so no geometric-series closed form
///   telescopes the product the way it would for a fixed stride.
///
/// # Panics
///
/// Panics if `w == 0` or `d == 0`.
pub fn plateau_puncture_factors(w: usize, d: usize) -> (suanpan::UBig, suanpan::UBig) {
    assert!(w >= 1, "the plateau needs at least one digit");
    assert!(
        d >= 1,
        "the plateau-puncture family needs at least one turn"
    );
    let x = dense_factor(PLATEAU_PUNCTURE_X_SEED, w);
    let mut y = suanpan::UBig::ZERO;
    for i in 1..=d {
        let jitter = if i == d {
            31
        } else {
            u64::from(factor_digit(PLATEAU_PUNCTURE_J_SEED, i as u64)) % 32
        };
        y += suanpan::UBig::ONE
            << usize::try_from(66 * (i as u64 - 1) + 33 + jitter)
                .expect("turn positions fit usize");
    }
    (x, y)
}

/// The plateau-puncture family `PP(w, d)`: the committed incompressible
/// instance of [`puncture_product`] — `V(x, y)` at the
/// [`plateau_puncture_factors`] content.
///
/// The answer-embedded-product family: the exact rank is `(2·x·y + 1) /
/// 2^(66d)` — a `Θ(w)`-digit × `Θ(d)`-term integer product whose factors the
/// input funds separately (`64w` bits of plateau code, `Θ(d)` topology bits),
/// with both factors' *content* incompressible under the settle's own balanced
/// signed-digit compaction (the factors' doc carries the two arguments). No
/// settle can telescope it — the complement sits at height 0, reached by one
/// funded plunge, so the product is the answer, not an accounting artifact;
/// [`puncture_product`] is the same embedding over arbitrary factors, which is
/// what makes the floor a reduction from arbitrary integer multiplication
/// rather than a bet on one shape. The stored skyline operand is `Θ(w + d)`
/// bits (the packed construction spells the plateau per turn, but the deltas
/// the version stores collapse to one climb and one plunge). The fold's cost on
/// this family is the close-time settle `P · segment` — parked `−(x − 1)`
/// against the punctured trailing mass — with no promotion ever firing: the
/// arming-free instance of the width × density residual. `min_ticks(PP(w, d)) =
/// d · x + 1` is the closed-form semantic leg. Exactly `d(64w + 262) + 4` bits.
/// Normal form: [`puncture_product`]'s.
///
/// # Panics
///
/// Panics if `w < 10` (the plunge must trip the freeze allowance past
/// a unit code) or `d == 0`.
fn plateau_puncture(w: usize, d: usize) -> Packed {
    assert!(
        w >= 10,
        "the plateau must out-span the freeze allowance past a unit code"
    );
    let (x, y) = plateau_puncture_factors(w, d);
    puncture_product(&x, &y)
}

/// The arming-train family `AT(n, w, g, alternate)`: `n` re-arm blocks with
/// `2^(32w)` armings, each preceded by its own `33g`-level gap-spine window.
///
/// The armings alternate sign when `alternate` and all climb otherwise; the
/// blocks ride one plateau band over the trailing 0-leaves.
///
/// The multi-arming ledger family the single-block shapes cannot reach: `n`
/// promotions whose parked masses are `Θ(w)` digits wide, with a `Θ(g)`-digit
/// incompressible interval mass banked *between* every consecutive pair of
/// armings (each gap's turn leaves sit at the running plateau, zero deltas, so
/// the windows are bought with topology alone). Every block spells `±2^(32w),
/// +1, +2^288, +1` in leaf absolutes: the wide swing parks at its unit, the
/// kicker's unit fires the freeze whose promotion arms the ledger — one entry
/// per block, sign following the swing — and the sweep closes with one funded
/// plunge whose parked width settles against the trailing run. With
/// `alternate`, consecutive entries cancel digit-wise inside the product tree's
/// parked sums; without it, every aggregate keeps the full arming width against
/// every dense window to its right. All wide leaves live in one gamma band
/// (`band = 32w + ⌈log₂ n⌉ + 2` headroom bits over the swings and kickers), so
/// the packed size is the closed form `n(g(2·band + 132) + 8·band + 16) + 2`
/// bits; the tests mirror the leaf recurrence for the `min_ticks` leg. Normal
/// form: all bases 0 — every subtree reaches a 0-leaf (the gap leans' trailing
/// siblings; the bottom 0 under the last block's wide leaf) and no sibling leaf
/// pair is equal.
///
/// # Panics
///
/// Panics if `n == 0` or `g == 0`, or if `w < 19` (an arming must
/// out-span the `2^288` kicker drift by more than the freeze
/// allowance, or promotion never fires).
fn arming_train(n: usize, w: usize, g: usize, alternate: bool) -> Packed {
    assert!(n >= 1, "the arming train needs at least one block");
    assert!(g >= 1, "the arming train needs at least one gap per window");
    assert!(
        w >= 19,
        "an arming must out-span the kicker drift plus the freeze allowance"
    );
    let band = 32 * w + bitlen(n) + 2;
    let arm = suanpan::UBig::ONE << (32 * w);
    let kicker = suanpan::UBig::ONE << PROMOTION_REARM_SETTLE_BITS;
    // The plateau band's floor plus double-swing headroom: every wide leaf
    // below stays inside [2^band, 2^(band+1)), one gamma width.
    let mut plateau = (suanpan::UBig::ONE << band) + (&arm << 1);
    let mut bits = BitsMut::with_capacity(n * (g * (2 * band + 132) + 8 * band + 16) + 2);
    let mut trailing = 0usize;
    for b in 0..n {
        for level in 0..DENSE_SUFFIX_DIGIT_STRIDE * g {
            bits.push(true); // window spine node flag
            codec::encode_int(&mut bits, &Base::ZERO);
            if level % DENSE_SUFFIX_DIGIT_STRIDE == 0 {
                // The turn: on the plateau, so the window's dense mass
                // is topology-funded (a zero delta in the store).
                ev_leaf_wide(&mut bits, &Base::from(plateau.clone()));
            } else {
                trailing += 1;
            }
        }
        if alternate && b % 2 == 1 {
            plateau -= &arm; // the swing: this block's arming descends
        } else {
            plateau += &arm;
        }
        for kick in [
            suanpan::UBig::ZERO, // the swing leaf itself
            suanpan::UBig::ONE,  // parks the swing
            kicker.clone(),      // the kicker
            suanpan::UBig::ONE,  // fires the promoting freeze
        ] {
            plateau += kick;
            bits.push(true); // block node: wide leaf left, chain right
            codec::encode_int(&mut bits, &Base::ZERO);
            ev_leaf_wide(&mut bits, &Base::from(plateau.clone()));
        }
    }
    // The bottom 0: the last block node's right child, unequal to its wide left
    // sibling, and the zero that keeps every block subtree's minimum at the
    // all-bases-0 spelling.
    ev_leaf(&mut bits, 0);
    for _ in 0..trailing {
        ev_leaf(&mut bits, 0);
    }
    Packed::from_bits(bits)
}

/// The ascending-cliff id over [`ascend_cliff`]: a right-descent `(0, ·)` chain
/// bottoming in `(1, 0)` over the cliff, `2k + 4` bits.
///
/// Layout: `k` tags `01` (left absent — the wide leaves stay), then `10 · 00` —
/// the `(1, 0)` node over the cliff, whose owned left half makes the cliff the
/// tick's one grow site. Normal form: no `(1, 1)` node.
///
/// # Panics
///
/// Panics if `k == 0`.
fn ascend_cliff_id(k: usize) -> Packed {
    assert!(
        k >= 1,
        "the ascending-cliff id needs at least one spine node"
    );
    let mut bits = BitsMut::with_capacity(2 * k + 4);
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

/// The raise value of every [`dominated_undercut`] site: each site's raised
/// leaf lands here, and the whole tick's closed form is the input with every
/// zero position lifted to it.
const DOMINATED_UNDERCUT_RAISE: u64 = 3;

/// The exit rise of every [`dominated_undercut`] site: the last leaf of the
/// copied region sits this far above the region's minimum, so the site's
/// block-minimum emission carries exactly this word-scale nonzero offset.
const DOMINATED_UNDERCUT_EXIT_RISE: u64 = 1;

/// The dominated-undercut spine `DU(k, b)`: `k` sibling raise sites, each
/// whose copied region climbs `5 · 2^b` and returns, `k(2b + 26) + 2` bits.
///
/// Layout: a right-leaning zero-base spine of `k` levels `1 · γ(0)`, each
/// level's left child the *site* `1 · γ(0)` over the raise leaf `0 · γ(0)` and
/// the copied region `1 · γ(3) · (1 · γ(0) · 0 · γ(5·2^b) · 0 · γ(0)) · 0 ·
/// γ(1)` — an internal node with base [`DOMINATED_UNDERCUT_RAISE`] whose left
/// child holds the wide leaf `5 · 2^b` beside a zero and whose right leaf
/// rises [`DOMINATED_UNDERCUT_EXIT_RISE`] above the region's minimum —
/// terminated by a leaf 0. Crossed with [`dominated_undercut_id`], each site's
/// left-full raise diverges the walk and arms at the sibling region's minimum,
/// the region's first (wide) leaf re-arms the watermark web at the climb's
/// top, and the region's remaining block then returns below with the exit one
/// above its minimum — so the block-minimum emission arrives with no latent, a
/// word-scale nonzero offset, and an anchor gap of `−(5·2^b − 1)`: the
/// emission arm where post-sign domination must decide a wide-negative gap
/// against a word and move the residue out whole, once per site. The residue
/// annihilates the site's own arming boundary exactly, so no boundary parks
/// and every site re-enters the same state; the terminal's right-full raise
/// then reads the surviving minimum, which pins the residue's documented
/// polarity `m − v = −gap − offset` in the tick's own output. The width `5 ·
/// 2^b` makes the domination read's decision a closed form: the gap magnitude
/// `5·2^b − 1` at `b ≥ 128` has its top digit at base-`2^32` index at least 4,
/// and the sign fold's running partial reaches the domination bound at or
/// above index 3 — one digit of descent at most — which is `sign_dominates_at`
/// floor `1` (the word bound) plus the two-digit clearance the certificate
/// requires. Normal form: every node's child minima meet 0 (each site's raise
/// leaf and the wide leaf's zero sibling), and no sibling leaves are equal.
///
/// # Panics
///
/// Panics if `k == 0`, or if `b < 128` (the closed-form decidability bound
/// above).
fn dominated_undercut(k: usize, b: usize) -> Packed {
    assert!(
        k >= 1,
        "the dominated-undercut spine needs at least one site"
    );
    assert!(
        b >= 128,
        "the wide width must decide the word-bound domination read"
    );
    let wide = pow2(b + 2) + pow2(b); // 5 · 2^b
    let raise = Base::from(DOMINATED_UNDERCUT_RAISE);
    let rise = Base::from(DOMINATED_UNDERCUT_EXIT_RISE);
    let mut bits = BitsMut::with_capacity(k * (2 * b + 26) + 2);
    for _ in 0..k {
        bits.push(true); // spine node
        codec::encode_int(&mut bits, &Base::ZERO);
        bits.push(true); // the site's node
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf(&mut bits, 0); // the raise leaf (the site's owned left half)
        bits.push(true); // the copied region's root: base = the raise value
        codec::encode_int(&mut bits, &raise);
        bits.push(true); // the climb carrier
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf_wide(&mut bits, &wide); // the wide climb: 5 · 2^b
        ev_leaf(&mut bits, 0); // the return to the region's minimum
        ev_leaf_wide(&mut bits, &rise); // the exit, one above the minimum
    }
    ev_leaf(&mut bits, 0); // the spine terminal (the right-full raise's leaf)
    Packed::from_bits(bits)
}

/// The dominated-undercut id over [`dominated_undercut`]: per site a `(1, 0)`
/// node over the raise leaf, bottoming in a full terminal, `6k + 2` bits.
///
/// Layout: `k` levels `11` (the spine node) · `10 · 00` (the site: a full left
/// child over the raise leaf, the copied region unowned), then `00` — the full
/// terminal, whose right-full shortcut at the deepest spine node raises the
/// terminal leaf to the enclosing watermark minimum: the read that surfaces
/// any mis-propagated residue in the tick's own bytes. Normal form: no
/// `(1, 1)` node.
///
/// # Panics
///
/// Panics if `k == 0`.
fn dominated_undercut_id(k: usize) -> Packed {
    assert!(k >= 1, "the dominated-undercut id needs at least one site");
    let mut bits = BitsMut::with_capacity(6 * k + 2);
    for _ in 0..k {
        bits.push(true); // the spine node: the site ...
        bits.push(true); // ... then deeper
        bits.push(true); // the site: full left child ...
        bits.push(false); // ... over an unowned copied region
        bits.push(false); // the full left terminal
        bits.push(false);
    }
    bits.push(false); // the spine terminus: full
    bits.push(false);
    Packed::from_bits(bits)
}

/// The narrow rung of the propagate-seam shapes: `5·2^64`.
///
/// The smallest magnitude that is three base-2^32 digits wide with its top
/// digit at the sign fold's decision bound (a top digit of 5 decides a
/// domination read at its first touch).
///
/// Every stacked boundary and every dying residue the seam shapes mint holds
/// exactly this width: wide enough that the compacting instantiation cannot
/// store it inline (`u64` covers two digits), narrow enough that the dying
/// side's one fold is three digit touches — the unit the seam bands' floors
/// count in.
fn seam_rung() -> Base {
    pow2(66) + pow2(64)
}

/// A `w`-digit magnitude whose top digit is 5: `5·2^(32(w−1))`, the seam
/// shapes' wide-operand constructor.
///
/// The top digit of 5 makes every domination read a closed form: the sign
/// fold's running partial reaches the decision bound (magnitude 3) at the top
/// digit itself, so `sign_dominates_at` decides — or honestly refuses — on
/// the digit-index clearance alone, with no descent, in one digit touch
/// (suanpan's witness `decision_bound_top_decides_on_the_first_touch`).
fn seam_wide(w: usize) -> Base {
    pow2(32 * (w - 1) + 2) + pow2(32 * (w - 1))
}

/// The seam-plunge spine `SP(k, r)`: `k + 1` ascending three-digit armings
/// over a floor `r` digits below, then one plunge to the floor,
/// `(k + 1)(64r − 56) + 2` bits.
///
/// Layout: a right-leaning spine of `k + 1` zero-base nodes `1 · γ(0)`, node
/// `i` carrying the left leaf `0 · γ(5·2^(32(r−1)) + i·5·2^64)`, terminated by
/// the plunge leaf `0 · γ(0)`. The sweep arms each spine range one rung
/// ([`seam_rung`], three digits) above its parent's minimum — `k` stacked
/// three-digit `Boundary::Wide` differences, each funded by its own consumed
/// delta code — and the plunge then drives one `r`-digit residue
/// ([`seam_wide`]`(r) + (k + 1)` rungs, same digit count) through all of
/// them. At `r = 5` every hop sits exactly at the wide-hop guards' two-digit
/// clearance line (`residue digits = boundary digits + 2`, the least
/// clearance `sign_dominates_at` can certify), so each boundary dies by one
/// fold of its own three digits into the residue — the
/// residue-dominates arm at its decision boundary, which no other committed
/// shape reaches (their dying differences are word-scale or the hop is an
/// exact annihilation). Growing `r` at fixed `k` moves only the residue's
/// clearance while the dying width stays put: the axis the clearance band
/// prices.
///
/// Normal form: every node's child minima meet 0 (the plunge leaf bottoms
/// the spine and every suffix contains it), and no sibling leaves are equal.
/// The size formula reads each ascending leaf at `bitlen = 32(r − 1) + 3`
/// (the rung sum stays under the top digit for every admitted `k`).
///
/// # Panics
///
/// Panics if `k == 0`, `r < 5` (two-digit clearance over a three-digit
/// boundary needs a top index of at least 4), or `k > 2^28` (the rung sum
/// must stay three digits, below the plunge base's own bit length).
fn seam_plunge(k: usize, r: usize) -> Packed {
    assert!(
        k >= 1,
        "the seam plunge needs at least one stacked boundary"
    );
    assert!(
        r >= 5,
        "the plunge residue must clear a three-digit boundary by two digits"
    );
    assert!(
        k <= 1 << 28,
        "the rung sum must stay within the ascending leaves' shared bit length"
    );
    let rung = seam_rung();
    let base = seam_wide(r);
    let leaf_bits = 64 * r - 58;
    let mut bits = BitsMut::with_capacity((k + 1) * (leaf_bits + 2) + 2);
    let mut value = base;
    for _ in 0..=k {
        value += &rung;
        bits.push(true); // spine node, base 0
        codec::encode_int(&mut bits, &Base::ZERO);
        ev_leaf_wide(&mut bits, &value); // the ascending arming leaf
    }
    ev_leaf(&mut bits, 0); // the plunge: the whole tree's floor
    Packed::from_bits(bits)
}

/// The seam-plunge control `SPC(k, r)`: the same spine, arming schedule, and
/// wire prefix as [`seam_plunge`], with the plunge leaf replaced by one more
/// ascent, `136k + 64r + 78` bits.
///
/// Layout: the ascent rides the bases instead of the leaves — node 1 is `1 ·
/// γ(5·2^(32(r−1)) + 5·2^64)` over the leaf `0 · γ(0)`, nodes 2..=k+1 are `1
/// · γ(5·2^64)` over `0 · γ(0)`, and the terminal right leaf is `0 ·
/// γ(5·2^64)` — so the *stored wire* is identical to `SP(k, r)`'s except its
/// final delta code (one rung up instead of the plunge). The web stacks the
/// same `k` three-digit boundaries; the final leaf reads one positive sign
/// and drives nothing, so the run's difference against `SP(k, r)` isolates
/// the plunge's propagation: per boundary, one domination read plus the
/// boundary's dying three-digit fold, less the control's drain (each of its
/// surviving boundaries parks into the latent register at its close). The
/// seam-plunge band prices that difference.
///
/// Normal form: minima ascend inward, so every node's leaf child sits at its
/// own subtree minimum (rel 0) and the bases carry the ascent; the terminal
/// pair is `(0, rung)`.
///
/// # Panics
///
/// As [`seam_plunge`].
fn seam_plunge_control(k: usize, r: usize) -> Packed {
    assert!(
        k >= 1,
        "the seam plunge needs at least one stacked boundary"
    );
    assert!(
        r >= 5,
        "the plunge residue must clear a three-digit boundary by two digits"
    );
    assert!(
        k <= 1 << 28,
        "the rung sum must stay within the ascending leaves' shared bit length"
    );
    let rung = seam_rung();
    let mut bits = BitsMut::with_capacity(136 * k + 64 * r + 78);
    bits.push(true); // node 1: base = the first arming's absolute height
    codec::encode_int(&mut bits, &(seam_wide(r) + &rung));
    ev_leaf(&mut bits, 0);
    for _ in 0..k {
        bits.push(true); // each deeper node climbs one rung
        codec::encode_int(&mut bits, &rung);
        ev_leaf(&mut bits, 0);
    }
    ev_leaf_wide(&mut bits, &rung); // the terminal: one more rung up
    Packed::from_bits(bits)
}

/// The seam-stop spine `SS(k)`: one five-digit boundary absorbing `k`
/// three-digit dying residues, `164k + 266` bits.
///
/// Layout: a root `1 · γ(0)` over the floor leaf `0 · γ(0)` and the descent
/// subtree — a node `1 · γ(B)` (`B = 5·2^128 − 2^80 − k·5·2^64`) whose spine
/// of `k − 1` zero-base nodes carries the leaves `0 · γ(2^80 + j·5·2^64)`
/// for `j = k..=1` in preorder, terminated by `0 · γ(0)`. The floor leaf
/// arms the root at the floor; the subtree's first leaf arms one boundary of
/// exactly `5·2^128` (five digits, top digit 5) above it; and each later
/// leaf descends one rung ([`seam_rung`]) — an arming undercut whose
/// three-digit residue climbs to the stacked boundary, is decided by the
/// wide-hop guard's other arm (`boundary digits = residue digits + 2` at
/// every hop, the survivor's clearance never leaving the decision bound's
/// reach), and dies by one fold of its own three digits into the survivor.
/// The final leaf drops to the subtree's own floor (`2^80 + 5·2^64`, still
/// three digits) as a plain undercut through the same arm. No other
/// committed shape reaches the boundary-dominates arm at all: their
/// cascades either annihilate exactly or penetrate to the stack's end.
///
/// Each cycle leases one fresh accumulator at its arming and retires its
/// dead residue in propagation, so the shape is also the steady-state
/// arm/retire churn family the pool-miss row drives: the pool's fill phase
/// is the peak outstanding lease count, not the churn length.
///
/// Normal form: the descent subtree's minimum is its final leaf (rel 0), the
/// root's children minima are `(0, B)`, and no sibling leaves are equal. All
/// descending leaves share `bitlen = 81` (the rung sum stays under `2^80`
/// for every admitted `k`).
///
/// # Panics
///
/// Panics if `k == 0` or `k > 2^11` (the descending leaves must share one
/// bit length and stay strictly positive under the shared base).
fn seam_stop(k: usize) -> Packed {
    let mut bits = BitsMut::with_capacity(164 * k + 266);
    bits.push(true); // the root, base 0
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the floor leaf: arms the root at 0
    seam_stop_descent(&mut bits, k);
    Packed::from_bits(bits)
}

/// The seam-stop control `SSC(k)`: [`seam_stop`]'s descent subtree standing
/// alone, `164k + 262` bits.
///
/// With no floor leaf below it, the subtree's first leaf is the web's first
/// arming — no boundary stacks — so each descending residue propagates into
/// an empty difference stack and retires: the same leaf codes, folds, sign
/// reads, and lease/retire churn as [`seam_stop`], less exactly the
/// per-cycle boundary hop (and the drain's one park). The seam-stop band
/// prices the difference.
///
/// # Panics
///
/// As [`seam_stop`].
fn seam_stop_control(k: usize) -> Packed {
    let mut bits = BitsMut::with_capacity(164 * k + 262);
    seam_stop_descent(&mut bits, k);
    Packed::from_bits(bits)
}

/// Append the seam-stop descent subtree: the based node over `k` descending
/// three-digit leaves and the rel-0 terminal (the layouts above).
fn seam_stop_descent(bits: &mut BitsMut, k: usize) {
    assert!(
        k >= 1,
        "the seam stop needs at least one descending residue"
    );
    assert!(
        k <= 1 << 11,
        "the descending leaves must share one bit length over a positive base"
    );
    let rung = seam_rung();
    let ladder = |j: usize| {
        // 2^80 + j·5·2^64: the descending leaves, one rung apart.
        let mut climb = rung.clone();
        climb *= u32::try_from(j).expect("the admitted k fits u32");
        pow2(80) + climb
    };
    // B = 5·2^128 − 2^80 − k·5·2^64: the first arming's absolute height is
    // then exactly 5·2^128, the stacked boundary's value.
    let base = (pow2(130) + pow2(128)) - &ladder(k);
    bits.push(true); // the descent node carries the subtree's floor
    codec::encode_int(bits, &base);
    for j in (1..=k).rev() {
        if j < k {
            bits.push(true); // spine node, base 0
            codec::encode_int(bits, &Base::ZERO);
        }
        ev_leaf_wide(bits, &ladder(j));
    }
    ev_leaf(bits, 0); // the subtree's floor: the final plain undercut
}

/// The latent-ladder comb `LL(w, k)`: one parked `w`-digit latent boundary
/// read by `k` scale-disparate undercut decisions, `k(64w − 56) + 64w − 48`
/// bits.
///
/// Layout: a left-leaning spine of `k` zero-base nodes, node `i` carrying
/// the right leaf `0 · γ(5·2^(32(w−1)) − (k − i + 1))`; the spine bottoms in
/// a head node `1 · γ(0)` over the floor leaf `0 · γ(0)` and the parked
/// pair `1 · γ(5·2^(32(w−1)))` over leaves `0 · γ(0) · 0 · γ(1)`. The floor
/// leaf arms every spine range at the floor; the parked pair's first leaf
/// arms one `w`-digit boundary above it ([`seam_wide`]`(w)`, funded by its
/// own consumed delta); and the pair's close *parks* that boundary in the
/// latent register. Each spine leaf then arrives one to `k` below the
/// stale anchor with no pending range — a strictly negative word-scale
/// `gap` against a live `w`-digit latent, the scale-disparate undercut
/// decision `decide_undercut_through_latent` answers by top-index
/// domination (the latent's top digit of 5 decides at its first touch),
/// returning "no undercut" with nothing changed. The `k` axis counts
/// decisions; the `w` axis widens only the parked operand the decisions
/// must *not* read across — the marginal decision cost across a `w`
/// doubling is the O(1) claim's meter.
///
/// Normal form: every spine node's child minima meet 0 through the floor
/// leaf, the parked pair's leaves are `(0, 1)`, and no sibling leaves are
/// equal (each ladder leaf is `w` digits, its sibling subtree bottoms at 0).
///
/// # Panics
///
/// Panics if `w < 3` (domination over a one-digit gap needs a top index of
/// at least 2), `k == 0`, or `k > 2^20` (the ladder leaves must stay one
/// word-scale step apart below the anchor, sharing one bit length).
fn latent_ladder(w: usize, k: usize) -> Packed {
    assert!(
        w >= 3,
        "the parked latent must decide domination over a word-scale gap"
    );
    assert!(k >= 1, "the ladder needs at least one decision leaf");
    assert!(
        k <= 1 << 20,
        "the ladder leaves must share one bit length just under the anchor"
    );
    let anchor = seam_wide(w);
    let leaf_bits = 64 * w - 58;
    let mut bits = BitsMut::with_capacity(k * (leaf_bits + 2) + 64 * w - 48);
    for _ in 0..k {
        bits.push(true); // spine node, base 0
        codec::encode_int(&mut bits, &Base::ZERO);
    }
    bits.push(true); // the head node, base 0
    codec::encode_int(&mut bits, &Base::ZERO);
    ev_leaf(&mut bits, 0); // the floor leaf: arms every open range at 0
    bits.push(true); // the parked pair: base = the anchor
    codec::encode_int(&mut bits, &anchor);
    ev_leaf(&mut bits, 0);
    ev_leaf(&mut bits, 1);
    // The ladder: preorder emits the deepest spine node's right leaf first,
    // so the values descend one per leaf from anchor − 1 to anchor − k.
    for j in 1..=k {
        let value = anchor.clone() - &Base::from(j as u64);
        ev_leaf_wide(&mut bits, &value);
    }
    Packed::from_bits(bits)
}

/// Shared-spine levels per isolated position digit in [`jump_pair`].
///
/// Each right-descent turn sets one isolated bit of every absolute position
/// below it, and a 33-level stride keeps successive bits more than a full
/// base-2^32 digit apart, so the balanced signed-digit compaction (which
/// cancels only ones-runs) can never merge two of them into one term: every
/// absolute position inside the comb carries `d` incompressible digits.
const JUMP_PAIR_DIGIT_STRIDE: usize = 33;

/// The two-operand jump comb `JP(k, m, d)`: a version pair whose height
/// difference crests `k` bits wide at every one of `m` comb levels, deep under
/// a spine that makes every absolute position `d` digits dense.
///
/// Each operand is certified-linear alone; the shape exists only in the
/// two-operand composition.
///
/// Both operands share a descent spine of `33d` zero-base levels that turns
/// right every 33rd level (one 0-leaf consumed *before* the comb per turn — the
/// `d` isolated position bits the stride constant derives) and left elsewhere
/// (those 0-leaf siblings trail after), then diverge in an `m`-level
/// right-leaning comb at the spine's bottom. Per comb level, over a
/// quarter-interval *tooth* and a quarter-interval *gap*:
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
/// Their overlay interleaves them: `|h_a − h_b|` sits at 1–2 inside every tooth
/// and `~2^k` across every gap, so per level a wide crest funded by `A`'s tooth
/// code rides over cheap codes from the *other* operand (`k ≥ 320` bits clears
/// the freeze allowance, so the drift is parked at `B`'s first cheap boundary —
/// `2m` freezes per distance, each fired by the operand that did not pay for
/// the drift). The spine makes every absolute position `d` incompressible
/// digits while the per-crest *segment* masses compact to O(1) digits: an
/// accounting that multiplies parked drift by absolute positions pays `Θ(m · d
/// · k)` limb work against a `Θ(m·k + d)`-bit input, and the anchored-segment
/// co-sweep (`version/skyline/query/integral.rs`'s module doc) settles each
/// crest against its own segment and stays linear — the separation the
/// `skyline_flatness` band test and the board cell hold. The **join** (the band
/// shades every gap) collapses to unit steps around one climb, and either
/// operand's own rank is flat: both inputs are individually innocuous, and the
/// shape exists only in the two-operand composition.
///
/// Layout, shared spine (level `ℓ = 0..33d`): `1 · γ(0)`, with the 0-leaf `0 ·
/// γ(0)` emitted before the descent at right turns (`ℓ ≡ 32 mod 33`) and queued
/// after it otherwise — 4 bits per level. Comb level `i = 1..=m`, teeth
/// operand: `1 · γ(0)` (spine `c_i`), `1 · γ(0)` (the tooth/gap pair node), `0
/// · γ(2^k + 3)` (the tooth), `1 · γ(0) · 0 · γ(1) · 0 · γ(0)` (the gap pair) —
/// `2k + 14` bits. Band operand: `1 · γ(2^k + 1)` at `c_1` (the hoisted plateau
/// base, `2k + 2` bits) and `1 · γ(0)` below, `1 · γ(0)` (the pair node), `1 ·
/// γ(0) · 0 · γ(1) · 0 · γ(0)` (the band pair, relative), `0 · γ(0)` (the gap
/// leaf) — 14 bits per level plus the one wide root code. Both end in the comb
/// terminal `0 · γ(0)` and the trailing left-turn siblings. Totals: `132d +
/// m(2k + 14) + 2` bits (teeth) and `132d + 14m + 2k + 2` bits (band). Normal
/// form holds everywhere: every spine and pair node has a 0-leaf or 0-min child
/// in reach, the band comb's plateau lift sits on `c_1`'s own base, and no two
/// sibling leaves are equal (`(1, 0)` pairs; every wide leaf pairs with an
/// internal node).
///
/// # Panics
///
/// Panics if `k < 3` (the closed form needs `γ(2^k + 3)` at `2k + 1` bits), `m
/// == 0`, or `d == 0`.
fn jump_pair(k: usize, m: usize, d: usize) -> (Packed, Packed) {
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
    let mut bits = BitsMut::with_capacity(132 * d + m * (2 * k + 14) + 2);
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
/// balanced fork of `n` parties, ticked so every adjacent region flips which
/// operand dominates — the emit side-switch population.
///
/// The seed party forks balanced to `n` single-leaf owners (leaf `i` of the
/// depth-`log2 n` fork tree). Each version joins `n` independent per-party
/// histories — party `i`'s empty version ticked to its target alone, then all
/// `n` merged through `join_all`'s balanced fold, so construction is `O(n log
/// n)` and every tick lands exactly one height unit (an isolated history has no
/// higher neighbor for the tick's fill leg to lift toward). The targets make
/// the winner alternate by leaf parity with no two adjacent plateaus ever
/// equal: leaf `i` reaches `3 + (i mod 3)` ticks on the dominant side and `1 +
/// (i mod 3)` on the other, the dominant side even-`i` for the first version
/// and odd-`i` for the second. The join's plateau sequence is `3 + (i mod 3)`
/// and the meet's `1 + (i mod 3)` — each adjacent-distinct, so neither emission
/// ever collapses a boundary and **every one of the `n − 1` overlay boundaries
/// is a side switch, in the join and the meet alike** (the corpus pairing `w =
/// v + one seed tick` reaches at most one). All heights are word-scale: the
/// family prices the switch machinery's density, not width. Every leaf's
/// dominant and dominated heights differ by exactly 2, so `distance = Σᵢ 2/n` —
/// the integer rank 2 at every `n`, which the generator's test pins as the
/// semantic witness that the schedule realized the heights it claims.
///
/// # Panics
///
/// Panics if `n` is not a power of two at least 2 (the balanced fork and the
/// parity schedule both need it).
fn concurrent_pair(n: usize) -> (crate::Version, crate::Version) {
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
    (v_parts.into_iter().collect(), w_parts.into_iter().collect())
}

/// The staggered-comb fold operand `SG(n, m, i)`: operand `i` of an `n`-operand
/// fold population whose `m` unit teeth land in the gaps of every other
/// operand's, `m(4L + 6) − 2` bits for `L = log2 n`.
///
/// The population's shared domain is `n·m` dyadic slots: a complete depth-`log2
/// m` top tree over `m` blocks of `n` slots each. Operand `i` owns slot `i` of
/// every block — a height-1 tooth at the end of a depth-`L` path whose
/// direction at level `t` is bit `L − 1 − t` of `i` (most significant first),
/// every path sibling a 0-leaf. Distinct operands' teeth never share a slot
/// and, because every slot's neighbors inside a partial union are zero gaps or
/// teeth of *other* heights' paths, no merge of a proper operand subset ever
/// collapses a boundary: each internal merge of the balanced reduction emits a
/// result near the sum of its inputs' sizes — the intermediate-swell loading
/// the fold's `O(D log k)` model prices, held until the last level (the full
/// union is the constant-1 skyline, so only the final merges collapse).
/// [`stagger_population`] carries the feed order that realizes the swell at
/// every reduction level.
///
/// Layout: `m − 1` top nodes `1 · γ(0)` in preorder, then per block `L` path
/// nodes `1 · γ(0)` (the 0-leaf sibling `0 · γ(0)` emitted before the deeper
/// child when the path turns right, after it when left), the tooth `0 · γ(1)`
/// at the bottom. `2(m − 1) + m(4L + 4)` bits. Normal form holds everywhere:
/// every internal node's subtree minimum is 0 (each path node has a 0-leaf
/// child; top nodes reach 0 through them), and the only sibling leaf pair is
/// the deepest node's `(1, 0)`.
///
/// # Panics
///
/// Panics if `n` is not a power of two at least 2, `m` is not a power
/// of two, or `i ≥ n`.
fn stagger_comb(n: usize, m: usize, i: usize) -> Packed {
    assert!(
        n >= 2 && n.is_power_of_two(),
        "the staggered comb needs a power-of-two operand count"
    );
    assert!(
        m >= 1 && m.is_power_of_two(),
        "the staggered comb needs a power-of-two block count"
    );
    assert!(i < n, "the operand index addresses one of the n slots");
    let levels = n.trailing_zeros();
    // The per-block path: L internal nodes toward slot i, a 0-leaf sibling per
    // level, the unit tooth at the bottom. Depth L + log2(m) is word-scale for
    // any buildable population, so plain recursion is safe here (the generators
    // are test-only construction code).
    fn path(bits: &mut BitsMut, levels: u32, i: usize, t: u32) {
        bits.push(true); // path node flag
        codec::encode_int(bits, &Base::ZERO);
        let deeper = |bits: &mut BitsMut| {
            if t + 1 == levels {
                ev_leaf(bits, 1); // the tooth: operand i's unit height
            } else {
                path(bits, levels, i, t + 1);
            }
        };
        if (i >> (levels - 1 - t)) & 1 == 0 {
            deeper(bits); // the slot sits left ...
            ev_leaf(bits, 0); // ... its right sibling is the gap
        } else {
            ev_leaf(bits, 0); // the gap sits left ...
            deeper(bits); // ... the slot right
        }
    }
    fn top(bits: &mut BitsMut, levels: u32, i: usize, m: usize) {
        if m == 1 {
            path(bits, levels, i, 0);
            return;
        }
        bits.push(true); // top node flag
        codec::encode_int(bits, &Base::ZERO);
        top(bits, levels, i, m / 2);
        top(bits, levels, i, m / 2);
    }
    let mut bits = BitsMut::with_capacity(m * (4 * levels as usize + 6) - 2);
    top(&mut bits, levels, i, m);
    Packed::from_bits(bits)
}

/// The staggered id `SI(n, m, i)`: [`stagger_comb`]'s party twin — operand `i`
/// owns slot `i` of every block, `m(2L + 4) − 2` bits.
///
/// The same slot domain as the comb: a complete depth-`log2 m` top of
/// both-present tags over per-block paths of `L` single-child tags to the owned
/// slot, the path siblings absent. The `n` operands are pairwise disjoint by
/// construction (distinct slots), their union is the whole seed region, and
/// every operand pair is both-present at the entire shared top — the
/// correlated-population loading of the party fold's up-front overlap test
/// (each input pays its both-present nodes times a search of the accumulator's
/// table) and of the reduction's merges (interleaved region sets that coalesce
/// only at the last level).
///
/// Layout: `m − 1` top tags `11` in preorder, then per block `L` path tags
/// (`10` toward a left slot, `01` toward a right one) and the owned tip `00`.
/// Normal form: no node has two fully-owned children (every `00` sits under a
/// single-child tag) and none has two absent children.
///
/// # Panics
///
/// Panics if `n` is not a power of two at least 2, `m` is not a power
/// of two, or `i ≥ n`.
fn stagger_id(n: usize, m: usize, i: usize) -> Packed {
    assert!(
        n >= 2 && n.is_power_of_two(),
        "the staggered id needs a power-of-two operand count"
    );
    assert!(
        m >= 1 && m.is_power_of_two(),
        "the staggered id needs a power-of-two block count"
    );
    assert!(i < n, "the operand index addresses one of the n slots");
    let levels = n.trailing_zeros();
    fn path(bits: &mut BitsMut, levels: u32, i: usize, t: u32) {
        if t == levels {
            bits.push(false); // the owned slot: terminal tag "00"
            bits.push(false);
            return;
        }
        let right = (i >> (levels - 1 - t)) & 1 == 1;
        bits.push(!right); // left child present iff the slot sits left
        bits.push(right); // right child present iff it sits right
        path(bits, levels, i, t + 1);
    }
    fn top(bits: &mut BitsMut, levels: u32, i: usize, m: usize) {
        if m == 1 {
            path(bits, levels, i, 0);
            return;
        }
        bits.push(true); // top tag: both halves hold owned slots
        bits.push(true);
        top(bits, levels, i, m / 2);
        top(bits, levels, i, m / 2);
    }
    let mut bits = BitsMut::with_capacity(m * (2 * levels as usize + 4) - 2);
    top(&mut bits, levels, i, m);
    Packed::from_bits(bits)
}

/// The staggered fold population `(versions, ids)`: all `n`
/// [`stagger_comb`]/[`stagger_id`] operands in bit-reversed feed order.
///
/// The feed order is the population's second load-bearing axis: the balanced
/// binary-counter reduction merges arrival-adjacent inputs first, and
/// bit-reversing the operand indices makes every merged pair's slot indices
/// diverge at their *most* significant bit — so every internal merge, at every
/// level, joins region sets that interleave maximally and swell to near the sum
/// of their sizes. Index order instead hands the counter pairs that diverge at
/// the last bit (near-adjacent slots, maximal path sharing), the coalescing
/// luck the wedge exists to foreclose.
///
/// # Panics
///
/// [`stagger_comb`]'s parameter contract.
fn stagger_population(n: usize, m: usize) -> (Vec<Packed>, Vec<Packed>) {
    assert!(
        n >= 2 && n.is_power_of_two(),
        "the staggered population needs a power-of-two operand count"
    );
    let bits = n.trailing_zeros();
    let order = (0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits));
    order
        .map(|i| (stagger_comb(n, m, i), stagger_id(n, m, i)))
        .unzip()
}

/// The meet-shade population `MS(d, k)`: one deep carrier, then `k − 1`
/// byte-equal plateau shades that dominate it — the meet fold's
/// non-shrinking-accumulator adversary.
///
/// The dual of the join wedges, derived from the meet's actual walk: a join
/// fold's hazard is an accumulator that *grows* without coalescing, a meet
/// fold's is one that never *shrinks*. The carrier is [`dense`]`(d)` (heights 0
/// and 1, the node-density maximizer); each shade is [`hugeleaf`]`(2)` — the
/// constant-3 skyline, one leaf, 6 packed bits — sitting strictly above the
/// carrier everywhere. The running meet is therefore the carrier,
/// byte-identical, at every step: `acc ∧ shade` re-walks the whole carrier (the
/// emission sweep visits every boundary of the overlay and the carrier supplies
/// `Θ(d)` of them), and no short-circuit applies — the shade is never
/// byte-equal to the accumulator, never empty, and the accumulator never
/// empties. A sequential reduce therefore pays `Θ(k · d)` on a `Θ(d + k)`-byte
/// population, while the balanced reduction re-walks the carrier once per
/// counter level, `O(d log k + k)`. Neither operand shape is adversarial alone
/// — both are committed linear families. The committed flatness band and the
/// sequential-reduce adequacy tripwire (`meet_all_shade_is_flat_per_unit` and
/// `sequential_meet_reduce_reads_superlinear_on_shade` in `tests/meter.rs`)
/// carry both measured readings; the population's semantic leg is exactness —
/// the fold returns the carrier.
///
/// Each shade is built independently: byte-equal streams in *distinct* buffers,
/// so the equal-shade combines are answered by the byte compare the band
/// describes — never by the folds' clone-identity collapse, which would drop
/// the shades before the fold reads them and turn the flatness band into a
/// measurement of the collapse instead. The collapse has its own liveness pin
/// (`identity_fast_paths` in `tests/meter.rs`).
///
/// # Panics
///
/// Panics if `d == 0` or `k < 2` (the fold needs the carrier and at least one
/// shade).
fn meet_shade(d: usize, k: usize) -> Vec<crate::Version> {
    assert!(d >= 1, "the meet shade needs a nonzero carrier depth");
    assert!(k >= 2, "the meet shade needs at least one shade");
    let carrier = dense(d).version();
    let mut population = Vec::with_capacity(k);
    population.push(carrier);
    for _ in 1..k {
        population.push(hugeleaf(2).version());
    }
    population
}

/// The masked-comparison correlated triple `MT(k, n)`: a boundary comb, a mask
/// owning every other tooth, and a wide plateau — the three-stream fused
/// comparison's adversary, each operand benign alone.
///
/// Returns `(event, id, event)`: [`cliff_comb`]`(k, n)` (the masked operand),
/// [`scattered_id`]`(n / 2)` (the mask, whose owned fragments sit exactly at
/// the comb's even tooth positions), and a single-leaf plateau at `2^k` (the
/// unmasked right operand, `2k + 2` bits). The correlation is the point — every
/// operand is a certified-linear genre by itself (the comb, the scattered id, a
/// hugeleaf-class plateau), and the heat exists only in the composition:
/// comparing `(comb / mask) ⋚ plateau` toggles ownership at every tooth
/// boundary, so the walk alternates between reading the difference `D = h_comb
/// − 2^k` inside owned teeth — a near-zero value spelled by cancelling wide
/// digits, oscillating across the `2^k` carry boundary behind 3-bit stored
/// deltas — and the zero-check `sign(h_plateau)` on unowned intervals. An
/// integrator that materializes either read pays `Θ(k)` limb work per 3-bit
/// code; the balanced signed-digit accumulator answers both in amortized O(1)
/// touches (the envelope rows and the flatness band in `tests/meter.rs` hold it
/// there). The verdict is `Less` — the projected comb sits under the plateau
/// everywhere and strictly under it outside the mask — so the walk never exits
/// early and the measurement prices the whole overlay.
///
/// # Panics
///
/// Panics if `k == 0`, or `n` is not an even count of at least 2 (the
/// mask owns every other tooth).
fn mask_drift_triple(k: usize, n: usize) -> (Packed, Packed, Packed) {
    assert!(k >= 1, "the mask-drift triple needs a nonzero magnitude");
    assert!(
        n >= 2 && n.is_multiple_of(2),
        "the mask-drift triple needs an even tooth count"
    );
    let mut plateau = BitsMut::with_capacity(2 * k + 2);
    ev_leaf_wide(&mut plateau, &pow2(k));
    (
        cliff_comb(k, n),
        scattered_id(n / 2),
        Packed::from_bits(plateau),
    )
}

/// The masked-comparison correlated quadruple `MQ(k, n)`: two comb/mask pairs
/// whose ownership parities interleave — the four-stream fused comparison's
/// adversary, each operand benign alone.
///
/// Returns `((event₁, id₁), (event₂, id₂))`: the sparse comb (teeth at odd
/// levels only, plain zero leaves at even levels, `(n/2)(2k + 14) + 2` bits)
/// under [`scattered_id`]`(n / 2)` (owning the even levels — exactly where its
/// event is zero), against the full [`cliff_comb`]`(k, n)` under the offset
/// mask (owning the odd levels — exactly where its event's teeth stand). Every
/// tooth boundary is a double mask toggle with the parities out of phase, so
/// the walk rotates through its ownership cases: even-level teeth read
/// `sign(h₁)` — the trichotomy's zero-check on a height that is zero
/// *semantically* but spelled by cancelling `2^k`-wide digits, each odd tooth's
/// climb and drop funded by its own wide codes — and odd-level teeth read
/// `sign(h₂)` mid-oscillation across the carry boundary. The projected verdict
/// is `Less` (view₁ is semantically empty; view₂ keeps its teeth), so the walk
/// never exits early. The envelope rows and the flatness band in
/// `tests/meter.rs` hold the composition linear.
///
/// # Panics
///
/// Panics if `k == 0`, or `n` is not an even count of at least 2.
fn mask_drift_quadruple(k: usize, n: usize) -> ((Packed, Packed), (Packed, Packed)) {
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

/// The sparse boundary comb: [`cliff_comb`]'s spine with teeth at odd levels
/// only and a plain zero leaf at each even level, `(n/2)(2k + 14) + 2` bits.
///
/// Layout per level pair: `"11" · "01"` (even level: spine node, zero left
/// leaf), then `"11" · "1" · gamma(2^k − 1) · "01" · "0010"` (odd level: spine
/// node and the comb's tooth), after all `n` levels `"01"` (the terminal spine
/// leaf). Normal form holds as the comb's: every spine node's zero-base leaf
/// child carries its subtree minimum, and the only sibling leaf pairs are the
/// teeth's `(0, 1)`.
fn sparse_cliff_comb(k: usize, n: usize) -> Packed {
    debug_assert!(k >= 1 && n >= 2 && n.is_multiple_of(2));
    let mut bits = BitsMut::with_capacity((n / 2) * (2 * k + 14) + 2);
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

/// The offset scattered id: [`scattered_id`]'s alternation shifted one level
/// down — a gap level, then an owned left subtree, repeated — `6e + 4` bits.
///
/// Layout, repeated `e` times: `01` (a right-only gap level), `11` (both
/// children present), `00` (the owned left leaf); terminated by `01 · 00` (a
/// final gap level whose right child is the owned tip). Owns exactly the *odd*
/// levels' left subtrees of a right-leaning spine — the complement, tooth for
/// tooth, of [`scattered_id`]'s even-level fragments. Normal form: no node has
/// two fully-owned children (each `11` node's right child is a gap node or the
/// final gap level) and no node has two absent children.
fn scattered_id_offset(e: usize) -> Packed {
    debug_assert!(e >= 1);
    let mut bits = BitsMut::with_capacity(6 * e + 4);
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

/// The masked-hole triple `MH(d, h)`: a deep dense spine under a shallow
/// diverted mask, compared against a dominating plateau — the fused
/// three-stream comparison's depth-independence adversary.
///
/// Returns `(spine, mask, plateau)`: the dense spine [`dense`]`(d)`, the
/// diverted id spine [`id_spine`]`(h, true)`, and a single leaf of value 2.
/// The mask owns exactly one leaf at depth `h` and leaves the spine's whole
/// continuation below depth `h` as one unowned region whose `Θ(d)`
/// boundaries no other cursor crosses: the masked walk's block skip must
/// consume that run as one block, so the comparison's accumulator work is a
/// function of `h` alone, however deep the spine grows. The plateau
/// strictly dominates every spine height, so the projected verdict is
/// `Less` at every elementary interval and the walk never exits early. The
/// `masked_cmp_hole` envelope and the `masked_cmp_hole_depth_band` band in
/// `tests/meter.rs` pin the flatness across a spine-depth doubling — the
/// reading a per-boundary walk cannot reproduce.
///
/// # Panics
///
/// Panics if `h < 2` (the mask needs a unary run to divert from) or if
/// `d <= h` (the spine must outrun the mask, or no deep run exists).
fn masked_hole(d: usize, h: usize) -> (Packed, Packed, Packed) {
    assert!(
        h >= 2,
        "the masked hole's mask needs a unary run to divert from"
    );
    assert!(d > h, "the masked hole's spine must outrun its mask");
    let mut plateau = BitsMut::with_capacity(4);
    ev_leaf(&mut plateau, 2); // dominates every spine height (they are 0 or 1)
    (dense(d), id_spine(h, true), Packed::from_bits(plateau))
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
/// segments the stack guard allocates never pass through the global allocator,
/// so no heap meter can see them; this reads the counter bumped at the one
/// place a segment is created. Process-global — meaningful per scenario only
/// under one-scenario-per-process isolation (nextest's model) or a
/// single-threaded caller.
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
/// can see: a magnitude blowup allocates little and visits no extra nodes — the
/// work is wider, not more frequent. The count is the operands' 64-bit limb
/// counts per `Base` operation (arithmetic, comparison, equality, and hashing;
/// a widening left shift records its output width, operand plus shifted-in
/// limbs, so a shift-and-discard loop cannot read near-zero) plus one
/// value-width record per wide-gamma decode, so an amortized-linear algorithm
/// counts linearly in packed input bits and a magnitude-quadratic one counts
/// quadratically. Process-global, same isolation requirement as
/// [`stack_segments`]; only compiled under the `limb-meter` feature, which adds
/// the counting to the arithmetic itself.
#[cfg(feature = "limb-meter")]
pub fn limb_ops() -> u64 {
    crate::codec::limb_meter::limb_ops()
}

/// Reset the limb-operation counter behind [`limb_ops`] to zero.
#[cfg(feature = "limb-meter")]
pub fn reset_limb_ops() {
    crate::codec::limb_meter::reset()
}

/// The settle's densified-image digits zero-filled since the last
/// [`reset_densified_digits`].
///
/// The deterministic stand-in for allocation-fill work, which no other meter
/// can see: the query folds' settle densifies each balanced-digit cluster
/// into two zero-filled byte images before multiplying, and a zeroed byte no
/// digit lands on enters no operand width (the limb column's proxy), touches
/// no accumulator digit, and raises no peak while the image stays under the
/// walk's own high-water mark. The count is the images' capacity in
/// base-2^32 digits — two images per multi-digit cluster, each at the
/// cluster's span — so span-priced densification counts linearly in the
/// settle's cluster spans, and a densification sized by cluster *positions*
/// counts by those positions instead (the axis the hoisted-window family
/// isolates). Process-global, same isolation requirement as
/// [`stack_segments`]; only compiled under the `limb-meter` feature.
#[cfg(feature = "limb-meter")]
pub fn densified_digits() -> u64 {
    crate::codec::limb_meter::densified_digits()
}

/// Reset the densified-image counter behind [`densified_digits`] to zero.
#[cfg(feature = "limb-meter")]
pub fn reset_densified_digits() {
    crate::codec::limb_meter::reset_densified()
}

/// The accumulator digit touches since the last [`reset_touch_ops`].
///
/// The deterministic stand-in for accumulator *fold* work, which the limb
/// counter no longer sees on narrow values: a word-scale fold rides the
/// accumulator's quick register (one touch, no `Base` arithmetic, no digit
/// traffic), so an algorithm that folds per leaf where another folds per block
/// separates here even when both read zero limb operations. Delegates to
/// `suanpan`'s own counter (`suanpan::touch_meter`), which the `limb-meter`
/// feature compiles in. Process-global, same isolation requirement as
/// [`stack_segments`].
#[cfg(feature = "limb-meter")]
pub fn touch_ops() -> u64 {
    suanpan::touch_meter::touches()
}

/// Reset the digit-touch counter behind [`touch_ops`] to zero.
#[cfg(feature = "limb-meter")]
pub fn reset_touch_ops() {
    suanpan::touch_meter::reset()
}

/// The pair-hull ladder's rung counters since the last
/// [`reset_span_traffic`]: how many span constructions each fast path answered,
/// and how many reached the emitting walk.
///
/// The deterministic stand-in for a *consumer's* traffic mix, which no
/// per-operation envelope can see: whether a workload's pair hulls are mostly
/// comparable (hand-back at one comparison sweep) or mostly concurrent (the
/// emitting walk) is a property of the caller's pairs, and it decides which
/// kernel regime the consumer actually pays. Counts every pair-hull
/// construction: every [`Version::span`](crate::Version::span), every leaf
/// combine of `span_all`, and every point-combine of the span union doors
/// (`Span | Span` and [`Span::union_all`](crate::Span::union_all) on coincident
/// operands), which derive their hull through the same kernel. Process-global,
/// same isolation requirement as [`stack_segments`].
pub fn span_traffic() -> SpanTraffic {
    crate::version::hull_traffic::snapshot()
}

/// Reset the rung counters behind [`span_traffic`] to zero.
pub fn reset_span_traffic() {
    crate::version::hull_traffic::reset()
}

/// The fill walk's priced-offset domination decisions since the last
/// [`reset_emit_traffic`]: how many word-scale emissions each of the
/// watermark web's no-fold arms answered, and how many fell back to the
/// fold path.
///
/// The deterministic stand-in for emission-arm *liveness*, which no cost
/// meter can see: the dominated arms and the fold path compute the same
/// values at nearby costs, so a routing change that quietly re-routes a
/// family's emissions off a fast arm leaves every differential green and
/// every cost band near its pin while the arm goes undriven — exactly the
/// regression this counter's committed floors catch (the
/// `dominated-undercut` family's band in `tests/meter.rs`).
/// Process-global, same isolation requirement as [`stack_segments`].
pub fn emit_traffic() -> EmitTraffic {
    crate::version::skyline::web_traffic::snapshot()
}

/// Reset the decision counters behind [`emit_traffic`] to zero.
pub fn reset_emit_traffic() {
    crate::version::skyline::web_traffic::reset()
}

/// The anchor web's pool misses since the last [`reset_pool_misses`]:
/// leases the accumulator pool could not serve.
///
/// The deterministic stand-in for steady-state churn allocation, which no
/// other meter can see: the web recycles dying accumulators through a pool,
/// and a dead recycle changes no peak-heap reading (each dropped buffer is
/// released before the fresh allocation replacing it) and no touch or limb
/// reading (a fresh accumulator folds exactly like a reset one) — only the
/// miss count separates a pool that recycles (misses bounded by peak
/// simultaneous demand) from one that leaks its churn (misses proportional
/// to it). The seam-stop pool row in `tests/meter.rs` pins both directions.
/// Process-global, same isolation requirement as [`stack_segments`]; only
/// compiled under the `limb-meter` feature.
#[cfg(feature = "limb-meter")]
pub fn pool_misses() -> u64 {
    crate::version::skyline::pool_traffic::misses()
}

/// Reset the pool-miss counter behind [`pool_misses`] to zero.
#[cfg(feature = "limb-meter")]
pub fn reset_pool_misses() {
    crate::version::skyline::pool_traffic::reset()
}

/// The packed-stream bits scanned and written since the last
/// [`reset_scan_bits`].
///
/// The deterministic stand-in for traversal work over the packed forms, which
/// every other meter can miss at once: an id-tree fold allocates little (no
/// heap delta), loops rather than recurses (no segments), and does no `Base`
/// arithmetic (no limb operations) — the work is *reading and writing stream
/// bits*, and this counter records exactly those, at the packed-stream
/// primitives (id tag reads and skip steps, id-builder bit writes and splice
/// lengths, event topology cursor advances and gamma code-skips, every
/// sequential decoder/validator bit read). Unit: bits. Process-global, same
/// isolation requirement as [`stack_segments`]; only compiled under the
/// `scan-meter` feature, which adds the counting to the primitives themselves.
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
