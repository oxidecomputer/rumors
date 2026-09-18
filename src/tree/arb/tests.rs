use super::{
    EARLY_DISPUTE_HINT, early_dispute_attempt, early_dispute_radices, is_early_dispute, nth_party,
};
use crate::{Version, tree::typed::Path};

/// Distinct fixture indices yield mutually disjoint parties.
#[test]
fn distinct_indices_are_pairwise_disjoint() {
    const PARTIES: usize = 16;
    for i in 0..PARTIES {
        for j in i + 1..PARTIES {
            let (a, b) = (nth_party(i), nth_party(j));
            assert!(
                a.is_disjoint(&b),
                "nth_party({i}) = {a:?} and nth_party({j}) = {b:?} are not disjoint",
            );
        }
    }
}

/// The checked hint has the geometry required by its consumers.
#[test]
fn early_dispute_hint_matches_current_paths() {
    let path_of = |version: &Version| <[u8; 32]>::from(Path::for_leaf(version));
    let [left, right] = early_dispute_radices(EARLY_DISPUTE_HINT, &path_of);
    assert!(is_early_dispute([&left, &right]));
}

/// A stale hint falls back to the full deterministic search.
#[test]
fn early_dispute_search_recovers_from_a_stale_hint() {
    const STALE_HINT: usize = 0;
    let path_of = |version: &Version| <[u8; 32]>::from(Path::for_leaf(version));
    let [left, right] = early_dispute_radices(STALE_HINT, &path_of);
    assert!(
        !is_early_dispute([&left, &right]),
        "the negative control must not satisfy the geometry",
    );
    assert_eq!(
        early_dispute_attempt(&path_of, STALE_HINT),
        Some(EARLY_DISPUTE_HINT),
    );
}
