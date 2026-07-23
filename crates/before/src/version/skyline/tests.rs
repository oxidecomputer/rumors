//! Hand-pinned streams for the skyline codec.
//!
//! Exact bit-level pins on hand-derivable trees: the coding is small enough
//! to re-derive in the margin, so a walk regression fails against
//! arithmetic a reader can check by hand, and each pin closes with the
//! decode round-trip.

use crate::testing::bridge::from_oracle_version;
use crate::{oracle, Version};

use super::{decode_bits, encode_bits};

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
