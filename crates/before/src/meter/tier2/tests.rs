//! Hand-computed pins for the Tier 2 size function on small trees.
//!
//! Each test states the tree, the hand-derived Tier 2 bit count, and the
//! hand-derived current bit count, so a regression in the walk (path sums,
//! zigzag map, gamma lengths, topology accounting) fails against arithmetic a
//! reader can re-derive in the margin. Gamma lengths used below:
//! `gamma(0) = 1`, `gamma(1) = 3`, `gamma(2) = 3`, `gamma(4) = 5`,
//! `gamma(2^b - 1) = 2b + 1`.

use crate::codec::Base;
use crate::meter::{cliff_comb, hugeleaf};
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

/// The boundary comb's Tier 2 size is exactly `10n + 4k + 2` bits against
/// today's `n(2k + 10) + 2`: Tier 2 wire bits do not bound value content.
///
/// `cliff_comb(k, n)` codes each `±1` leaf delta in 3 bits where today's
/// form stores a fresh `gamma(2^k − 1)` per tooth, so the current/Tier 2
/// size ratio grows without bound in `k` — the `≤ 2×` compactness envelope
/// holds in the useless direction while the comb's `2n + 1` leaves carry
/// `Θ(nk)` bits of absolute value content behind `Θ(n + k)` Tier 2 wire
/// bits. The per-part pin: `4n + 1` topology bits, `gamma(2^k − 1) =
/// 2k + 1` first-leaf bits, `3(2n − 1)` oscillation deltas plus the
/// `2k + 3`-bit closing delta to the terminal leaf 0. The exact ratios at
/// `n = k`: 9.837× (n = 64), 146.980× (n = 1024), 585.837× (n = 4096);
/// the floors below sit just under them.
#[test]
fn cliff_comb_tier2_size_is_linear_while_current_is_quadratic() {
    for (k, n) in [(3, 2), (64, 64), (200, 50), (1024, 1024), (4096, 4096)] {
        let packed = cliff_comb(k, n);
        let v = Version::decode(&packed.bytes[..]).expect("comb is strict normal form");
        let size = tier2_size(&v);
        assert_eq!(
            size,
            Tier2Size {
                total_bits: (10 * n + 4 * k + 2) as u64,
                nodes: (4 * n + 1) as u64,
                leaves: (2 * n + 1) as u64,
                first_leaf_bits: (2 * k + 1) as u64,
                delta_bits: (6 * n + 2 * k) as u64,
            }
        );
        assert_eq!(v.encoded_bits(), n * (2 * k + 10) + 2);
    }
    for (n, ratio_floor) in [(64, 9.83), (1024, 146.97), (4096, 585.83)] {
        let current = (n * (2 * n + 10) + 2) as f64;
        let tier2 = (14 * n + 2) as f64;
        assert!(
            current / tier2 >= ratio_floor,
            "current/tier2 ratio at n = k = {n} fell below its pinned floor {ratio_floor}"
        );
    }
}

/// A plain running-value accumulator over the comb's Tier 2 delta stream
/// costs limb work quadratic in the wire bits: per-wire-bit cost roughly
/// doubles when the size doubles.
///
/// This is the executable witness that carry-run amortization does not
/// transfer to the delta coding: each 3-bit `±1` delta lands exactly on
/// the `2^k` carry boundary, so applying it to a plain big-integer
/// accumulator propagates a full `k`-bit carry or borrow — `Θ(k)` limb work
/// bought by `O(1)` wire bits, `Θ(W²)` total in wire bits `W`. Under
/// today's coding the same tree pays `2k + 1` stored bits per crossing
/// (the envelope suite pins those operations linear). Any Tier 2 sweep
/// that must materialize running leaf values — strict decode's
/// nonnegativity validation included, since values are naturals and a
/// plain 2-bit/level topology check cannot see a delta drive one negative
/// — inherits this cost unless it uses a carry-immune accumulator design.
#[cfg(feature = "limb-meter")]
#[test]
fn cliff_comb_plain_delta_sweep_is_quadratic_in_tier2_wire_bits() {
    // Apply the comb's delta stream to a plain accumulator: v1 = 2^k − 1,
    // then the 2n − 1 oscillation deltas (+1, −1, …) and the closing −2^k,
    // exactly the values a Tier 2 leaf sweep must materialize in order.
    let limb_ops_per_wire_bit = |scale: usize| {
        let (k, n) = (scale, scale);
        let one = Base::from(1u8);
        let mut v = (Base::from(1u8) << k as u32) - &one;
        crate::meter::reset_limb_ops();
        for i in 1..(2 * n) {
            v = if i % 2 == 1 { &v + &one } else { v - &one };
        }
        let closing = Base::from(1u8) << k as u32;
        v -= &closing;
        let ops = crate::meter::limb_ops();
        assert_eq!(
            v,
            Base::ZERO,
            "the delta stream telescopes back to the terminal leaf 0"
        );
        ops as f64 / (14 * n + 2) as f64
    };
    let small = limb_ops_per_wire_bit(512);
    let large = limb_ops_per_wire_bit(1024);
    assert!(
        large / small >= 1.8,
        "per-wire-bit limb cost must roughly double per size doubling \
         (measured {small:.2} then {large:.2} limb ops per wire bit): \
         a plain accumulator over the comb's delta stream is quadratic"
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
