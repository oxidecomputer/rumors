use std::collections::BTreeMap;

use super::{
    EARLY_DISPUTE_HINT, FULL_FAN_INITIATOR_HINT, FULL_FAN_INITIATOR_PREFIXES,
    FULL_FAN_RESPONDER_HINT, early_dispute_matches, full_fan_responder_prefixes, hinted_versions,
    nth_party,
};
use crate::{Version, tree::typed::Path};
use before::Party;

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
    let path_of = |version: &Version| <[u8; 32]>::from(Path::for_fixture_leaf(version));
    if !early_dispute_matches(EARLY_DISPUTE_HINT, &path_of) {
        let fresh =
            (0..EARLY_DISPUTE_ATTEMPTS).find(|&attempt| early_dispute_matches(attempt, &path_of));
        panic!("EARLY_DISPUTE_HINT is stale; replace it with {fresh:?}");
    }
}

/// The first version on `party`'s chain under each of `prefixes`, as tick
/// counts in prefix order.
fn searched_ticks(party: &Party, prefixes: &[[u8; 2]]) -> Vec<usize> {
    let mut found = BTreeMap::<[u8; 2], usize>::new();
    let mut version = Version::new();
    let mut ticks = 0;
    while found.len() < prefixes.len() {
        version.tick(party);
        ticks += 1;
        let path = <[u8; 32]>::from(Path::for_fixture_leaf(&version));
        let prefix = [path[0], path[1]];
        if prefixes.contains(&prefix) {
            found.entry(prefix).or_insert(ticks);
        }
    }
    prefixes.iter().map(|prefix| found[prefix]).collect()
}

/// Both full-fan hints land every version under its prefix.
///
/// When path derivation changes, a hint goes stale; this test then searches
/// the chain and reports the replacement table. The search takes seconds in
/// a test build; run it under `--release`.
#[test]
fn full_fan_hints_match_current_paths() {
    let cases = [
        (
            "FULL_FAN_INITIATOR_HINT",
            nth_party(2),
            FULL_FAN_INITIATOR_PREFIXES.to_vec(),
            FULL_FAN_INITIATOR_HINT.to_vec(),
        ),
        (
            "FULL_FAN_RESPONDER_HINT",
            nth_party(1),
            full_fan_responder_prefixes(),
            FULL_FAN_RESPONDER_HINT.to_vec(),
        ),
    ];
    for (name, party, prefixes, hint) in cases {
        if hinted_versions(&party, &prefixes, &hint).is_none() {
            let fresh = searched_ticks(&party, &prefixes);
            panic!("{name} is stale; replace it with {fresh:?}");
        }
    }
}

/// A hint that no longer lands under its prefix is rejected.
#[test]
fn stale_version_hint_is_rejected() {
    assert!(hinted_versions(&nth_party(1), &[[0, 1]], &[1]).is_none());
}
