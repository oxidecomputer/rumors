//! Differential pins for Version comparison against the recursive oracle's
//! pointwise `leq` comparison.
//!
//! The oracle (through the bridge) is the verdict witness over the registered
//! families, arbitrary trees, organic histories, and the exhaustive small scope
//! — it shares no cursor, no delta, and no accumulator with the sweep.
//!
//! Every assertion runs all four entry points, so a bookkeeping error that
//! misreads a direction (rather than panicking) has four chances to separate
//! from the oracle on each pair, in both operand orders.

use core::cmp::Ordering;

use proptest::prelude::*;
use rayon::prelude::*;

use crate::testing::bridge::{from_oracle_version, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::meter::registry::Shape;
use crate::testing::meter::Encoding;
use crate::testing::oracles::tree;
use crate::testing::{generators, optrace};
use crate::{Clock, Version};

/// Decode a meter-generated encoded shape as a [`Version`].
fn version_of(p: &Encoding) -> Version {
    p.version()
}

/// The sweep's causal order of two versions, on their stored streams.
fn cmp_enc(a: &Version, b: &Version) -> Option<Ordering> {
    a.causal_cmp(b)
}

/// Assert all four entry points agree with the recursive oracle's
/// comparison on one pair, in both operand orders.
fn assert_verdicts(a: &Version, b: &Version) {
    let want = to_oracle_version(a).partial_cmp(&to_oracle_version(b));
    assert_eq!(
        a.causal_cmp(b),
        want,
        "causal_cmp disagrees with the recursive oracle: {a:?} vs {b:?}"
    );
    assert_eq!(
        b.causal_cmp(a),
        want.map(Ordering::reverse),
        "causal_cmp breaks antisymmetry against the recursive oracle: {b:?} vs {a:?}"
    );
    let equal = want == Some(Ordering::Equal);
    assert_eq!(a.walk_eq(b), equal, "eq disagrees: {a:?} vs {b:?}");
    assert_eq!(b.walk_eq(a), equal, "eq disagrees: {b:?} vs {a:?}");
    assert_eq!(
        a.walk_concurrent(b),
        want.is_none(),
        "concurrent disagrees: {a:?} vs {b:?}"
    );
    assert_eq!(
        a.walk_le(b),
        matches!(want, Some(Ordering::Less | Ordering::Equal)),
        "le disagrees: {a:?} vs {b:?}"
    );
    assert_eq!(
        b.walk_le(a),
        matches!(want, Some(Ordering::Greater | Ordering::Equal)),
        "le disagrees: {b:?} vs {a:?}"
    );
}

/// All four verdict outcomes are reachable and every entry point agrees with
/// the recursive oracle on each: Equal on an identical history, Less/Greater
/// across a join, None across concurrent forks.
#[test]
fn all_four_outcomes_agree() {
    let mut a = Clock::seed();
    let mut b = a.fork();
    let va = a.tick().clone();
    let vb = b.tick().clone();
    let joined = &va | &vb;

    assert_eq!(cmp_enc(&va, &va), Some(Ordering::Equal));
    assert_eq!(cmp_enc(&va, &joined), Some(Ordering::Less));
    assert_eq!(cmp_enc(&joined, &vb), Some(Ordering::Greater));
    assert_eq!(cmp_enc(&va, &vb), None);
    for (x, y) in [(&va, &va), (&va, &joined), (&joined, &vb), (&va, &vb)] {
        assert_verdicts(x, y);
    }
}

/// The flush-right tie at unequal depths: the deeper side's plateau ends
/// exactly at the shallower side's boundary, so both cursors advance in one
/// step — and the verdict still matches the recursive oracle.
#[test]
fn flush_right_ties_agree() {
    // `a`'s depth-2 pair fills the left half: its second leaf ends flush at
    // 1/2, exactly where `b`'s depth-1 first leaf ends. The heights mix
    // strictly across the overlay (`a` above on [1/4, 1/2), below on [3/4, 1)),
    // so the pair is concurrent.
    let a = from_oracle_version(&tree::Version::node(
        0u64,
        tree::Version::node(0u64, tree::Version::leaf(0u64), tree::Version::leaf(1u64)),
        tree::Version::leaf(1u64),
    ));
    let b = from_oracle_version(&tree::Version::node(
        0u64,
        tree::Version::leaf(0u64),
        tree::Version::node(0u64, tree::Version::leaf(1u64), tree::Version::leaf(2u64)),
    ));
    assert_eq!(cmp_enc(&a, &b), None, "the heights mix strictly");
    assert_verdicts(&a, &b);
}

/// A shallow operand is consumed as one long plateau: deep and wide shapes
/// against the empty version agree in both orders, with the whole deep side
/// merged against a single depth-0 leaf.
#[test]
fn deep_versus_empty_agrees() {
    for deep in [
        version_of(&Shape::Dense.build1(1_000)),
        version_of(&Shape::CliffComb.build2(64, 64)),
        version_of(&Shape::Bigroot.build2(64, 32)),
    ] {
        assert_verdicts(&deep, &Version::new());
    }
}

/// Every ordered pair drawn from the registered families yields identical
/// verdicts from the sweep and the recursive oracle.
///
/// The pool includes the empty version, and each operand is also compared
/// against the pair's join — the ordered outcome raw cross-family pairs
/// under-hit.
#[test]
fn family_pairs_agree() {
    let pool: Vec<Version> = vec![
        Version::new(),
        version_of(&Shape::Dense.build1(1)),
        version_of(&Shape::Dense.build1(2)),
        version_of(&Shape::Dense.build1(64)),
        version_of(&Shape::Bigroot.build2(7, 3)),
        version_of(&Shape::Bigroot.build2(64, 16)),
        version_of(&Shape::Hugeleaf.build1(1)),
        version_of(&Shape::Hugeleaf.build1(64)),
        version_of(&Shape::CliffComb.build2(3, 2)),
        version_of(&Shape::CliffComb.build2(16, 16)),
        version_of(&Shape::WideToothComb.build3(16, 8, 8)),
        version_of(&Shape::CliffFan.build2(16, 8)),
        version_of(&Shape::CancellingChain.build2(16, 8)),
        version_of(&Shape::AltSpine.build1(3)),
        version_of(&Shape::AltSpine.build1(64)),
        version_of(&Shape::Harmonic.build1(16)),
    ];
    for a in &pool {
        for b in &pool {
            assert_verdicts(a, b);
            let joined = a | b;
            assert_verdicts(a, &joined);
            assert_verdicts(&joined, b);
        }
    }
}

/// Exhaustive small scope: every ordered pair of normal-form event trees to the
/// small-scope depth yields identical verdicts from all four entry points and
/// the recursive oracle.
///
/// Brute force rather than sampling reaches every boundary case
/// deterministically: aligned ties, flush-right ties at unequal depths, plateau
/// consumption, zero deltas across subtree boundaries.
#[test]
fn exhaustive_small_scope_agrees() {
    let pool: Vec<(tree::Version, Version)> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(|t| {
            let v = from_oracle_version(t);
            (t.clone(), v)
        })
        .collect();
    pool.par_iter().for_each(|(ta, va)| {
        for (tb, vb) in &pool {
            let want = ta.partial_cmp(tb);
            assert_eq!(
                va.causal_cmp(vb),
                want,
                "causal_cmp disagrees: {va:?} vs {vb:?}"
            );
            assert_eq!(
                va.walk_eq(vb),
                want == Some(Ordering::Equal),
                "eq disagrees: {va:?} vs {vb:?}"
            );
            assert_eq!(
                va.walk_concurrent(vb),
                want.is_none(),
                "concurrent disagrees: {va:?} vs {vb:?}"
            );
            assert_eq!(
                va.walk_le(vb),
                matches!(want, Some(Ordering::Less | Ordering::Equal)),
                "le disagrees: {va:?} vs {vb:?}"
            );
        }
    });
}

proptest! {
    /// Arbitrary normal-form pairs (magnitudes past `u64::MAX` included) yield
    /// identical verdicts from the sweep and the recursive oracle; the pair's
    /// join and meet supply the ordered outcomes arbitrary pairs alone
    /// under-hit.
    #[test]
    fn arbitrary_pairs_agree(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
    ) {
        let (va, vb) = (from_oracle_version(&a), from_oracle_version(&b));
        assert_verdicts(&va, &vb);
        let joined = &va | &vb;
        assert_verdicts(&va, &joined);
        let met = &va & &vb;
        assert_verdicts(&met, &vb);
    }

    /// Every pair of versions produced by one organic fork/tick/send/sync/join
    /// history yields identical verdicts from the sweep and the recursive
    /// oracle.
    #[test]
    fn organic_histories_agree(ops in optrace::world_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        let pool: Vec<(tree::Version, &Version)> = clocks
            .iter()
            .map(|c| (to_oracle_version(c.version()), c.version()))
            .collect();
        for (ta, va) in &pool {
            for (tb, vb) in &pool {
                let want = ta.partial_cmp(tb);
                prop_assert_eq!(va.causal_cmp(vb), want, "causal_cmp disagrees: {:?} vs {:?}", va, vb);
                prop_assert_eq!(
                    va.walk_eq(vb),
                    want == Some(Ordering::Equal),
                    "eq disagrees: {:?} vs {:?}", va, vb
                );
                prop_assert_eq!(
                    va.walk_concurrent(vb),
                    want.is_none(),
                    "concurrent disagrees: {:?} vs {:?}", va, vb
                );
                prop_assert_eq!(
                    va.walk_le(vb),
                    matches!(want, Some(Ordering::Less | Ordering::Equal)),
                    "le disagrees: {:?} vs {:?}", va, vb
                );
            }
        }
    }
}
