//! Hand-computed pins for the Tier 2 size function on small trees.
//!
//! Each test states the tree, the hand-derived Tier 2 bit count, and the
//! hand-derived current bit count, so a regression in the walk (path sums,
//! zigzag map, gamma lengths, topology accounting) fails against arithmetic a
//! reader can re-derive in the margin. Gamma lengths used below:
//! `gamma(0) = 1`, `gamma(1) = 3`, `gamma(2) = 3`, `gamma(4) = 5`,
//! `gamma(2^b - 1) = 2b + 1`.

use crate::meter::hugeleaf;
use crate::testing::bridge::from_oracle_version;
use crate::{oracle, Party, Version};

use super::{tier2_size, Tier2Size};

/// The empty version is the single leaf 0: one topology bit plus `gamma(0)`,
/// 2 bits in both encodings.
#[test]
fn empty_version_is_two_bits() {
    let v = Version::new();
    let size = tier2_size(&v);
    assert_eq!(
        size,
        Tier2Size {
            total_bits: 2,
            nodes: 1,
            leaves: 1,
            first_leaf_bits: 1,
            delta_bits: 0,
        }
    );
    assert_eq!(size.total_bits, v.encoded_bits() as u64);
}

/// A single ticked leaf (value 1) is one topology bit plus `gamma(1) = 3`,
/// 4 bits in both encodings.
#[test]
fn single_small_leaf_matches_current_size() {
    let mut v = Version::new();
    v.tick(&Party::seed());
    let size = tier2_size(&v);
    assert_eq!(size.total_bits, 4);
    assert_eq!((size.nodes, size.leaves), (1, 1));
    assert_eq!(size.total_bits, v.encoded_bits() as u64);
}

/// A single huge leaf `2^b - 1` is one topology bit plus `gamma(2^b - 1) =
/// 2b + 1`: Tier 2 equals the current `2b + 2` bits exactly, at a magnitude
/// wide enough to spill machine-word arithmetic.
#[test]
fn single_big_leaf_matches_current_size() {
    for b in [7, 200] {
        let packed = hugeleaf(b);
        let v = Version::decode(&packed.bytes[..]).expect("hugeleaf is strict normal form");
        let size = tier2_size(&v);
        assert_eq!(size.total_bits as usize, 2 * b + 2);
        assert_eq!((size.nodes, size.leaves), (1, 1));
        assert_eq!(size.first_leaf_bits as usize, 2 * b + 1);
        assert_eq!(size.total_bits, v.encoded_bits() as u64);
    }
}

/// One fork `(1, 0, 2)`: leaves are 1 and 3 absolute, so Tier 2 is 3 topology
/// bits + `gamma(1) = 3` + `zigzag(+2) = 4 -> gamma(4) = 5`, 11 bits against
/// today's 10 (`3 + gamma(1) + gamma(0) + gamma(2) = 3 + 3 + 1 + 3`).
#[test]
fn one_fork_matches_hand_computation() {
    let v = from_oracle_version(&oracle::Version::node(
        1u64,
        oracle::Version::leaf(0u64),
        oracle::Version::leaf(2u64),
    ));
    assert_eq!(v.encoded_bits(), 10);
    let size = tier2_size(&v);
    assert_eq!(
        size,
        Tier2Size {
            total_bits: 11,
            nodes: 3,
            leaves: 2,
            first_leaf_bits: 3,
            delta_bits: 5,
        }
    );
}

/// The dense spine `S(2)` has preorder leaves 0, 1, 0: Tier 2 is 5 topology
/// bits + `gamma(0) = 1` + `zigzag(+1) = 2 -> 3` + `zigzag(-1) = 1 -> 3`,
/// 12 bits, exactly today's `4d + 4 = 12`.
#[test]
fn dense_spine_matches_hand_computation() {
    let packed = crate::meter::dense(2);
    let v = Version::decode(&packed.bytes[..]).expect("dense spine is strict normal form");
    assert_eq!(v.encoded_bits(), 12);
    let size = tier2_size(&v);
    assert_eq!(
        size,
        Tier2Size {
            total_bits: 12,
            nodes: 5,
            leaves: 3,
            first_leaf_bits: 1,
            delta_bits: 6,
        }
    );
}

/// Equal leaf values meeting across a subtree boundary make Tier 2 strictly
/// smaller: `(0, (0, 0, 1), 1)` is 10 bits against today's 14.
///
/// Preorder leaves are 0, 1, 1, so the second delta is zero — a 1-bit code
/// where today stores `gamma(1)` twice. Pins the "sometimes smaller" claim.
#[test]
fn cross_boundary_equal_leaves_are_smaller_in_tier2() {
    let v = from_oracle_version(&oracle::Version::node(
        0u64,
        oracle::Version::node(
            0u64,
            oracle::Version::leaf(0u64),
            oracle::Version::leaf(1u64),
        ),
        oracle::Version::leaf(1u64),
    ));
    assert_eq!(v.encoded_bits(), 14);
    let size = tier2_size(&v);
    assert_eq!(
        size,
        Tier2Size {
            total_bits: 10,
            nodes: 5,
            leaves: 3,
            first_leaf_bits: 1,
            delta_bits: 4,
        }
    );
}
