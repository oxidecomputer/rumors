//! Radix sets against an ordered set of bytes as the oracle.

use std::collections::BTreeSet;

use proptest::prelude::*;

use super::Radices;

proptest! {
    /// A set collected from any bytes counts exactly the distinct bytes
    /// collected, and two sets are equal exactly when they were collected
    /// from the same distinct bytes.
    #[test]
    fn a_set_holds_exactly_its_distinct_radices(
        radices in proptest::collection::vec(any::<u8>(), 0..600),
        others in proptest::collection::vec(any::<u8>(), 0..600),
    ) {
        let set: Radices = radices.iter().copied().collect();
        let oracle: BTreeSet<u8> = radices.iter().copied().collect();
        prop_assert_eq!(set.len(), oracle.len());
        let other: BTreeSet<u8> = others.iter().copied().collect();
        prop_assert_eq!(set == others.iter().copied().collect::<Radices>(), oracle == other);
        // Each radix alone is its own set, distinct from every other.
        for radix in 0..=u8::MAX {
            let alone = std::iter::once(radix).collect::<Radices>();
            prop_assert_eq!(alone.len(), 1);
            prop_assert_eq!(set.union(alone) == set, oracle.contains(&radix), "radix {}", radix);
        }
    }

    /// The union of two sets holds the radices either holds, counted once:
    /// its size is the sum of theirs less the radices they share.
    #[test]
    fn a_union_counts_each_shared_radix_once(
        ours in proptest::collection::vec(any::<u8>(), 0..=512),
        theirs in proptest::collection::vec(any::<u8>(), 0..=512),
    ) {
        let ours: BTreeSet<u8> = ours.into_iter().collect();
        let theirs: BTreeSet<u8> = theirs.into_iter().collect();
        let union = ours
            .iter()
            .copied()
            .collect::<Radices>()
            .union(theirs.iter().copied().collect());
        let shared = ours.intersection(&theirs).count();
        prop_assert_eq!(union.len(), ours.len() + theirs.len() - shared);
        prop_assert_eq!(union, ours.union(&theirs).copied().collect::<Radices>());
    }
}
