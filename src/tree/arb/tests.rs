use super::{EARLY_DISPUTE_HINT, early_dispute_matches, nth_party};
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

/// Candidates the early-dispute search tries before giving up.
const EARLY_DISPUTE_ATTEMPTS: usize = 2048;

/// The early-dispute hint has the geometry its consumers need.
///
/// When path derivation changes, the hint goes stale; this test then
/// searches for the first qualifying candidate and reports it.
#[test]
fn early_dispute_hint_matches_current_paths() {
    let path_of = |version: &Version| <[u8; 32]>::from(Path::for_leaf(version));
    if !early_dispute_matches(EARLY_DISPUTE_HINT, &path_of) {
        let fresh =
            (0..EARLY_DISPUTE_ATTEMPTS).find(|&attempt| early_dispute_matches(attempt, &path_of));
        panic!("EARLY_DISPUTE_HINT is stale; replace it with {fresh:?}");
    }
}
