//! Input generators for the property tests, in two families:
//!
//! - **Adversarial deep shapes** ([`Shape`], [`shape_party`]/[`shape_version`],
//!   the stress-pair builders, [`deep_left_spine_party`]) — the deep, unbalanced
//!   trees that are the worst case for any traversal locating a right child by
//!   re-scanning its left subtree. Each is parameterized by a `scale` knob linear
//!   in the node count, so the complexity proptests can build at `scale` and `4 *
//!   scale` and assert near-linear step growth.
//!
//! - **Arbitrary normal-form** ([`arb_base`], [`arb_oracle_party`],
//!   [`arb_oracle_version`]) — random recursive shapes with random base
//!   magnitudes (including values near/beyond `u64::MAX`), pushed through the
//!   oracle's normalizing constructors so they are always valid normal form.
//!   These break the op-trace generator's coupling (which only ever produces
//!   causally *related* pairs of the shapes operations build).
//!
//! All trees are built via the oracle's normalizing constructors (`O(1)` per
//! node), then lowered to the impl with [`super::bridge`].

use proptest::prelude::*;

use crate::codec;
use crate::oracle;
use crate::{Party, Version};

use super::bridge::{from_oracle_party, from_oracle_version};

// ───────────────────────────── adversarial deep shapes ─────────────────────────────

/// A deep tree shape. The spines (depth linear in `scale`) stress right-child
/// location; the bushy shape stresses multi-region cost comparisons (a node
/// whose two children are both feasible), which the spines — with a single
/// owned leaf — never produce.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Shape {
    /// Every node leans left: `(…((·,·),·)…,·)`.
    LeftSpine,
    /// Every node leans right.
    RightSpine,
    /// Alternating left/right lean.
    Zigzag,
    /// A balanced-ish bushy tree: many leaves at varying depths, so an id built from it
    /// has multiple genuinely feasible owned regions (see [`shape_party`]).
    Bushy,
}

/// A random deep shape for the complexity proptests.
pub(crate) fn arb_shape() -> impl Strategy<Value = Shape> {
    prop_oneof![
        Just(Shape::LeftSpine),
        Just(Shape::RightSpine),
        Just(Shape::Zigzag),
        Just(Shape::Bushy),
    ]
}

/// Build a balanced-ish bushy event tree over `leaves` distinct-based leaves,
/// numbered from `lo` (so no two siblings collapse). Splitting an odd count
/// unevenly gives leaves at varying depths. Recursive over a `O(log)` depth
/// (test-only; the impl is iterative).
fn bushy_version(lo: u64, leaves: usize) -> oracle::Version {
    use oracle::Version as V;
    if leaves <= 1 {
        return V::leaf(lo);
    }
    let half = leaves / 2;
    V::node(
        0u64,
        bushy_version(lo, half),
        bushy_version(lo + half as u64, leaves - half),
    )
}

/// Build a balanced-ish bushy id over `leaves` leaves with bases alternating
/// `1`/`0`, so adjacent leaves never collapse and multiple owned (`1`) regions
/// sit at varying depths. Recursive over a `O(log)` depth (test-only; the impl
/// is iterative).
fn bushy_party(lo: usize, leaves: usize) -> oracle::Party {
    use oracle::Party as P;
    if leaves <= 1 {
        return P::Leaf(lo.is_multiple_of(2)); // even index owned, odd empty
    }
    let half = leaves / 2;
    P::node(bushy_party(lo, half), bushy_party(lo + half, leaves - half))
}

/// Build a normal-form event tree of `shape` sized linearly in `scale`. The
/// spines have `scale` internal nodes (`2*scale + 1` nodes total); the bushy
/// shape has `~scale` leaves. Distinct leaf bases prevent collapse, preserving
/// the shape and size.
pub(crate) fn shape_version(shape: Shape, scale: usize) -> Version {
    use oracle::Version as V;
    if let Shape::Bushy = shape {
        return from_oracle_version(&bushy_version(0, scale + 1));
    }
    let mut t = V::leaf(0u64);
    for k in 1..=scale as u64 {
        let leaf = V::leaf(k);
        t = match shape {
            Shape::LeftSpine => V::node(0u64, t, leaf),
            Shape::RightSpine => V::node(0u64, leaf, t),
            Shape::Zigzag if k % 2 == 0 => V::node(0u64, t, leaf),
            Shape::Zigzag => V::node(0u64, leaf, t),
            Shape::Bushy => unreachable!("handled above"),
        };
    }
    from_oracle_version(&t)
}

/// Build a disjoint "staircase" id pair `(a, b)` that drives the bounded
/// lazy-skip in `is_disjoint` to its worst case: `Θ(scale)` distinct skips,
/// each over a small subtree. `b` is a right-spine whose every left child is a
/// 2-leaf subtree `(1, 0)`; `a` is a right-spine of `0`-leaf left children. In
/// lockstep, at every one of the `scale` levels `a`'s left `0`-leaf aligns
/// against `b`'s left *subtree*, so that subtree is skipped once. The pair is
/// disjoint (`a` owns only its deepest-right tip, `b` owns its left subtrees
/// and deepest-left tip), so the walk runs to completion (no early `false`) and
/// the cumulative skip cost is measured. Both ids are linear in `scale`. With a
/// *bounded* skip the total is `O(scale)`; an *unbounded* (rescanning) skip
/// would be `O(scale²)` — which the linear-scaling assertion would catch.
pub(crate) fn skip_stress_pair(scale: usize) -> (Party, Party) {
    use oracle::Party as P;
    // A 2-leaf subtree `(1, 0)`: a small node that owns its left half.
    let owned_left = || P::node(P::seed(), P::Leaf(false));
    // `b`: right-spine, each left child a small owned subtree, deepest-right
    // tip empty.
    let mut b = P::Leaf(false);
    for _ in 0..scale {
        b = P::node(owned_left(), b);
    }
    // `a`: right-spine of `0`-leaf left children; owns only its deepest-right
    // `1` tip, which lands in `b`'s empty deepest-right region — so the pair is
    // disjoint and the walk runs to completion, skipping `b`'s left subtree
    // once at every level.
    let mut a = P::seed();
    for _ in 0..scale {
        a = P::node(P::Leaf(false), a);
    }
    (from_oracle_party(&a), from_oracle_party(&b))
}

/// Build a containment "staircase" pair `(big, small)` that drives the bounded
/// lazy-skip in `compare` to its worst case: `Θ(scale)` distinct skips. `big`
/// is a right-spine whose every left child is a `1`-leaf (owns that whole left
/// region); `small` is a right-spine whose every left child is a 2-leaf subtree
/// `(1, 0)`. In lockstep, at each level `big`'s left `1`-leaf dominates
/// `small`'s left *subtree*, so that subtree is skipped once. `big ⊇ small`, so
/// `compare` reports `Less` (ancestor) and runs to completion; the cumulative
/// skip cost is measured. Both ids are linear in `scale`; a bounded skip is
/// `O(scale)`, an unbounded one `O(scale²)`.
pub(crate) fn contain_stress_pair(scale: usize) -> (Party, Party) {
    use oracle::Party as P;
    // `big`: right-spine, every left child fully owned (`1`); deepest-right
    // empty so the spine does not collapse (`(1, 1)` would). Owns every left
    // region `small` touches.
    let mut big = P::Leaf(false);
    for _ in 0..scale {
        big = P::node(P::seed(), big);
    }
    // A 2-leaf subtree `(1, 0)`: a sub-region of `big`'s corresponding `1`.
    let owned_left = || P::node(P::seed(), P::Leaf(false));
    // `small`: right-spine, every left child a sub-region of `big`'s
    // corresponding `1`.
    let mut small = P::Leaf(false);
    for _ in 0..scale {
        small = P::node(owned_left(), small);
    }
    (from_oracle_party(&big), from_oracle_party(&small))
}

/// Build a non-empty normal-form id of `shape` sized linearly in `scale`. The
/// spines carry a single owned region (a `1` leaf at the tip) with `0`
/// off-spine; the bushy shape carries many owned regions at varying depths (so
/// a `grow` over it has nodes whose two children are both feasible, exercising
/// the multi-region cost comparison).
pub(crate) fn shape_party(shape: Shape, scale: usize) -> Party {
    use oracle::Party as P;
    if let Shape::Bushy = shape {
        return from_oracle_party(&bushy_party(0, scale + 1));
    }
    let mut t = P::seed(); // the `1` leaf
    for k in 0..scale {
        let zero = P::Leaf(false);
        t = match shape {
            Shape::LeftSpine => P::node(t, zero),
            Shape::RightSpine => P::node(zero, t),
            Shape::Zigzag if k % 2 == 0 => P::node(t, zero),
            Shape::Zigzag => P::node(zero, t),
            Shape::Bushy => unreachable!("handled above"),
        };
    }
    from_oracle_party(&t)
}

/// Build a depth-`depth` left-spine [`Party`] directly as canonical packed
/// bits, with a single owned region at the deep-left tip. Used by the
/// stack-safety test, which needs structures far deeper than the recursive
/// oracle bridge (`emit_id`) or the oracle's own recursive `Drop` could build
/// or tear down. The preorder `enc_id` stream is `depth` node flags, then
/// `Leaf(true)` (`01`), then `depth` `Leaf(false)` right children (`00`) — a
/// normal-form id (every node mixes a `true`/node child with a `false` child,
/// so nothing collapses). Built with a flat loop: no recursion at any depth, in
/// the builder or in `Drop` (the packed form is a flat `BitVec`).
pub(crate) fn deep_left_spine_party(depth: usize) -> Party {
    let mut bits = codec::Bits::with_capacity(3 * depth + 2);
    for _ in 0..depth {
        bits.push(true); // node flag
    }
    bits.push(false); // Leaf
    bits.push(true); //   value 1 (the deep-left owned tip)
    for _ in 0..depth {
        bits.push(false); // Leaf
        bits.push(false); //   value 0 (each node's right child)
    }
    Party::from_bits(bits)
}

// ───────────────────────── arbitrary normal-form ─────────────────────────
//
// Base magnitudes deliberately span small values AND values near/beyond
// `u64::MAX`: this is the natural home for the path-sum-overflow regression
// class (path sums that would overflow a `u64`). With arbitrary-precision
// `Base` values the impl threads them losslessly, so the large-base
// differentials must agree with the oracle exactly.

/// Recursion-depth cap for the arbitrary generators. Kept small so the default
/// proptest run stays CI-cheap while still covering every arm; deeper coverage
/// is the job of the (ignored) exhaustive variant and the deep-tree
/// stack-safety test.
const ARB_DEPTH: u32 = 4;

/// Branching budget for the arbitrary generators: the expected interior-node
/// count, which bounds how bushy a generated tree gets.
const ARB_NODES: u32 = 16;

/// An arbitrary event base magnitude. Mixes a dense small range (where
/// collapses and `one_zero` corners live) with values straddling `u64::MAX`, so
/// a generated event tree can have root-to-leaf path sums that would overflow
/// `u64`. The big-value arms are built from `u128`/shifted `BigUint` literals,
/// well beyond `u64`.
pub(crate) fn arb_base() -> impl Strategy<Value = codec::Base> {
    prop_oneof![
        6 => (0u64..6).prop_map(codec::Base::from),
        2 => any::<u64>().prop_map(codec::Base::from),
        1 => (u64::MAX - 4..=u64::MAX).prop_map(codec::Base::from),
        1 => any::<u128>().prop_map(|n| codec::Base::from(n) + codec::Base::from(u64::MAX)),
        1 => (0u32..96).prop_map(|k| (codec::Base::from(1u8) << k) + codec::Base::from(1u8)),
    ]
}

/// An arbitrary normal-form id tree (may be the anonymous `Leaf(false)`).
/// Random recursive shape; every interior node goes through the oracle's
/// normalizing `Party::node`, so the result is always in normal form (no
/// collapsible `(b, b)` node survives).
pub(crate) fn arb_oracle_party() -> impl Strategy<Value = oracle::Party> {
    let leaf = any::<bool>().prop_map(oracle::Party::Leaf);
    leaf.prop_recursive(ARB_DEPTH, ARB_NODES, 2, |inner| {
        (inner.clone(), inner).prop_map(|(l, r)| oracle::Party::node(l, r))
    })
}

/// An arbitrary *non-empty* normal-form id tree — a valid standalone [`Party`]
/// (owns at least one region). Filters out the anonymous tree so the impl
/// bridge and ops that require a real share (fork/join) get a meaningful input.
pub(crate) fn arb_oracle_party_nonempty() -> impl Strategy<Value = oracle::Party> {
    arb_oracle_party().prop_filter("non-anonymous id", |p| !p.is_empty())
}

/// An arbitrary normal-form event tree. Random recursive shape with random base
/// magnitudes from [`arb_base`] (including values near/beyond `u64::MAX`);
/// every interior node goes through the oracle's normalizing `Version::node`,
/// so the result is always in normal form (a zero-base child at every node, no
/// collapsible `(n, m, m)`).
pub(crate) fn arb_oracle_version() -> impl Strategy<Value = oracle::Version> {
    let leaf = arb_base().prop_map(oracle::Version::Leaf);
    leaf.prop_recursive(ARB_DEPTH, ARB_NODES, 2, |inner| {
        (arb_base(), inner.clone(), inner).prop_map(|(n, l, r)| oracle::Version::node(n, l, r))
    })
}
