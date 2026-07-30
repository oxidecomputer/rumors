//! Input generators for the property tests, in two families:
//!
//! - **Adversarial deep shapes** ([`Shape`], [`shape_party`]/[`shape_version`],
//!   [`skip_stress_pair`], [`deep_left_spine_party`]) — the deep, unbalanced
//!   trees that are the worst case for any traversal locating a right child by
//!   re-scanning its left subtree. Each is parameterized by a `scale` knob
//!   linear in the node count, so the deep differentials can drive real depth
//!   at chosen sizes. (Asymptotic traversal cost itself is enforced elsewhere:
//!   the amplification board's scan column and the fuzzfit fuel bands.)
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
//!
//! A different instrument entirely from `crate::meter`'s generators: those
//! are hand-derived worst-case *encodings* with closed-form sizes, built
//! for the resource-envelope pins and the amplification board; these are
//! proptest strategies over random inputs.

use proptest::prelude::*;

use crate::codec;
use crate::oracle;
use crate::{Party, Version};

use super::bridge::{from_oracle_party, from_oracle_version};

// ───────────────────────────── adversarial deep shapes ─────────────────────────────

/// A deep tree shape.
///
/// The spines (depth linear in `scale`) stress right-child
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

/// A random deep shape for the deep-operand differentials.
pub(crate) fn arb_shape() -> impl Strategy<Value = Shape> {
    prop_oneof![
        Just(Shape::LeftSpine),
        Just(Shape::RightSpine),
        Just(Shape::Zigzag),
        Just(Shape::Bushy),
    ]
}

/// Build a balanced-ish bushy event tree over `leaves` distinct-based leaves,
/// numbered from `lo` (so no two siblings collapse).
///
/// Splitting an odd count unevenly gives leaves at varying depths. Recursive
/// over a `O(log)` depth (test-only; the impl is iterative).
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
/// sit at varying depths.
///
/// Recursive over a `O(log)` depth (test-only; the impl is iterative).
fn bushy_party(lo: usize, leaves: usize) -> oracle::Party {
    use oracle::Party as P;
    if leaves <= 1 {
        return P::Leaf(lo.is_multiple_of(2)); // even index owned, odd empty
    }
    let half = leaves / 2;
    P::node(bushy_party(lo, half), bushy_party(lo + half, leaves - half))
}

/// A bushy id rooted beside one owned terminal: `(bushy(scale), 1)`.
///
/// The expansion-heavy shape: the bushy left subtree makes the tick
/// walk's route fold weigh two feasible children at every branch, while
/// the right terminal is the cheapest inflation at every scale — so the
/// splice's chosen path (and its one skip of the whole off-path bushy
/// subtree) is scale-independent.
pub(crate) fn bushy_expand_party(scale: usize) -> Party {
    use oracle::Party as P;
    from_oracle_party(&P::node(bushy_party(0, scale + 1), P::Leaf(true)))
}

/// Build a normal-form event tree of `shape` sized linearly in `scale`.
///
/// The spines have `scale` internal nodes (`2*scale + 1` nodes total); the
/// bushy shape has `~scale` leaves. Distinct leaf bases prevent collapse,
/// preserving the shape and size.
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
/// each over a small subtree.
///
/// `b` is a right-spine whose every left child is a 2-leaf subtree `(1, 0)`;
/// `a` is a right-spine of `0`-leaf left children. In lockstep, at every one of
/// the `scale` levels `a`'s left `0`-leaf aligns against `b`'s left *subtree*,
/// so that subtree is skipped once. The pair is disjoint (`a` owns only its
/// deepest-right tip, `b` owns its left subtrees and deepest-left tip), so the
/// walk runs to completion (no early `false`) and every level's skip is
/// exercised. Both ids are linear in `scale`.
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

/// Build a non-empty normal-form id of `shape` sized linearly in `scale`.
///
/// The spines carry a single owned region (a `1` leaf at the tip) with `0`
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
/// bits, with a single owned region at the deep-left tip.
///
/// Used by the stack-safety test, which needs structures far deeper than the
/// recursive oracle bridge (`emit_id`) or the oracle's own recursive `Drop`
/// could build or tear down. In the pruned encoding each spine node is a
/// `Left-only` tag (`10`: left child present, right absent — the `0` right
/// children take no bits), and the deep-left tip is a terminal (`00`). The
/// result `(((…(1, 0)…), 0), 0)` is normal form (no node has two terminal
/// children). Built with a flat loop: no recursion at any depth, in the builder
/// or in `Drop` (the packed form is a flat `BitVec`).
pub(crate) fn deep_left_spine_party(depth: usize) -> Party {
    let mut bits = codec::BitsMut::with_capacity(2 * depth + 2);
    for _ in 0..depth {
        bits.push(true); // Left-only tag `10`: left child present ...
        bits.push(false); //   ... right child absent
    }
    bits.push(false); // terminal tag `00`: the deep-left owned tip
    bits.push(false);
    Party::from_bits(bits)
}

// ───────────────────────── arbitrary normal-form ─────────────────────────
//
// Base magnitudes deliberately span small values AND values near/beyond
// `u64::MAX`: this is the natural home for the path-sum-overflow regression
// class (path sums that would overflow a `u64`). With arbitrary-precision
// `Base` values the impl threads them losslessly, so the large-base
// differentials must agree with the oracle exactly.

/// Recursion-depth cap for the arbitrary generators.
///
/// Kept small so the default proptest run stays CI-cheap while still covering
/// every arm; deeper coverage is the job of the (ignored) exhaustive variant
/// and the deep-tree stack-safety test.
const ARB_DEPTH: u32 = 4;

/// Branching budget for the arbitrary generators: the expected interior-node
/// count, which bounds how bushy a generated tree gets.
const ARB_NODES: u32 = 16;

/// An arbitrary event base magnitude.
///
/// Mixes a dense small range (where collapses and `one_zero` corners live) with
/// values straddling `u64::MAX`, so a generated event tree can have
/// root-to-leaf path sums that would overflow `u64`. The big-value arms are
/// built from `u128` conversions and shifted powers, well beyond `u64`. The
/// `2^64`-aligned arm produces small nonzero multiples of `2^64`: values (and
/// differences of two draws — the fused tick's raise offsets) whose low limb
/// is exactly zero, the class a limb-truncated value comparison misreads as
/// zero. The fill flag's full-width worked witnesses pin that comparison
/// pointwise; this arm keeps the class under ongoing generator mass.
pub(crate) fn arb_base() -> impl Strategy<Value = codec::Base> {
    prop_oneof![
        6 => (0u64..6).prop_map(codec::Base::from),
        2 => any::<u64>().prop_map(codec::Base::from),
        1 => (u64::MAX - 4..=u64::MAX).prop_map(codec::Base::from),
        1 => any::<u128>().prop_map(|n| codec::Base::from(n) + codec::Base::from(u64::MAX)),
        1 => (0u32..96).prop_map(|k| (codec::Base::from(1u8) << k) + codec::Base::from(1u8)),
        1 => (1u64..8).prop_map(|k| codec::Base::from(k) << 64u32),
    ]
}

/// An arbitrary normal-form id tree (may be the anonymous `Leaf(false)`).
///
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
/// (owns at least one region).
///
/// Filters out the anonymous tree so the impl bridge and ops that require a
/// real share (fork/join) get a meaningful input.
pub(crate) fn arb_oracle_party_nonempty() -> impl Strategy<Value = oracle::Party> {
    arb_oracle_party().prop_filter("non-anonymous id", |p| !p.is_empty())
}

/// An arbitrary normal-form event tree.
///
/// Random recursive shape with random base magnitudes from [`arb_base`]
/// (including values near/beyond `u64::MAX`); every interior node goes through
/// the oracle's normalizing `Version::node`, so the result is always in normal
/// form (a zero-base child at every node, no collapsible `(n, m, m)`).
pub(crate) fn arb_oracle_version() -> impl Strategy<Value = oracle::Version> {
    let leaf = arb_base().prop_map(oracle::Version::Leaf);
    leaf.prop_recursive(ARB_DEPTH, ARB_NODES, 2, |inner| {
        (arb_base(), inner.clone(), inner).prop_map(|(n, l, r)| oracle::Version::node(n, l, r))
    })
}

// ───────────────────────── variadic-law families ─────────────────────────

/// A list arity for the variadic law drivers, swept past every
/// structural boundary of the balanced binary counter every n-ary fold
/// runs on ([`crate::fold`]).
///
/// The counter's behavior over `k` inputs changes only at these
/// boundaries, derived from its structure:
///
/// - `k = 0` and `k = 1`: the identity and lone-input short-circuits —
///   no combine runs at all;
/// - `k = 2`: the first in-counter combine, of two raw inputs (the leaf
///   arm);
/// - `k = 3`: the first closing-drain combine (the drain performs
///   `popcount(k) - 1` combines, so it first runs here), pairing a
///   merged group with a lone raw input;
/// - `k = 4`: the first merged–merged combine — two weight-1 groups
///   carrying inside the counter — the arm beyond every fixed arity-3
///   law signature;
/// - `k = 6`: the first *drain* combine of two merged groups (surviving
///   weights 2 and 1);
/// - `k = 2^j` and `k = 2^j + 1`: each octave carries one weight deeper
///   (a chain of `j` in-counter combines), then leaves a lone raw input
///   under the deep group for the drain.
///
/// Every combine-arm *genre* the folds dispatch on (leaf,
/// merged–input, merged–merged; in-counter and drain) is reachable by
/// `k = 6`. The band `0..=9` covers each genre plus the first full
/// octave boundary (8, 9); the band `15..=17` crosses the next octave
/// (15 = 0b1111 drains four groups through three combines, 16 carries
/// to a single weight-4 group, 17 leaves a lone input under it), so
/// behavior keyed to a particular *weight* rather than a genre still
/// meets two octaves of weights.
pub(crate) fn arb_fold_arity() -> impl Strategy<Value = usize> {
    prop_oneof![
        3 => 0usize..=9,
        1 => 15usize..=17,
    ]
}

/// A small pool of arbitrary versions with the empty version always
/// present.
///
/// Variadic-law lists are built by *indexing* into a pool this small,
/// so repeats and shared-structure elements arise naturally at every
/// arity. Repeats matter: repeated raw inputs coalesce into counter
/// groups that each carry information their partners lack in both
/// lattice directions, so an arm that drops or misreads a merged
/// operand loses a fresh input and diverges — where lattice-derived
/// items would be absorbed and leave the misread invisible. The empty
/// version keeps the folds' `O(1)` identity short-circuits under mass.
fn arb_version_pool() -> impl Strategy<Value = Vec<oracle::Version>> {
    proptest::collection::vec(arb_oracle_version(), 1..=3).prop_map(|mut pool| {
        pool.push(oracle::Version::new());
        pool
    })
}

/// Draws from `pool` for one receiver and a boundary-swept
/// ([`arb_fold_arity`]) list of items.
fn family_picks<T: Clone + core::fmt::Debug>(pool: Vec<T>) -> impl Strategy<Value = (T, Vec<T>)> {
    (
        any::<prop::sample::Index>(),
        arb_fold_arity()
            .prop_flat_map(|arity| proptest::collection::vec(any::<prop::sample::Index>(), arity)),
    )
        .prop_map(move |(receiver, picks)| {
            (
                pool[receiver.index(pool.len())].clone(),
                picks
                    .into_iter()
                    .map(|pick| pool[pick.index(pool.len())].clone())
                    .collect(),
            )
        })
}

/// A pool-indexed version family — a receiver and a boundary-swept
/// list of items — for the variadic version-law drivers.
///
/// The receiver is drawn from the same pool as the items, so
/// receiver-repeats (an input aliasing the fold's seed) arise
/// naturally too. Arity per [`arb_fold_arity`]; pool per
/// [`arb_version_pool`].
pub(crate) fn arb_version_family() -> impl Strategy<Value = (oracle::Version, Vec<oracle::Version>)>
{
    arb_version_pool().prop_flat_map(family_picks)
}

/// A pool-indexed party family — a receiver and a boundary-swept list
/// of items — for the variadic party-law drivers.
///
/// The pool holds live (non-anonymous) parties only, the admissible
/// inputs of every party law. Repeats are *aliases* — regions
/// overlapping byte-identically — which is exactly the input class the
/// fallible folds' rejection paths exist for, so pool indexing keeps
/// the refusal arm under mass at every arity.
pub(crate) fn arb_party_family() -> impl Strategy<Value = (oracle::Party, Vec<oracle::Party>)> {
    proptest::collection::vec(arb_oracle_party_nonempty(), 1..=3).prop_flat_map(family_picks)
}

/// A pool-indexed clock family — a receiver and a boundary-swept list
/// of items, each a canonical party/version pairing — for the variadic
/// clock-law drivers.
///
/// Parties and versions are drawn from independent small pools and
/// paired combinatorially (every canonical pairing is a valid clock,
/// including ones no op sequence reaches); party repeats give aliased
/// clocks for the refusal arm, and the version pool's empty element
/// keeps fresh-line clocks under mass.
pub(crate) fn arb_clock_family() -> impl Strategy<
    Value = (
        (oracle::Party, oracle::Version),
        Vec<(oracle::Party, oracle::Version)>,
    ),
> {
    (
        proptest::collection::vec(arb_oracle_party_nonempty(), 1..=3),
        arb_version_pool(),
    )
        .prop_flat_map(|(parties, versions)| {
            let pool: Vec<(oracle::Party, oracle::Version)> = parties
                .iter()
                .flat_map(|p| versions.iter().map(move |v| (p.clone(), v.clone())))
                .collect();
            family_picks(pool)
        })
}
