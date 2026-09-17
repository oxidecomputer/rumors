use before::{Party, Version};
use num_bigint::BigUint;
use rayon::prelude::*;

use crate::enumerate::{party_subtrees, version_subtrees};

use super::{
    bit_window, PartyCounts, VersionCounts, MIN_PARTY_BITS, MIN_VERSION_BITS, PAR_SPLIT_THRESHOLD,
};

/// Largest bit length enumerated by the exact grammar checks.
///
/// At this limit the development-profile test completes in seconds.
const EXHAUSTIVE_BITS: usize = 24;

/// Version counts equal independent grammar enumeration at every tested length.
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

/// Enumerated canonical versions match the independent per-length census.
///
/// Enumeration filters the grammar by nonnegative height; the expected values
/// come from a separate dynamic program over the decoder's acceptance rules.
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

/// Party counts equal independent grammar enumeration at every tested length.
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

/// Parallel and sequential table construction produce identical entries.
///
/// The compile-time assertion ensures the chosen range reaches the parallel
/// reduction path for both grammars.
#[test]
fn parallel_build_matches_sequential_reference() {
    const PIN_BITS: usize = 2048;
    // A version entry at `j` bits sums `j - 4` splits, a party entry
    // `j - 5`: the span's top entries must actually take the rayon path.
    const {
        assert!(
            PIN_BITS - 5 >= PAR_SPLIT_THRESHOLD,
            "pin span too small to exercise the parallel path"
        )
    };
    let version_par = VersionCounts::build(PIN_BITS);
    let version_seq = VersionCounts::build_sequential(PIN_BITS);
    let party_par = PartyCounts::build(PIN_BITS);
    let party_seq = PartyCounts::build_sequential(PIN_BITS);
    for j in 0..=PIN_BITS {
        assert_eq!(
            version_par.subtree(j),
            version_seq.subtree(j),
            "version parallel build diverges from the sequential reference at {j} bits"
        );
        assert_eq!(
            party_par.subtree(j),
            party_seq.subtree(j),
            "party parallel build diverges from the sequential reference at {j} bits"
        );
    }
}

/// Largest byte length exhaustively passed through each public decoder.
const CENSUS_BYTES: usize = 3;

/// Largest live bit length covered by the decoder census.
///
/// A byte length `L` carries live bit lengths in `[8(L-1), 8L-1]` (the
/// decode padding rule — the marker claims one bit of the final byte),
/// so lengths `1..=CENSUS_BYTES` partition `0..=8 * CENSUS_BYTES - 1`
/// exactly.
const CENSUS_BITS: usize = 8 * CENSUS_BYTES - 1;

/// Count every accepted byte string by its decoded live bit length.
fn decoder_census(accept: impl Fn(&[u8]) -> Option<u64> + Sync) -> Vec<u64> {
    let empty = || vec![0u64; CENSUS_BITS + 1];
    let mut census = empty();
    for len in 1..=CENSUS_BYTES {
        let strings = 1u32 << (8 * len);
        let hist = (0..strings)
            .into_par_iter()
            .fold(empty, |mut hist, i| {
                let mut buf = [0u8; CENSUS_BYTES];
                for (b, slot) in buf[..len].iter_mut().enumerate() {
                    *slot = (i >> (8 * (len - 1 - b))) as u8;
                }
                if let Some(bits) = accept(&buf[..len]) {
                    assert!(
                        bits < 8 * len as u64,
                        "decoder reported {bits} live bits from a {len}-byte input"
                    );
                    // In range per the assert above: census inputs are a few
                    // bytes, so the bit length indexes the histogram.
                    hist[bits as usize] += 1;
                }
                hist
            })
            .reduce(empty, |mut a, b| {
                for (x, y) in a.iter_mut().zip(&b) {
                    *x += y;
                }
                a
            });
        for (c, h) in census.iter_mut().zip(&hist) {
            *c += h;
        }
    }
    census
}

/// `Version::decode` accepts exactly the enumerated canonical versions.
///
/// Both sides are counted by live bit length. The enumeration follows the
/// grammar and nonnegative-height rule, while the decoder census tries every
/// byte string through the public entry point.
#[test]
fn version_decoder_census_matches_constrained_family() {
    let census = decoder_census(|bytes| Version::decode(bytes).ok().map(|v| v.encoded_bits()));
    for (n, &counted) in census.iter().enumerate() {
        let expected = version_subtrees(n)
            .iter()
            .filter(|m| m.heights_nonnegative())
            .count() as u64;
        assert_eq!(
            counted, expected,
            "decoder accept census diverges from the constrained family at {n} bits"
        );
    }
}

/// `Party::decode` accepts exactly the parties counted by the table.
///
/// The table uses the grammar recurrence; the census tries every short byte
/// string through the public decoder and groups accepted values by bit length.
#[test]
fn party_decoder_census_matches_count_table() {
    let counts = PartyCounts::build(CENSUS_BITS);
    let census = decoder_census(|bytes| Party::decode(bytes).ok().map(|p| p.encoded_bits()));
    for (n, &counted) in census.iter().enumerate() {
        assert_eq!(
            &BigUint::from(counted),
            counts.whole(n),
            "decoder accept census diverges from the count table at {n} bits"
        );
    }
}

/// A canonical encoding of exactly `n` bytes carries a live bit length in
/// `[8(n-1), 8n-1]` — the marker claims one bit, and decode bounds the
/// padding to one byte — floored at the grammar's minimum subtree size.
#[test]
fn bit_window_matches_decode_padding_rule() {
    assert_eq!(bit_window(1, MIN_VERSION_BITS), 2..=7);
    assert_eq!(bit_window(1, MIN_PARTY_BITS), 2..=7);
    assert_eq!(bit_window(2, MIN_VERSION_BITS), 8..=15);
    assert_eq!(bit_window(3, MIN_PARTY_BITS), 16..=23);
}
