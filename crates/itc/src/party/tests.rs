//! Phase 4 party tests (Appendix D group D 17–20): descent order, fork/join
//! round-trip, disjointness, and the meet / overlap behavior, all differential
//! against the oracle.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::{ops, Party};
use crate::metrics;
use crate::test_support::{
    arb_shape, assert_linear_scaling, contain_stress_pair, from_oracle_party, run, shape_party,
    skip_stress_pair, world_strategy,
};

/// Steps taken by `f` on a fresh counter.
fn steps_of(f: impl FnOnce()) -> u64 {
    metrics::reset();
    f();
    metrics::taken()
}

/// Smallest spine scale to measure at; below this the step count is too noisy for the
/// ratio to be meaningful. The big input is always `4x` this.
const MIN_SCALE: usize = 64;

/// `a <= b` under the impl descent order.
fn le(a: &Party, b: &Party) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

proptest! {
    /// D17/D20. The impl descent order and `is_disjoint` agree with the oracle, and
    /// the order obeys the partial-order laws (reflexive, antisymmetric, transitive).
    #[test]
    fn d_party_matches_oracle(
        ops in world_strategy(),
        i in 0usize..64,
        j in 0usize..64,
        k in 0usize..64,
    ) {
        let cs = run(&ops);
        let n = cs.len();
        let (oa, ob, oc) = (cs[i % n].party(), cs[j % n].party(), cs[k % n].party());
        let (ia, ib, ic) = (
            from_oracle_party(oa),
            from_oracle_party(ob),
            from_oracle_party(oc),
        );

        prop_assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(ob));
        prop_assert_eq!(ia.is_disjoint(&ib), oa.is_disjoint(ob));

        prop_assert_eq!(ia.partial_cmp(&ia), Some(Ordering::Equal));
        if le(&ia, &ib) && le(&ib, &ia) {
            prop_assert!(ia == ib);
        }
        if le(&ia, &ib) && le(&ib, &ic) {
            prop_assert!(le(&ia, &ic));
        }
    }
}

proptest! {
    /// D18/D19. `fork` yields two disjoint descendants of the parent (parent `<`
    /// each child), both matching the oracle; `join` of the two recovers the parent
    /// (the meet — a lower bound of both).
    #[test]
    fn d_fork_join_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut oracle_party = cs[i % n].party().clone();
        let snapshot = oracle_party.clone();

        let mut keep = from_oracle_party(&snapshot);
        let parent = from_oracle_party(&snapshot);
        let oracle_child = oracle_party.fork();
        let child = keep.fork();

        // Both halves match the oracle's split.
        prop_assert!(keep == from_oracle_party(&oracle_party));
        prop_assert!(child == from_oracle_party(&oracle_child));

        // Descent: the parent is strictly below (a lower bound of) each child.
        prop_assert_eq!(parent.partial_cmp(&keep), Some(Ordering::Less));
        prop_assert_eq!(parent.partial_cmp(&child), Some(Ordering::Less));

        // Forks are disjoint, and join recovers the parent.
        prop_assert!(keep.is_disjoint(&child));
        prop_assert!(keep.join(child).is_ok());
        prop_assert!(keep == parent);
    }
}

proptest! {
    /// Complexity. `split` is `O(n)`: over a random deep id shape, its traversal steps
    /// grow linearly (not quadratically) from `scale` to `4 * scale` — proving no
    /// re-scan to find a right child.
    #[test]
    fn split_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let p = shape_party(shape, s);
            steps_of(|| {
                ops::split(p.as_bits());
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `sum` is `O(n + m)`: on a deep disjoint pair (the halves of a forked
    /// spine) its steps grow linearly with shape size.
    #[test]
    fn sum_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let mut keep = shape_party(shape, s);
            let give = keep.fork(); // a deep disjoint pair; this build is not measured
            steps_of(|| {
                ops::sum(keep.as_bits(), give.as_bits());
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `is_disjoint` is `O(n + m)`: a *misaligned* disjoint pair (a shallow
    /// `0`-leaf on one side aligned with the other's whole deep subtree) drives the
    /// bounded lazy-skip at scale. The pair is disjoint, so the walk runs to completion
    /// (no early `false`) and the skip dominates; steps stay linear from `scale` to
    /// `4 * scale`, proving each node is skipped at most once (no per-node re-scan).
    #[test]
    fn is_disjoint_is_linear(scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let (a, b) = skip_stress_pair(s);
            steps_of(|| {
                ops::is_disjoint(a.as_bits(), b.as_bits());
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `compare` is `O(n + m)`: a *misaligned* containment pair (a shallow
    /// `1`-leaf on the container aligned with the contained's whole deep subtree) drives
    /// the bounded lazy-skip at scale. `big ⊇ small`, so the `a ⊇ b` direction stays live
    /// and the walk runs to completion (the `b ⊇ a` direction is excluded early but does
    /// not stop it); the skip dominates, and steps stay linear over the `4x` growth.
    #[test]
    fn compare_is_linear(scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let (big, small) = contain_stress_pair(s);
            steps_of(|| {
                ops::compare(big.as_bits(), small.as_bits());
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// D20. Joining overlapping parties errors and hands the party back unchanged.
    #[test]
    fn d_join_overlap_hands_back(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let snapshot = cs[i % n].party().clone();

        let mut sub = from_oracle_party(&snapshot);
        let _ = sub.fork(); // sub is now a sub-region of the snapshot
        let whole = from_oracle_party(&snapshot);
        let whole_copy = from_oracle_party(&snapshot);

        prop_assert!(!sub.is_disjoint(&whole));
        match sub.join(whole) {
            Err(handed_back) => prop_assert!(handed_back == whole_copy),
            Ok(()) => prop_assert!(false, "expected an overlap error"),
        }
    }
}

#[test]
fn parse_bare_notation() {
    let _party: Party = 1.try_into().unwrap();
    assert!(Party::try_from(0).is_err());
    let _party: Party = (1, 0).try_into().unwrap();
    let _party: Party = ((0, 1), (1, (1, 0))).try_into().unwrap();
}
