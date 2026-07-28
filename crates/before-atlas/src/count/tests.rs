use num_bigint::BigUint;

use crate::enumerate::{party_subtrees, version_subtrees};

use super::{bit_window, PartyCounts, VersionCounts, MIN_PARTY_BITS, MIN_VERSION_BITS};

/// The largest exact bit length the exhaustive cross-checks enumerate.
/// Chosen by measured runtime: the enumerations stay in the tens of
/// thousands of members here, seconds under the dev profile.
const EXHAUSTIVE_BITS: usize = 24;

/// The version table equals brute-force enumeration at every bit length.
///
/// The counting table is the sampler's measure; if it miscounts any size,
/// every probability downstream is wrong. For every exact bit length up
/// to the enumeration bound, the sibling-rule family's table must equal
/// the count of members listed by brute force from the grammar rules — a
/// derivation sharing no code with the table's convolution.
#[test]
fn version_counts_match_exhaustive_enumeration() {
    let counts = VersionCounts::build(EXHAUSTIVE_BITS);
    for n in 0..=EXHAUSTIVE_BITS {
        assert_eq!(
            counts.subtree(n),
            &BigUint::from(version_subtrees(n).len()),
            "sibling-rule family count diverges from enumeration at {n} bits"
        );
    }
}

/// The canonical per-bit-length counts match the independent census.
///
/// The number of canonical version streams at each exact bit length,
/// derived here by enumerating the coding grammar (topology bits + gamma
/// payloads) and filtering on the nonnegative-height rule, must equal the
/// counts independently derived by the entropy census of the same grammar
/// (an exact dynamic program over the validator's accept rules,
/// cross-pinned against brute force over all bit strings). Two
/// derivations, one number: drift in either grammar transcription moves a
/// committed integer.
#[test]
fn version_constrained_counts_match_independent_census() {
    const CENSUS: [usize; 20] = [
        0, 1, 0, 2, 0, 4, 1, 8, 6, 18, 17, 48, 52, 124, 160, 342, 488, 984, 1521, 2874,
    ];
    for (i, &expected) in CENSUS.iter().enumerate() {
        let n = i + 1;
        let count = version_subtrees(n)
            .iter()
            .filter(|m| m.heights_nonnegative())
            .count();
        assert_eq!(count, expected, "canonical census diverges at {n} bits");
    }
}

/// The party table counts the whole canonical id family (no payload
/// constraints exist to reject), so it must equal the brute-force
/// enumeration exactly at every bit length up to the bound.
#[test]
fn party_counts_match_exhaustive_enumeration() {
    let counts = PartyCounts::build(EXHAUSTIVE_BITS);
    for n in 0..=EXHAUSTIVE_BITS {
        assert_eq!(
            counts.subtree(n),
            &BigUint::from(party_subtrees(n).len()),
            "id family count diverges from enumeration at {n} bits"
        );
    }
}

/// A packed encoding of exactly `n` bytes carries a live bit length in
/// `(8(n-1), 8n]` — decode rejects 8 or more pad bits — floored at the
/// grammar's minimum subtree size.
#[test]
fn bit_window_matches_decode_padding_rule() {
    assert_eq!(bit_window(1, MIN_VERSION_BITS), 2..=8);
    assert_eq!(bit_window(1, MIN_PARTY_BITS), 2..=8);
    assert_eq!(bit_window(2, MIN_VERSION_BITS), 9..=16);
    assert_eq!(bit_window(3, MIN_PARTY_BITS), 17..=24);
}
