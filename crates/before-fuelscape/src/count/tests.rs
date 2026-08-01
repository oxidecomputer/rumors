use before::{Party, Version};
use num_bigint::BigUint;
use rayon::prelude::*;

use crate::enumerate::{party_subtrees, version_subtrees};

use super::{
    bit_window, PartyCounts, VersionCounts, MIN_PARTY_BITS, MIN_VERSION_BITS, PAR_SPLIT_THRESHOLD,
};

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

/// The parallel build equals the sequential reference, entry for entry.
///
/// The parallel path re-associates each entry's split reduction across
/// worker threads; big-integer addition is associative and commutative,
/// so every reduction order must produce the identical table. This pin
/// holds `build` (parallel) equal to `build_sequential` (the reference)
/// for both grammars, at a span whose top entries reach past the
/// sequential-fallback threshold — the guard keeps the pin from going
/// vacuous if the threshold moves.
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

/// The largest byte length the decoder census sweeps: every byte string
/// of every length in `1..=CENSUS_BYTES` goes through the real decoder.
///
/// Chosen by measured runtime: `256^3` decodes per grammar run low
/// single-digit seconds under the dev profile with the sweep fanned out
/// over rayon.
const CENSUS_BYTES: usize = 3;

/// The largest exact bit length the decoder census covers: a byte length
/// `L` carries live bit lengths in `(8(L-1), 8L]` (the decode padding
/// rule), so lengths `1..=CENSUS_BYTES` partition `1..=8 * CENSUS_BYTES`
/// exactly.
const CENSUS_BITS: usize = 8 * CENSUS_BYTES;

/// Accepted-input counts per exact live bit length, from a real decoder.
///
/// Sweeps every byte string of every length in `1..=CENSUS_BYTES` through
/// `accept` (a public decode entry; `Some(live bit length)` on accept)
/// and tallies acceptances per live bit length. The sweep fans out over
/// rayon; per-chunk histograms add commutatively, so the fan-out cannot
/// change any count.
fn decoder_census(accept: impl Fn(&[u8]) -> Option<usize> + Sync) -> Vec<u64> {
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
                        bits <= 8 * len,
                        "decoder reported {bits} live bits from a {len}-byte input"
                    );
                    hist[bits] += 1;
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

/// The shipping decoder's accept census equals the grammar's constrained
/// family at every exact bit length up to [`CENSUS_BITS`].
///
/// Two derivations of the same number — the count of canonical version
/// streams at each exact bit length:
///
/// - the grammar transcription: brute-force enumeration of the
///   sibling-rule family filtered by the nonnegative-height rule (the
///   derivation the committed census literals and the counting table are
///   pinned against above);
/// - the shipping parser: every byte string of `1..=CENSUS_BYTES` bytes
///   through `Version::decode`, accepts bucketed by live bit length.
///
/// The two share no code, so a disagreement at any entry is a bug in
/// exactly one of them: either the enumeration mis-transcribes a
/// canonical-form rule, or the decoder's accept set has drifted from the
/// grammar the sampler's measure is built on. Bucketing by live bit
/// length also holds the padding rule: an accept carrying eight or more
/// pad bits would land a count in a bit length whose own byte window was
/// already tallied, and the entry would read high.
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

/// The shipping decoder's accept census equals the counting table, entry
/// for entry, at every exact bit length up to [`CENSUS_BITS`].
///
/// The party table counts the whole canonical family exactly (no
/// payloads, so nothing is deliberately relaxed), which makes this the
/// direct decoder-versus-table pin: the table's convolution recurrence on
/// one side, every byte string of `1..=CENSUS_BYTES` bytes through
/// `Party::decode` on the other, accepts bucketed by live bit length. The
/// two share no code, so a disagreement at any entry is a bug in exactly
/// one of them: either the recurrence mis-counts the grammar, or the
/// decoder's accept set has drifted from the grammar the sampler's
/// measure is built on.
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
