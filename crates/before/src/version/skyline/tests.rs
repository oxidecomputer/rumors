//! Agreement pins and the strict-reject corpus for the skyline codec.
//!
//! Three independent artifacts triangulate here: the stored streams the
//! operations and the transcoder emit, the sizer [`tier2_size`] (an
//! independent walk over the packed construction language with its own
//! zigzag map), and the decoder (validation plus wrap). Length agreement
//! pins every built stream against the sizer; the round-trip pins the
//! stream against the decoder through canonical uniqueness; the reject
//! corpus and the mutation sweeps pin the validator's strictness — every
//! non-canonical spelling is rejected, so acceptance implies the stream is
//! *the* canonical encoding of its value.

use std::collections::BTreeSet;

use proptest::prelude::*;

use crate::codec::{self, Base, Bits};
use crate::error::Decode;
use crate::meter::tier2::tier2_size;
use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, hugeleaf, wide_tooth_comb,
    Packed,
};
use crate::testing::bridge::{from_oracle_version, packed_bits_of, to_oracle_version};
use crate::testing::compactness::{arb_comb_params, comb};
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::{generators, optrace};
use crate::{oracle, Clock, Version};

use super::{decode_bits, unzigzag, validate_bits, zigzag};

/// Lift a meter-generated packed shape into a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// The stored skyline stream of a version, as live bits.
fn stream_of(v: &Version) -> Bits {
    let enc = v.as_encoded();
    codec::bytes_as_bits(&enc.bytes)[..enc.bits].to_bitvec()
}

// ─── hand-pinned streams ────────────────────────────────────────────────────

/// The empty version is the leaf 0 in both codings: topology `0` plus
/// `gamma(0)`, the two-bit stream `01`, and it round-trips.
#[test]
fn empty_version_is_the_two_bit_stream() {
    let v = Version::new();
    let bits = stream_of(&v);
    assert_eq!(bits.len(), 2);
    assert!(!bits[0], "a leaf's topology flag is 0");
    assert!(bits[1], "gamma(0) is the single bit 1");
    assert_eq!(decode_bits(&bits).expect("canonical"), v);
}

/// One fork `(1, 0, 2)` codes as hand-derived: 3 topology bits,
/// `gamma(1)` for the first leaf (height 1), and `zigzag(+2) = 4 ->
/// gamma(4)` for the second (height 3), 11 bits total — and round-trips.
#[test]
fn one_fork_matches_hand_derivation() {
    let v = from_oracle_version(&oracle::Version::node(
        1u64,
        oracle::Version::leaf(0u64),
        oracle::Version::leaf(2u64),
    ));
    let bits = stream_of(&v);
    // Preorder: internal root, leaf(gamma(1) = 010), leaf(gamma(4) = 00101).
    let expected: Vec<bool> = [
        true, // root: internal
        false, false, true, false, // left leaf: flag 0, gamma(1)
        false, false, false, true, false, true, // right leaf: flag 0, gamma(4)
    ]
    .to_vec();
    assert_eq!(bits.iter().by_vals().collect::<Vec<bool>>(), expected);
    assert_eq!(decode_bits(&bits).expect("canonical"), v);
}

// ─── the strict-reject corpus ───────────────────────────────────────────────

/// Append a leaf carrying a raw payload value (the caller pre-zigzags).
fn push_leaf(bits: &mut Bits, payload: u64) {
    bits.push(false);
    codec::encode_int(bits, &Base::from(payload));
}

/// An internal node whose two leaf children carry a zero right delta is
/// the collapsible pair: rejected as [`Decode::NotCanonical`].
#[test]
fn rejects_zero_right_sibling_delta() {
    // (5, 5): internal root, leaf height 5, then delta 0.
    let mut bits = Bits::new();
    bits.push(true);
    push_leaf(&mut bits, 5); // gamma(5): the first leaf, absolute
    push_leaf(&mut bits, 0); // zigzag(0) = 0 -> gamma(0): equal sibling
    assert!(matches!(validate_bits(&bits), Err(Decode::NotCanonical)));
}

/// A zero delta between non-sibling consecutive leaves is a canonical
/// shape: two equal plateaus across a subtree boundary validate and
/// round-trip to the expected version.
#[test]
fn accepts_zero_delta_across_a_subtree_boundary() {
    // (0, (0, 0, 1), 1): preorder leaves 0, 1, 1 — the second delta is 0,
    // legal because the leaves flank a subtree boundary.
    let expected = from_oracle_version(&oracle::Version::node(
        0u64,
        oracle::Version::node(
            0u64,
            oracle::Version::leaf(0u64),
            oracle::Version::leaf(1u64),
        ),
        oracle::Version::leaf(1u64),
    ));
    let mut bits = Bits::new();
    bits.push(true); // root
    bits.push(true); // left child: internal
    push_leaf(&mut bits, 0); // leaf 0: gamma(0), absolute
    push_leaf(&mut bits, 2); // leaf 1: zigzag(+1) = 2
    push_leaf(&mut bits, 0); // leaf 1 again: zigzag(0) = 0, non-sibling
    assert!(validate_bits(&bits).is_ok());
    assert_eq!(decode_bits(&bits).expect("canonical"), expected);
    assert_eq!(stream_of(&expected), bits);
}

/// A delta that drives the running leaf height negative is rejected as
/// [`Decode::NotCanonical`]: leaf heights are naturals, and no topology
/// bit can see this — only the running value state can.
#[test]
fn rejects_negative_running_height() {
    // (1, -1): internal root, leaf height 1, then delta -2.
    let mut bits = Bits::new();
    bits.push(true);
    push_leaf(&mut bits, 1); // first leaf: height 1
    push_leaf(&mut bits, 3); // zigzag(-2) = 3: height would be -1
    assert!(matches!(validate_bits(&bits), Err(Decode::NotCanonical)));
}

/// A negative excursion is rejected even when later deltas would climb
/// back up: validity is per prefix, not per total.
#[test]
fn rejects_negative_height_midstream() {
    // Root over leaf(1) and (node over leaf(-1), leaf(5)): the middle leaf
    // dips negative before the last one recovers.
    let mut bits = Bits::new();
    bits.push(true); // root
    push_leaf(&mut bits, 1); // first leaf: height 1
    bits.push(true); // right child: internal
    push_leaf(&mut bits, 3); // zigzag(-2) = 3: height -1, invalid here
    push_leaf(&mut bits, 12); // zigzag(+6) = 12: would recover to 5
    assert!(matches!(validate_bits(&bits), Err(Decode::NotCanonical)));
}

/// Every proper prefix of a valid stream is rejected as
/// [`Decode::Truncated`], swept over every cut point of several shapes:
/// the coding is self-delimiting, so no prefix is a complete tree.
#[test]
fn rejects_every_truncation() {
    let shapes: Vec<Version> = vec![
        Version::new(),
        version_of(&dense(3)),
        version_of(&cliff_comb(4, 3)),
        version_of(&hugeleaf(9)),
        version_of(&alt_spine(4)),
    ];
    for v in &shapes {
        let bits = stream_of(v);
        for cut in 0..bits.len() {
            assert!(
                matches!(validate_bits(&bits[..cut]), Err(Decode::Truncated)),
                "a {cut}-bit prefix of a {}-bit stream must read as truncated",
                bits.len(),
            );
        }
    }
}

/// Live bits after a complete tree are rejected as
/// [`Decode::TrailingBits`]: one zero bit, one set bit, or a whole second
/// tree.
#[test]
fn rejects_trailing_bits() {
    let v = version_of(&dense(3));
    let clean = stream_of(&v);
    for extra in [false, true] {
        let mut bits = clean.clone();
        bits.push(extra);
        assert!(matches!(validate_bits(&bits), Err(Decode::TrailingBits)));
    }
    let mut two_trees = clean.clone();
    two_trees.extend_from_bitslice(&clean);
    assert!(matches!(
        validate_bits(&two_trees),
        Err(Decode::TrailingBits)
    ));
}

/// The zigzag map is a bijection with no negative-zero spelling, checked
/// exhaustively at small scope.
///
/// This is why the reject corpus has no "non-canonical zigzag" member —
/// the genre is empty by construction, as is non-minimal gamma (a prefix
/// code with one spelling per natural).
#[test]
fn zigzag_is_a_bijection_without_negative_zero() {
    let mut seen: BTreeSet<(bool, u64)> = BTreeSet::new();
    for m in 0..=100u64 {
        let (negative, magnitude) = unzigzag(Base::from(m));
        let mag = magnitude
            .to_u64()
            .expect("small codes decode to small magnitudes");
        assert!(!(negative && mag == 0), "no code may spell a negative zero");
        assert!(
            seen.insert((negative, mag)),
            "two codes decoded to one delta: the map is not injective"
        );
        // Re-encode through the encoder's map: the round-trip pins the two
        // helpers as mutual inverses over the same sign convention.
        let (prev, cur) = if negative {
            (Base::from(mag), Base::ZERO)
        } else {
            (Base::ZERO, Base::from(mag))
        };
        assert_eq!(zigzag(&prev, &cur), Base::from(m));
    }
}

// ─── single-bit mutation sweeps ─────────────────────────────────────────────

/// Assert one mutated stream never aliases its origin: it is rejected, or
/// it decodes to a different version whose canonical encoding is the
/// mutated stream itself.
fn assert_mutation_never_aliases(v: &Version, bits: &Bits, flip: usize) {
    let mut mutated = bits.clone();
    let old = mutated[flip];
    mutated.set(flip, !old);
    match decode_bits(&mutated) {
        Err(_) => {}
        Ok(w) => {
            assert_ne!(
                &w, v,
                "a single-bit mutation decoded back to the same version: \
                 two spellings of one value were both accepted"
            );
            assert_eq!(
                stream_of(&w),
                mutated,
                "an accepted stream must be the canonical encoding of its value"
            );
        }
    }
}

/// Exhaustive small scope: flipping any single bit of any depth-2
/// normal form's encoding either rejects or round-trips to a different
/// canonical value — no silent acceptance of non-canonical spellings.
#[test]
fn exhaustive_single_bit_mutations_never_alias() {
    for t in all_normal_events(EV_SMALL_DEPTH) {
        let v = from_oracle_version(&t);
        let bits = stream_of(&v);
        for flip in 0..bits.len() {
            assert_mutation_never_aliases(&v, &bits, flip);
        }
    }
}

proptest! {
    /// Arbitrary trees under a proptest-chosen single-bit flip either
    /// reject or round-trip to a different canonical value.
    #[test]
    fn arbitrary_single_bit_mutations_never_alias(
        t in generators::arb_oracle_version(),
        flip_seed in any::<prop::sample::Index>(),
    ) {
        let v = from_oracle_version(&t);
        let bits = stream_of(&v);
        let flip = flip_seed.index(bits.len());
        assert_mutation_never_aliases(&v, &bits, flip);
    }
}

// ─── agreement over the generator families ──────────────────────────────────

/// The full agreement pin on one version.
///
/// The stored stream's length equals the independent sizer bit for bit
/// (the sizer walks the packed construction language, re-derived through
/// the oracle lowering), the stream validates, and decoding it reproduces
/// the version exactly.
fn assert_agreement(v: &Version) {
    let bits = stream_of(v);
    let packed = packed_bits_of(&to_oracle_version(v));
    let size = tier2_size(&packed);
    assert_eq!(
        bits.len() as u64,
        size.total_bits,
        "stored skyline length disagrees with the tier2 sizer: one of the \
         two independent walks is wrong"
    );
    assert!(
        validate_bits(&bits).is_ok(),
        "the encoder emits canonical streams"
    );
    let back = decode_bits(&bits).expect("a canonical stream decodes");
    assert_eq!(
        &back, v,
        "the skyline round-trip reproduces the version exactly"
    );
}

/// Every adversarial generator family agrees with the sizer and round-trips
/// exactly, across a deterministic size grid per family.
#[test]
fn generator_families_agree_and_round_trip() {
    let shapes: Vec<Packed> = vec![
        dense(1),
        dense(2),
        dense(64),
        dense(1_000),
        bigroot(7, 3),
        bigroot(200, 50),
        bigroot(1_000, 200),
        hugeleaf(1),
        hugeleaf(64),
        hugeleaf(5_000),
        cliff_comb(3, 2),
        cliff_comb(64, 64),
        cliff_comb(512, 512),
        wide_tooth_comb(64, 8, 16),
        wide_tooth_comb(512, 192, 64),
        cliff_fan(64, 64),
        cliff_fan(512, 128),
        cancelling_chain(64, 64),
        cancelling_chain(512, 128),
        alt_spine(1),
        alt_spine(2),
        alt_spine(3),
        alt_spine(64),
        alt_spine(1_001),
    ];
    for p in &shapes {
        assert_agreement(&version_of(p));
    }
}

/// Exhaustive small scope: every normal-form tree to depth 2 round-trips,
/// agrees with the sizer, and no two distinct versions share a skyline
/// stream (injectivity, the other face of byte uniqueness).
#[test]
fn exhaustive_small_scope_agrees_and_is_injective() {
    let pool = all_normal_events(EV_SMALL_DEPTH);
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    for t in &pool {
        let v = from_oracle_version(t);
        assert_agreement(&v);
        let enc = super::encode(&v);
        // Key on padded bytes plus the live length: distinct versions must
        // differ somewhere a decoder can see.
        let mut key = enc.bytes.clone();
        key.extend_from_slice(&enc.bits.to_le_bytes());
        assert!(
            seen.insert(key),
            "two distinct versions encoded to one skyline stream: {v}"
        );
    }
}

proptest! {
    /// Arbitrary normal-form trees (magnitudes past `u64::MAX` included)
    /// agree with the sizer and round-trip exactly.
    #[test]
    fn arbitrary_trees_agree_and_round_trip(t in generators::arb_oracle_version()) {
        assert_agreement(&from_oracle_version(&t));
    }

    /// Every version produced by an organic fork/tick/send/sync/join
    /// history agrees with the sizer and round-trips exactly.
    #[test]
    fn organic_histories_agree_and_round_trip(ops in optrace::world_strategy_up_to(120)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for clock in &clocks {
            assert_agreement(clock.version());
        }
    }

    /// Alternating combs — the compactness suite's tightness family, every
    /// consecutive-leaf delta a full magnitude swing — agree and round-trip.
    #[test]
    fn alternating_combs_agree_and_round_trip((m, p) in arb_comb_params()) {
        assert_agreement(&comb(m, p));
    }

    /// Value-equal versions built along different operation paths produce
    /// byte-identical skyline streams: the coding is a function of the
    /// value, never of the op path that constructed it.
    #[test]
    fn op_paths_yield_identical_bytes(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
        c in generators::arb_oracle_version(),
    ) {
        let (a, b, c) = (
            from_oracle_version(&a),
            from_oracle_version(&b),
            from_oracle_version(&c),
        );
        prop_assert_eq!(stream_of(&(&a | &b)), stream_of(&(&b | &a)));
        prop_assert_eq!(
            stream_of(&(&(&a | &b) | &c)),
            stream_of(&(&a | &(&b | &c)))
        );
        prop_assert_eq!(stream_of(&(&a & &b)), stream_of(&(&b & &a)));
    }
}
