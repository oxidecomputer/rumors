use std::collections::{BTreeSet, HashMap};

use before::{Party, Version};
use proptest::prelude::*;

use crate::count::{bit_window, MIN_PARTY_BITS, MIN_VERSION_BITS};
use crate::enumerate::{pack, party_subtrees, version_subtrees};

use super::{cell_rng, PartySampler, VersionSampler};

/// The largest byte length whose whole byte-string space the decoder
/// cross-checks enumerate (`256^n` decodes per length). Chosen by
/// measured runtime to keep the pins at seconds under the dev profile;
/// together with the grammar-level census pin (which reaches 24 bits
/// member-by-member) the accept set is covered into the low twenties of
/// bits.
const EXHAUSTIVE_BYTES: usize = 2;

/// Every canonical version of at most [`EXHAUSTIVE_BYTES`] packed bytes,
/// listed from the grammar: enumerate the sibling-rule family per exact
/// bit length, keep the nonnegative-height members, pack.
fn version_ground_truth(bytes: usize) -> BTreeSet<Vec<u8>> {
    bit_window(bytes, MIN_VERSION_BITS)
        .flat_map(|n| {
            version_subtrees(n)
                .into_iter()
                .filter(|m| m.heights_nonnegative())
                .map(|m| pack(&m.bits))
        })
        .collect()
}

/// Every canonical party of exactly `bytes` packed bytes, from the grammar.
fn party_ground_truth(bytes: usize) -> BTreeSet<Vec<u8>> {
    bit_window(bytes, MIN_PARTY_BITS)
        .flat_map(|n| party_subtrees(n).into_iter().map(|(bits, _)| pack(&bits)))
        .collect()
}

/// All byte strings of exactly `len` bytes that `accept` takes.
fn accepted_byte_strings(len: usize, accept: impl Fn(&[u8]) -> bool) -> BTreeSet<Vec<u8>> {
    let mut out = BTreeSet::new();
    let mut buf = vec![0u8; len];
    loop {
        if accept(&buf) {
            out.insert(buf.clone());
        }
        // Odometer increment; done when it wraps past the last byte.
        let mut i = len;
        loop {
            if i == 0 {
                return out;
            }
            i -= 1;
            buf[i] = buf[i].wrapping_add(1);
            if buf[i] != 0 {
                break;
            }
        }
    }
}

/// The sampler's domain must be exactly the decoder's accept set — not a
/// subset, not a superset. For every byte length up to the exhaustive
/// bound, the packed members the grammar enumeration produces must equal,
/// as a set, the byte strings `Version::decode` accepts out of all
/// `256^n` candidates.
#[test]
fn version_grammar_is_exactly_the_decoder_accept_set() {
    for len in 1..=EXHAUSTIVE_BYTES {
        let ours = version_ground_truth(len);
        let real = accepted_byte_strings(len, |b| Version::decode(b).is_ok());
        assert_eq!(
            ours, real,
            "version grammar and decoder accept set diverge at {len} bytes"
        );
    }
}

/// The party grammar's packed members must equal, as a set, the byte
/// strings `Party::decode` accepts out of all `256^n` candidates, for
/// every byte length up to the exhaustive bound.
#[test]
fn party_grammar_is_exactly_the_decoder_accept_set() {
    for len in 1..=EXHAUSTIVE_BYTES {
        let ours = party_ground_truth(len);
        let real = accepted_byte_strings(len, |b| Party::decode(b).is_ok());
        assert_eq!(
            ours, real,
            "party grammar and decoder accept set diverge at {len} bytes"
        );
    }
}

/// One-sided chi-square acceptance against the uniform null: the statistic
/// over `k` categories has mean `k - 1` and variance `2(k - 1)`, so six
/// standard deviations above the mean rejects honest uniformity with
/// probability under 1e-9 — a fixed seed makes the run deterministic
/// anyway; the margin documents how far from uniform a failure is.
fn chi_square_threshold(categories: usize) -> f64 {
    let dof = (categories - 1) as f64;
    dof + 6.0 * (2.0 * dof).sqrt()
}

/// The version sampler at a fixed byte size must (a) emit only members of
/// the enumerated canonical space — any alien draw fails by name — and
/// (b) hit them at frequencies a chi-square test cannot tell from
/// uniform: counting-guided generation plus whole-sample rejection is
/// exactly uniform, so a skew here means a weight, a window, or the
/// rejection filter is wrong.
#[test]
fn version_sampler_draws_uniformly_over_the_exact_size_space() {
    let size = 2; // 767 canonical members at exactly 2 bytes.
    let members: Vec<Vec<u8>> = version_ground_truth(size).into_iter().collect();
    let index: HashMap<&[u8], usize> = members
        .iter()
        .enumerate()
        .map(|(i, b)| (b.as_slice(), i))
        .collect();
    let sampler = VersionSampler::new(size);
    let per_category = 40u64;
    let draws = per_category * members.len() as u64;
    let mut observed = vec![0u64; members.len()];
    let mut rng = cell_rng(0xa71a5, "version_uniformity_pin", size, 0);
    for _ in 0..draws {
        let draw = sampler
            .sample_bytes(size, &mut rng)
            .expect("space is nonempty");
        let slot = index
            .get(draw.bytes.as_slice())
            .unwrap_or_else(|| panic!("sampler emitted a non-member: {:02x?}", draw.bytes));
        observed[*slot] += 1;
    }
    let expected = per_category as f64;
    let chi2: f64 = observed
        .iter()
        .map(|&o| {
            let d = o as f64 - expected;
            d * d / expected
        })
        .sum();
    let threshold = chi_square_threshold(members.len());
    assert!(
        chi2 <= threshold,
        "chi-square {chi2:.1} exceeds {threshold:.1} over {} categories",
        members.len()
    );
}

/// The party sampler at a fixed byte size must emit only enumerated
/// members, at frequencies a chi-square test cannot tell from uniform —
/// with no rejection anywhere, this pins the counting-guided walk alone.
#[test]
fn party_sampler_draws_uniformly_over_the_exact_size_space() {
    let size = 2;
    let members: Vec<Vec<u8>> = party_ground_truth(size).into_iter().collect();
    let index: HashMap<&[u8], usize> = members
        .iter()
        .enumerate()
        .map(|(i, b)| (b.as_slice(), i))
        .collect();
    let sampler = PartySampler::new(size);
    let per_category = 40u64;
    let draws = per_category * members.len() as u64;
    let mut observed = vec![0u64; members.len()];
    let mut rng = cell_rng(0xa71a5, "party_uniformity_pin", size, 0);
    for _ in 0..draws {
        let draw = sampler
            .sample_bytes(size, &mut rng)
            .expect("space is nonempty");
        let slot = index
            .get(draw.bytes.as_slice())
            .unwrap_or_else(|| panic!("sampler emitted a non-member: {:02x?}", draw.bytes));
        observed[*slot] += 1;
    }
    let expected = per_category as f64;
    let chi2: f64 = observed
        .iter()
        .map(|&o| {
            let d = o as f64 - expected;
            d * d / expected
        })
        .sum();
    let threshold = chi_square_threshold(members.len());
    assert!(
        chi2 <= threshold,
        "chi-square {chi2:.1} exceeds {threshold:.1} over {} categories",
        members.len()
    );
}

/// The same cell coordinates must replay the identical draw (the whole
/// plan's reproducibility rests on it), and a different sample index must
/// decorrelate — a cheap liveness check that the seed expansion actually
/// feeds the coordinates through.
#[test]
fn cell_seeding_is_deterministic_and_index_sensitive() {
    let sampler = VersionSampler::new(8);
    let a = sampler
        .sample_bytes(8, &mut cell_rng(7, "op", 8, 3))
        .expect("space is nonempty");
    let b = sampler
        .sample_bytes(8, &mut cell_rng(7, "op", 8, 3))
        .expect("space is nonempty");
    assert_eq!(a.bytes, b.bytes, "identical cells must replay identically");
    let c = sampler
        .sample_bytes(8, &mut cell_rng(7, "op", 8, 4))
        .expect("space is nonempty");
    assert_ne!(
        a.bytes, c.bytes,
        "adjacent sample indices drew byte-identical versions; the seed \
         expansion is not feeding the index through"
    );
}

proptest! {
    /// Every sampled version must round-trip through the real codec:
    /// `Version::decode` accepts the bytes, re-encoding reproduces them
    /// byte-identically, the live bit length matches the draw's, and the
    /// packed length is exactly the requested size. Stated over random
    /// (seed, size) cells so the family, not points, carries the claim.
    #[test]
    fn version_draws_round_trip_at_the_requested_size(
        seed in any::<u64>(),
        size in 1usize..=48,
    ) {
        let sampler = VersionSampler::new(48);
        let draw = sampler
            .sample_bytes(size, &mut cell_rng(seed, "roundtrip", size, 0))
            .expect("every byte size down to 1 has canonical versions");
        prop_assert_eq!(draw.bytes.len(), size);
        let version = Version::decode(&draw.bytes[..]).expect("sampler output must be canonical");
        prop_assert_eq!(version.encoded_bits(), draw.bits);
        prop_assert_eq!(version.encode(), draw.bytes);
    }

    /// Every sampled party must round-trip through the real codec, at the
    /// requested exact byte size, with the drawn bit length.
    #[test]
    fn party_draws_round_trip_at_the_requested_size(
        seed in any::<u64>(),
        size in 1usize..=48,
    ) {
        let sampler = PartySampler::new(48);
        let draw = sampler
            .sample_bytes(size, &mut cell_rng(seed, "roundtrip", size, 0))
            .expect("every byte size down to 1 has canonical parties");
        prop_assert_eq!(draw.bytes.len(), size);
        let party = Party::decode(&draw.bytes[..]).expect("sampler output must be canonical");
        prop_assert_eq!(party.encoded_bits(), draw.bits);
        prop_assert_eq!(party.encode(), draw.bytes);
    }
}
