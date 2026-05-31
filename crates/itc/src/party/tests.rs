//! Party tests: descent order, fork/join round-trip, disjointness, and the
//! meet / overlap behavior, all differential against the oracle.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::Party;
use crate::idbits::IdView;
use crate::test_support::{
    arb_oracle_party, arb_oracle_party_nonempty, arb_shape, assert_linear_scaling,
    contain_stress_pair, from_oracle_party, run, shape_party, skip_stress_pair, steps_of,
    to_oracle_party, world_strategy, MIN_SCALE,
};

/// `a <= b` under the impl descent order.
fn le(a: &Party, b: &Party) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

// ───────────────────────────── differential vs oracle ─────────────────────────────

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

// ───────────────────────────── complexity (linear scaling) ─────────────────────────────

proptest! {
    /// Complexity. `split` is `O(n)`: over a random deep id shape, its traversal steps
    /// grow linearly (not quadratically) from `scale` to `4 * scale` — proving no
    /// re-scan to find a right child.
    #[test]
    fn split_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let p = shape_party(shape, s);
            steps_of(|| {
                IdView(p.as_bits()).split();
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
                IdView(keep.as_bits()).sum(&IdView(give.as_bits()));
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
                IdView(a.as_bits()).is_disjoint(&IdView(b.as_bits()));
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
                IdView(big.as_bits()).compare(&IdView(small.as_bits()));
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

// ───────────────────────────── join overlap ─────────────────────────────

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

// ───────────────────────── paper-notation TryFrom ─────────────────────────

/// `TryFrom` numeric/tuple literals build parties via the same paper notation as the
/// string parser: the seed `1`, a flat `(1, 0)`, and a nested `((0, 1), (1, (1, 0)))`
/// all construct, while the anonymous bare `0` is rejected (a standalone id must own
/// some region).
#[test]
fn parse_bare_notation() {
    let _party: Party = 1.try_into().unwrap();
    assert!(Party::try_from(0).is_err());
    let _party: Party = (1, 0).try_into().unwrap();
    let _party: Party = ((0, 1), (1, (1, 0))).try_into().unwrap();
}

// ───────────── arbitrary normal-form ids (decoupled from the op pipeline) ─────────────
//
// The op-trace differentials above only ever compare ids that descend from one seed (so
// every pair is causally related and pairwise disjoint by construction). These feed
// *arbitrary* normal-form ids — random shape, random ownership, including genuinely
// *overlapping* and *unrelated* pairs — to every id op and diff against the oracle. They
// reach the overlap/incomparable arms (`is_disjoint == false`, `compare == None`,
// `sum == None`) that the seed-derived pipeline cannot produce.

proptest! {
    /// `partial_cmp` (descent order) and `is_disjoint` on arbitrary id pairs —
    /// typically *unrelated* and frequently *overlapping* — agree with the oracle,
    /// including the incomparable (`None`) and not-disjoint verdicts the op pipeline never
    /// produces.
    #[test]
    fn compare_disjoint_arbitrary(
        oa in arb_oracle_party(),
        ob in arb_oracle_party(),
    ) {
        let (ia, ib) = (from_oracle_party(&oa), from_oracle_party(&ob));
        prop_assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(&ob));
        prop_assert_eq!(ia.is_disjoint(&ib), oa.is_disjoint(&ob));
        // Disjointness is symmetric on the impl directly.
        prop_assert_eq!(ia.is_disjoint(&ib), ib.is_disjoint(&ia));
    }
}

proptest! {
    /// `split` (the structural op behind `fork`) on an arbitrary non-empty id
    /// matches the oracle's `split`, structurally — on shapes the seed pipeline never
    /// forks. The two halves are read straight off the impl's packed `IdView::split`
    /// output and lowered for comparison.
    #[test]
    fn split_arbitrary(op in arb_oracle_party_nonempty()) {
        let mut oracle_self = op.clone();
        let oracle_give = oracle_self.fork(); // fork = split; mutates `oracle_self` to the kept half

        let p = from_oracle_party(&op);
        let (keep_bits, give_bits) = IdView(p.as_bits()).split();
        let keep = Party::from_bits(keep_bits);
        let give = Party::from_bits(give_bits);

        prop_assert!(keep == from_oracle_party(&oracle_self));
        prop_assert!(give == from_oracle_party(&oracle_give));
    }
}

proptest! {
    /// `sum` on arbitrary id pairs agrees with the oracle: it returns the merged
    /// id exactly when the pair is disjoint (matching `oracle::Party::join`), and `None`
    /// on overlap. The op pipeline only ever sums disjoint halves, so the overlap `None`
    /// arm is otherwise untested at arbitrary shapes.
    #[test]
    fn sum_arbitrary(
        oa in arb_oracle_party(),
        ob in arb_oracle_party(),
    ) {
        let (ia, ib) = (from_oracle_party(&oa), from_oracle_party(&ob));
        let summed = IdView(ia.as_bits()).sum(&IdView(ib.as_bits()));

        if oa.is_disjoint(&ob) {
            let mut oracle_sum = oa.clone();
            oracle_sum.join(ob.clone()).expect("disjoint, just checked");
            let bits = summed.expect("disjoint pair sums");
            prop_assert!(Party::from_bits(bits) == from_oracle_party(&oracle_sum));
        } else {
            prop_assert!(summed.is_none(), "overlapping ids must not sum");
        }
    }
}

proptest! {
    /// `decode ∘ encode == identity` over arbitrary non-empty normal-form ids,
    /// and the decoded value lowers to the same oracle tree. (The anonymous tree is
    /// excluded: a standalone `Party` must own a region, and `decode` rejects it.)
    #[test]
    fn decode_encode_arbitrary(op in arb_oracle_party_nonempty()) {
        let p = from_oracle_party(&op);
        let bytes = p.encode();
        let decoded = Party::decode(&bytes).expect("canonical encoding decodes");
        prop_assert!(decoded == p);
        prop_assert_eq!(to_oracle_party(&decoded), op);
    }
}
