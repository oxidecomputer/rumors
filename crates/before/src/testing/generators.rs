//! Property-test inputs for arbitrary valid trees and selected deep shapes.
//!
//! Arbitrary values are normalized by the recursive oracle before conversion
//! to the production representation. Deep shapes have a scale proportional to
//! their node count, so traversal properties can exercise meaningful depth.

mod tests;

use num_bigint::BigUint;
use proptest::prelude::*;

use crate::bits;
use crate::testing::oracles::tree;
use crate::{Party, Version};

use super::bridge::{from_oracle_party, from_oracle_version};

// ───────────────────────────── deep worst-case shapes ─────────────────────────────

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
fn bushy_version(lo: u64, leaves: usize) -> tree::Version {
    bushy_version_with(lo, leaves, &|k| k.into())
}

/// [`bushy_version`] with a caller-provided value for each leaf.
fn bushy_version_with(
    lo: u64,
    leaves: usize,
    leaf_value: &impl Fn(u64) -> BigUint,
) -> tree::Version {
    use tree::Version as V;
    if leaves <= 1 {
        return V::leaf(leaf_value(lo));
    }
    let half = leaves / 2;
    V::node(
        0u64,
        bushy_version_with(lo, half, leaf_value),
        bushy_version_with(lo + half as u64, leaves - half, leaf_value),
    )
}

/// Build a balanced-ish bushy id over `leaves` leaves with bases alternating
/// `1`/`0`, so adjacent leaves never collapse and multiple owned (`1`) regions
/// sit at varying depths.
///
/// Recursive over a `O(log)` depth (test-only; the impl is iterative).
fn bushy_party(lo: usize, leaves: usize) -> tree::Party {
    use tree::Party as P;
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
    use tree::Party as P;
    from_oracle_party(&P::node(bushy_party(0, scale + 1), P::Leaf(true)))
}

/// Build a normal-form event tree of `shape` sized linearly in `scale`.
///
/// The spines have `scale` internal nodes (`2*scale + 1` nodes total); the
/// bushy shape has `~scale` leaves. Distinct leaf bases prevent collapse,
/// preserving the shape and size.
pub(crate) fn shape_version(shape: Shape, scale: usize) -> Version {
    use tree::Version as V;
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

/// Build [`shape_version`]'s tree with one leaf raised by `wide`, at the tip
/// or at an interior position halfway up. Requires `scale ≥ 1`.
///
/// The tip is the spines' deepest leaf and the bushy shape's first; the
/// interior position is the spines' mid-level off-spine leaf and the bushy
/// shape's middle leaf.
///
/// The deep-operand differentials draw `wide` from [`arb_magnitude`], so a walk's
/// suspended-ancestor state and pre-scan latents carry genuinely wide values
/// across real depth — the conjunction that neither the depth-capped
/// arbitrary trees nor the small-valued deep shapes reach on their own.
/// Raising one distinct counter by `wide` keeps every leaf base distinct, so
/// the shape and size survive normalization unchanged.
pub(crate) fn shape_version_wide(
    shape: Shape,
    scale: usize,
    wide: &BigUint,
    at_tip: bool,
) -> Version {
    use tree::Version as V;
    debug_assert!(scale >= 1, "a scale-0 shape has nowhere to put the leaf");
    if let Shape::Bushy = shape {
        let wide_at = if at_tip {
            0
        } else {
            (scale as u64).div_ceil(2)
        };
        let leaf_value = |k: u64| {
            if k == wide_at {
                wide.clone() + k
            } else {
                k.into()
            }
        };
        return from_oracle_version(&bushy_version_with(0, scale + 1, &leaf_value));
    }
    let mid = (scale as u64).div_ceil(2);
    let mut t = V::leaf(if at_tip {
        wide.clone()
    } else {
        BigUint::from(0u64)
    });
    for k in 1..=scale as u64 {
        let value = if !at_tip && k == mid {
            wide.clone() + k
        } else {
            BigUint::from(k)
        };
        let leaf = V::leaf(value);
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

/// Build a non-empty normal-form id of `shape` sized linearly in `scale`.
///
/// The spines carry a single owned region (a `1` leaf at the tip) with `0`
/// off-spine; the bushy shape carries many owned regions at varying depths (so
/// a `grow` over it has nodes whose two children are both feasible, exercising
/// the multi-region cost comparison).
pub(crate) fn shape_party(shape: Shape, scale: usize) -> Party {
    use tree::Party as P;
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

/// Build a depth-`depth` left-spine [`Party`] directly as canonical encoded
/// bits, with a single owned region at the deep-left tip.
///
/// Used by the stack-safety test, which needs structures far deeper than the
/// recursive oracle bridge (`emit_id`) or the oracle's own recursive `Drop`
/// could build or tear down. In the pruned encoding each spine node is a
/// `Left-only` tag (`10`: left child present, right absent — the `0` right
/// children take no bits), and the deep-left tip is a terminal (`00`). The
/// result `(((…(1, 0)…), 0), 0)` is normal form (no node has two terminal
/// children). Built with a flat loop: no recursion at any depth, in the builder
/// or in `Drop` (the encoded forms are flat buffers).
pub(crate) fn deep_left_spine_party(depth: usize) -> Party {
    let mut bits = bits::BitsWriter::with_capacity(2 * depth as u64 + 2);
    for _ in 0..depth {
        bits.push(true); // Left-only tag `10`: left child present ...
        bits.push(false); //   ... right child absent
    }
    bits.push(false); // terminal tag `00`: the deep-left owned tip
    bits.push(false);
    Party::from_test_bits(bits)
}

/// Build a depth-`depth` right-spine [`Party`] directly as canonical encoded
/// bits, with a single owned region at the deep-right tip.
///
/// The mirror of [`deep_left_spine_party`]: each spine node is a `Right-only`
/// tag (`01`), giving `(0, (0, …(0, 1)…))`. A walk that loops down left
/// children but recurses into right ones keeps one frame per level only on
/// this spine.
pub(crate) fn deep_right_spine_party(depth: usize) -> Party {
    let mut bits = bits::BitsWriter::with_capacity(2 * depth as u64 + 2);
    for _ in 0..depth {
        bits.push(false); // Right-only tag `01`: left child absent ...
        bits.push(true); //   ... right child present
    }
    bits.push(false); // terminal tag `00`: the deep-right owned tip
    bits.push(false);
    Party::from_test_bits(bits)
}

// ───────────────────────── arbitrary normal-form ─────────────────────────
//
// BigUint magnitudes deliberately span small values AND values near/beyond
// `u64::MAX`: this is the natural home for the path-sum-overflow regression
// class (path sums that would overflow a `u64`). With arbitrary-precision
// `BigUint` values the impl threads them losslessly, so the large-base
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

/// An event magnitude spanning small values and arbitrary-width boundaries.
///
/// The weighted arms keep common collapses frequent while reaching `u64` and
/// `u128` boundaries, low zero limbs, sparse high bits, and values wider than
/// two limbs. [`tests::generator_classes_stay_under_mass`] checks that the
/// wide classes remain reachable.
pub(crate) fn arb_magnitude() -> impl Strategy<Value = BigUint> {
    prop_oneof![
        6 => (0u64..6).prop_map(BigUint::from),
        2 => any::<u64>().prop_map(BigUint::from),
        1 => (u64::MAX - 4..=u64::MAX).prop_map(BigUint::from),
        1 => any::<u128>().prop_map(|n| BigUint::from(n) + BigUint::from(u64::MAX)),
        1 => (0u32..96).prop_map(|k| (BigUint::from(1u8) << k) + BigUint::from(1u8)),
        1 => (1u64..8).prop_map(|k| BigUint::from(k) << 64u32),
        1 => (0u64..4, 0u32..512).prop_map(|(j, k)| BigUint::from(2 * j + 1) << k),
    ]
}

/// An arbitrary normal-form id tree (may be the anonymous `Leaf(false)`).
///
/// Random recursive shape; every interior node goes through the oracle's
/// normalizing `Party::node`, so the result is always in normal form (no
/// collapsible `(b, b)` node survives).
pub(crate) fn arb_oracle_party() -> impl Strategy<Value = tree::Party> {
    let leaf = any::<bool>().prop_map(tree::Party::Leaf);
    leaf.prop_recursive(ARB_DEPTH, ARB_NODES, 2, |inner| {
        (inner.clone(), inner).prop_map(|(l, r)| tree::Party::node(l, r))
    })
}

/// An arbitrary nonempty normal-form id tree suitable for a [`Party`].
pub(crate) fn arb_oracle_party_nonempty() -> impl Strategy<Value = tree::Party> {
    arb_oracle_party().prop_map(|party| {
        if party.is_empty() {
            tree::Party::Leaf(true)
        } else {
            party
        }
    })
}

/// An arbitrary version whose live bits end at a byte boundary.
pub(crate) fn arb_flush_version() -> impl Strategy<Value = Version> {
    arb_oracle_version()
        .prop_map(|version| from_oracle_version(&version))
        .prop_filter("live bits end at a byte boundary", |version| {
            version.encoded_bits().is_multiple_of(8)
        })
}

/// An arbitrary party whose live bits end at a byte boundary.
pub(crate) fn arb_flush_party() -> impl Strategy<Value = Party> {
    arb_oracle_party_nonempty()
        .prop_map(|party| from_oracle_party(&party))
        .prop_filter("live bits end at a byte boundary", |party| {
            party.encoded_bits().is_multiple_of(8)
        })
}

/// An arbitrary normal-form event tree.
///
/// Random recursive shape with random base magnitudes from [`arb_magnitude`]
/// (including values near/beyond `u64::MAX`); every interior node goes through
/// the oracle's normalizing `Version::node`, so the result is always in normal
/// form (a zero-base child at every node, no collapsible `(n, m, m)`).
pub(crate) fn arb_oracle_version() -> impl Strategy<Value = tree::Version> {
    let leaf = arb_magnitude().prop_map(tree::Version::Leaf);
    leaf.prop_recursive(ARB_DEPTH, ARB_NODES, 2, |inner| {
        (arb_magnitude(), inner.clone(), inner).prop_map(|(n, l, r)| tree::Version::node(n, l, r))
    })
}

// ───────────────────────── variadic-law families ─────────────────────────

/// A list length spanning every fold case through two carry boundaries.
///
/// The range includes empty and singleton folds, combinations of raw and
/// merged values, both the carry and final-drain paths, and the boundaries at
/// eight and sixteen inputs.
pub(crate) fn arb_fold_arity() -> impl Strategy<Value = usize> {
    0usize..=17
}

/// A small version pool containing arbitrary values and the identity.
///
/// Drawing fold inputs from a pool makes repeats and shared values common,
/// while the empty version exercises identity shortcuts.
fn arb_version_pool() -> impl Strategy<Value = Vec<tree::Version>> {
    proptest::collection::vec(arb_oracle_version(), 1..=3).prop_map(|mut pool| {
        pool.push(tree::Version::new());
        pool
    })
}

/// Draw one receiver and a boundary-swept list of items from `pool`.
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

/// A receiver and list for variadic version laws.
///
/// Both come from the same small pool, making repeated values and receiver
/// aliases common at the lengths selected by [`arb_fold_arity`].
pub(crate) fn arb_version_family() -> impl Strategy<Value = (tree::Version, Vec<tree::Version>)> {
    arb_version_pool().prop_flat_map(family_picks)
}

/// A receiver and list for variadic party laws.
///
/// Repeated pool entries create overlapping regions, keeping both successful
/// joins and rejection with conservation under test.
pub(crate) fn arb_party_family() -> impl Strategy<Value = (tree::Party, Vec<tree::Party>)> {
    proptest::collection::vec(arb_oracle_party_nonempty(), 1..=4).prop_flat_map(family_picks)
}

/// A receiver and list for variadic clock laws.
///
/// Independent party and version pools cover arbitrary valid pairings. Reused
/// parties exercise overlap, and the empty version exercises fresh clocks.
pub(crate) fn arb_clock_family() -> impl Strategy<
    Value = (
        (tree::Party, tree::Version),
        Vec<(tree::Party, tree::Version)>,
    ),
> {
    (
        proptest::collection::vec(arb_oracle_party_nonempty(), 1..=3),
        arb_version_pool(),
    )
        .prop_flat_map(|(parties, versions)| {
            let pool: Vec<(tree::Party, tree::Version)> = parties
                .iter()
                .flat_map(|p| versions.iter().map(move |v| (p.clone(), v.clone())))
                .collect();
            family_picks(pool)
        })
}
