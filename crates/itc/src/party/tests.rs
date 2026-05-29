//! Phase 4 party tests (Appendix D group D 17–20): descent order, fork/join
//! round-trip, disjointness, and the meet / overlap behavior, all differential
//! against the oracle.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::{ops, Party};
use crate::metrics;
use crate::test_support::{
    assert_linear_scaling, deep_party_bits, from_oracle_party, run, world_strategy,
};

/// Steps taken by `f` on a fresh counter.
fn steps_of(f: impl FnOnce()) -> u64 {
    metrics::reset();
    f();
    metrics::taken()
}

/// Two left-spine party depths a 4× node-count apart, for linear-scaling checks.
const SMALL_DEPTH: usize = 256;
const BIG_DEPTH: usize = 1024;

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

/// Complexity. `split` is `O(n)` in its input: traversal steps grow linearly, not
/// quadratically, on a deep left-spine id (the worst case for any right-child lookup
/// that re-scans the left subtree).
#[test]
fn split_is_linear() {
    let (small, sn) = deep_party_bits(SMALL_DEPTH);
    let (big, bn) = deep_party_bits(BIG_DEPTH);
    let small_steps = steps_of(|| {
        ops::split(&small);
    });
    let big_steps = steps_of(|| {
        ops::split(&big);
    });
    assert!(bn >= 3 * sn, "sizes should differ ~4x: {sn} vs {bn}");
    assert_linear_scaling(small_steps, big_steps);
}

/// Complexity. `sum` is `O(n + m)`: on a deep disjoint pair (the two halves of a forked
/// spine) its steps grow linearly.
#[test]
fn sum_is_linear() {
    let measure = |depth| {
        let (bits, _) = deep_party_bits(depth);
        let mut keep = Party::from_bits(bits);
        let give = keep.fork(); // a deep disjoint pair; this build is not measured
        steps_of(|| {
            ops::sum(keep.as_bits(), give.as_bits());
        })
    };
    assert_linear_scaling(measure(SMALL_DEPTH), measure(BIG_DEPTH));
}

/// Complexity. `is_disjoint` is `O(n + m)`: comparing a deep spine against a copy of
/// itself drives the both-internal lockstep down the whole spine, yet stays linear.
#[test]
fn is_disjoint_is_linear() {
    let measure = |depth| {
        let (bits, _) = deep_party_bits(depth);
        steps_of(|| {
            ops::is_disjoint(&bits, &bits);
        })
    };
    assert_linear_scaling(measure(SMALL_DEPTH), measure(BIG_DEPTH));
}

/// Complexity. `contains` is `O(n + m)`: the deep-lockstep self-comparison stays linear.
#[test]
fn contains_is_linear() {
    let measure = |depth| {
        let (bits, _) = deep_party_bits(depth);
        steps_of(|| {
            ops::contains(&bits, &bits);
        })
    };
    assert_linear_scaling(measure(SMALL_DEPTH), measure(BIG_DEPTH));
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
