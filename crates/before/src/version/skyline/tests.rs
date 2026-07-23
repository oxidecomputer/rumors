//! Hand-pinned streams and the strict-reject corpus for the skyline codec.
//!
//! The hand pins fix the coding against arithmetic a reader can re-derive
//! in the margin; the reject corpus and the mutation sweeps pin the
//! validator's strictness — every non-canonical spelling is rejected, so
//! acceptance implies the stream is *the* canonical encoding of its value.

use std::collections::BTreeSet;

use proptest::prelude::*;

use crate::codec::{self, Base, Bits};
use crate::error::Decode;
use crate::meter::{alt_spine, cliff_comb, dense, hugeleaf, Packed};
use crate::testing::bridge::from_oracle_version;
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::generators;
use crate::{oracle, Version};

use super::{decode_bits, encode_bits, unzigzag, validate_bits, zigzag};

/// Decode a meter-generated packed shape as a [`Version`].
fn version_of(p: &Packed) -> Version {
    Version::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
}

// ─── hand-pinned streams ────────────────────────────────────────────────────

/// The empty version is the leaf 0 in both codings: topology `0` plus
/// `gamma(0)`, the two-bit stream `01`, and it round-trips.
#[test]
fn empty_version_is_the_two_bit_stream() {
    let v = Version::new();
    let bits = encode_bits(&v);
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
    let bits = encode_bits(&v);
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
    assert_eq!(encode_bits(&expected), bits);
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
        let bits = encode_bits(v);
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
    let clean = encode_bits(&v);
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
/// exhaustively at small scope: this is why the reject corpus has no
/// "non-canonical zigzag" member — the genre is empty by construction, as
/// is non-minimal gamma (a prefix code with one spelling per natural).
#[test]
fn zigzag_is_a_bijection_without_negative_zero() {
    let mut seen: BTreeSet<(bool, u64)> = BTreeSet::new();
    for m in 0..=100u64 {
        let (negative, magnitude) = unzigzag(Base::from(m));
        let mag = match magnitude {
            Base::Small(n) => n,
            Base::Big(_) => unreachable!("small codes decode to small magnitudes"),
        };
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
                encode_bits(&w),
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
        let bits = encode_bits(&v);
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
        let bits = encode_bits(&v);
        let flip = flip_seed.index(bits.len());
        assert_mutation_never_aliases(&v, &bits, flip);
    }
}
