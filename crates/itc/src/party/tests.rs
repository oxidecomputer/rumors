//! Phase 4 party tests (Appendix D group D 17–20): descent order, fork/join
//! round-trip, disjointness, and the meet / overlap behavior, all differential
//! against the oracle.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::Party;
use crate::test_support::{from_oracle_party, run, world_strategy};

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
