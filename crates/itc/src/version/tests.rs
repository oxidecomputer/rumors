//! Phase 2 working-form tests (Appendix D group A 4): `repack ∘ unpack == identity`
//! and yields canonical bytes.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::working::{repack, unpack};
use super::Version;
use crate::test_support::{
    assert_linear_scaling, deep_version_bits, from_oracle_party, from_oracle_version, run,
    versions, world_strategy,
};
use crate::{metrics, Party};

/// Steps taken by `f` on a fresh counter.
fn steps_of(f: impl FnOnce()) -> u64 {
    metrics::reset();
    f();
    metrics::taken()
}

/// Two left-spine event-tree depths a 4× node-count apart, for linear-scaling checks.
const SMALL_DEPTH: usize = 256;
const BIG_DEPTH: usize = 1024;

/// `a <= b` under the impl causal order.
fn le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

/// `unpack` lays out a known event tree as preorder topology + base arrays.
#[test]
fn unpack_layout() {
    use crate::oracle::Version::{Leaf, Node};
    // (0, 1, 0): internal root, two leaves.
    let v = from_oracle_version(&Node(0, Box::new(Leaf(1)), Box::new(Leaf(0))));
    let w = unpack(v.as_bits());
    assert_eq!(w.len(), 3);
    assert_eq!(
        w.topo.iter().by_vals().collect::<Vec<_>>(),
        [true, false, false]
    );
    assert_eq!(w.base, [0, 1, 0]);
}

proptest! {
    /// A4. `repack(unpack(v)) == v` and the repacked bytes are canonical (equal to
    /// `v`'s own encoding).
    #[test]
    fn a4_working_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let v = from_oracle_version(&vs[i % n]);

        let work = unpack(v.as_bits());
        let repacked = Version::from_bits(repack(&work));

        prop_assert!(repacked == v);
        prop_assert_eq!(repacked.encode(), v.encode());
    }
}

/// Complexity. The causal order is `O(n + m)`: comparing a deep left-spine event tree
/// against itself drives the both-internal lockstep down the whole spine, yet steps
/// grow linearly rather than quadratically (no right-child re-scan).
#[test]
fn leq_is_linear() {
    let measure = |depth| {
        let v = Version::from_bits(deep_version_bits(depth).0);
        steps_of(|| {
            let _ = v.partial_cmp(&v);
        })
    };
    assert_linear_scaling(measure(SMALL_DEPTH), measure(BIG_DEPTH));
}

proptest! {
    /// C7–C10 (differential). The impl causal order agrees with the oracle's on
    /// every generated pair; this subsumes the order laws since the oracle satisfies
    /// them (O3) and the impl matches it exactly.
    #[test]
    fn c_compare_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let (oa, ob) = (&vs[i % n], &vs[j % n]);
        let (ia, ib) = (from_oracle_version(oa), from_oracle_version(ob));
        prop_assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(ob));
    }
}

proptest! {
    /// C7–C10. The order laws on impl versions directly: reflexive, antisymmetric,
    /// transitive; `==` ⇔ `Some(Equal)`; concurrency ⇔ `None`.
    #[test]
    fn c_order_laws(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let (a, b, c) = (
            from_oracle_version(&vs[i % n]),
            from_oracle_version(&vs[j % n]),
            from_oracle_version(&vs[k % n]),
        );

        prop_assert_eq!(a.partial_cmp(&a), Some(Ordering::Equal)); // reflexive
        if le(&a, &b) && le(&b, &a) {
            prop_assert!(a == b); // antisymmetric
        }
        if le(&a, &b) && le(&b, &c) {
            prop_assert!(le(&a, &c)); // transitive
        }
        prop_assert_eq!(a == b, a.partial_cmp(&b) == Some(Ordering::Equal));
        let concurrent = !le(&a, &b) && !le(&b, &a);
        prop_assert_eq!(concurrent, a.partial_cmp(&b).is_none());
    }
}

proptest! {
    /// F28. The comparison matrix agrees: `cmp(a,b)`, `cmp(a.batch(),b)`,
    /// `cmp(a,b.batch())`, and `cmp(a.batch(),b.batch())` all equal the bare
    /// version comparison (a fresh batch reflects its version).
    #[test]
    fn f28_representation_parity(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let base = a.partial_cmp(&b);

        let mut ba = a.clone();
        let mut bb = b.clone();
        let batch_a = ba.batch();
        let batch_b = bb.batch();

        prop_assert_eq!(batch_a.partial_cmp(&b), base); // Batch vs Version
        prop_assert_eq!(a.partial_cmp(&batch_b), base); // Version vs Batch
        prop_assert_eq!(batch_a.partial_cmp(&batch_b), base); // Batch vs Batch
        prop_assert_eq!(a == b, batch_a == batch_b); // PartialEq matrix agrees
    }
}

// ───────────────────────────── Phase 5: event mutation ─────────────────────────────

/// O1/C14. `Version::new()` is the empty history and the two-sided identity for `|`.
#[test]
fn new_is_join_identity() {
    use crate::oracle::Version::{Leaf, Node};
    let empty = Version::new();
    assert!(empty == from_oracle_version(&Leaf(0))); // empty history is Leaf(0)
    for v in [
        Leaf(0),
        Leaf(7),
        Node(1, Box::new(Leaf(0)), Box::new(Leaf(2))),
    ] {
        let iv = from_oracle_version(&v);
        assert!(empty.clone() | iv.clone() == iv);
        assert!(iv.clone() | empty.clone() == iv);
    }
}

proptest! {
    /// Phase 5 differential. The impl `tick` matches the oracle's `event` for every
    /// clock's own `(party, version)` (the party owns the regions tick may inflate).
    #[test]
    fn tick_matches_oracle(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (party, version) = cs[i % n].trees();

        let mut oracle_after = version.clone();
        oracle_after.tick(party);

        let mut iv = from_oracle_version(version);
        iv.tick(&from_oracle_party(party));

        prop_assert!(iv == from_oracle_version(&oracle_after));
    }
}

proptest! {
    /// O13 differential. The impl version join (`|`) matches the oracle's `ev_join`.
    #[test]
    fn merge_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let oracle_join = vs[i % n].clone() | vs[j % n].clone();
        let merged = from_oracle_version(&vs[i % n]) | from_oracle_version(&vs[j % n]);
        prop_assert!(merged == from_oracle_version(&oracle_join));
    }
}

proptest! {
    /// C11–C14, C16. The join lattice laws on impl values: upper bound, least upper
    /// bound, commutative/associative/idempotent, identity, and absorbing.
    #[test]
    fn c_lattice_laws(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let c = from_oracle_version(&vs[k % n]);

        let ab = a.clone() | b.clone();
        prop_assert!(le(&a, &ab) && le(&b, &ab)); // C11 upper bound

        // C12 least upper bound: any common upper bound dominates a|b.
        let upper = ab.clone() | c.clone();
        prop_assert!(le(&a, &upper) && le(&b, &upper));
        prop_assert!(le(&ab, &upper));

        prop_assert!(ab == (b.clone() | a.clone())); // C13 commutative
        let lhs = (a.clone() | b.clone()) | c.clone();
        let rhs = a.clone() | (b.clone() | c.clone());
        prop_assert!(lhs == rhs); // C13 associative
        prop_assert!((a.clone() | a.clone()) == a); // C13 idempotent

        prop_assert!((Version::new() | a.clone()) == a); // C14 identity

        if le(&a, &b) {
            prop_assert!((a.clone() | b.clone()) == b); // C16 absorbing
        }
    }
}

proptest! {
    /// C15. `tick` strictly advances the causal order: `a < a.tick(p)`.
    #[test]
    fn c15_monotone_tick(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (party, version) = cs[i % n].trees();
        let a = from_oracle_version(version);
        let p = from_oracle_party(party);

        let mut b = a.clone();
        b.tick(&p);
        prop_assert!(le(&a, &b)); // a <= a.tick
        prop_assert!(!le(&b, &a)); // strictly: not a.tick <= a
        prop_assert!(a != b);
    }
}

/// Complexity. `tick` is `O(n + m)`: ticking a deep event spine with the seed party
/// (which owns everything) scans the whole tree once; steps grow linearly.
#[test]
fn tick_is_linear() {
    let measure = |depth| {
        let mut v = Version::from_bits(deep_version_bits(depth).0);
        let p = Party::seed();
        steps_of(|| v.tick(&p))
    };
    assert_linear_scaling(measure(SMALL_DEPTH), measure(BIG_DEPTH));
}

/// Complexity. `merge` (`|`) is `O(n + m)`: joining two deep event spines stays linear.
#[test]
fn merge_is_linear() {
    let measure = |depth| {
        let a = Version::from_bits(deep_version_bits(depth).0);
        let b = a.clone();
        steps_of(|| {
            let _ = a.clone() | b.clone();
        })
    };
    assert_linear_scaling(measure(SMALL_DEPTH), measure(BIG_DEPTH));
}
