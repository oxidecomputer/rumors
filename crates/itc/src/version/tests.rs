//! Phase 2 working-form tests (Appendix D group A 4): `repack ∘ unpack == identity`
//! and yields canonical bytes.

use proptest::prelude::*;

use super::working::{repack, unpack};
use super::Version;
use crate::test_support::{from_oracle_version, run, versions, world_strategy};

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
