//! Phase 2 working-form tests (Appendix D group A 4): `repack ∘ unpack == identity`
//! and yields canonical bytes.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::working::{repack, unpack};
use super::Version;
use crate::metrics;
use crate::test_support::{
    assert_linear_scaling, deep_version_bits, from_oracle_version, run, versions, world_strategy,
};

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
