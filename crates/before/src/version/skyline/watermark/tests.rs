//! Direct tests of [`RangeMinima`]'s deferred-boundary and closing invariants.
//!
//! Fixed cases isolate the comparisons around the anchor and true minimum.
//! Properties vary value widths and the number of ranges.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;
use suanpan::Accumulator;

use super::{Close, RangeMinima};
use crate::codec::accumulator;

/// Return the offset for a value `n` below the running height.
fn below(n: u64) -> BigInt {
    BigInt::from_biguint(Sign::Minus, BigUint::from(n))
}

/// Return the offset for a value `n` below the running height.
fn below_magnitude(n: &BigUint) -> BigInt {
    BigInt::from_biguint(Sign::Minus, n.clone())
}

/// Resolving a deferred boundary leaves the tracked minimum unchanged.
///
/// Closing the inner range defers a distance of `1000`. An emission at `25`
/// makes the two compared distances similar enough to resolve that state.
/// Probes on both sides of the original minimum verify the result.
#[test]
fn resolving_a_deferred_boundary_preserves_the_minimum() {
    let mut minima: RangeMinima<()> = RangeMinima::new();
    minima.open(2);
    minima.emit_here(); // both ranges arm at v = 0
    minima.open(1);
    minima.fold_height(&BigInt::from(1000u64)); // h = 1000
    minima.emit_here(); // the inner range arms at v = 1000
    minima.close(); // Λ = 1000, A = 1000, m = 0
    assert!(minima.deferred_live(), "the close defers the boundary");
    minima.emit_offset(&below(975)); // v = 25, above m
    assert!(
        !minima.deferred_live(),
        "the comparison resolves the deferred distance"
    );
    // h = 1000 and m = 0 after resolution.
    assert_eq!(
        minima.compare_above(&below(1000)),
        Ordering::Equal,
        "the probe at the true minimum reads exact"
    );
    assert_eq!(
        minima.compare_above(&below(975)),
        Ordering::Greater,
        "a probe above the minimum reads above"
    );
    assert_eq!(
        minima.compare_above(&below(1001)),
        Ordering::Less,
        "a probe below the minimum reads below"
    );
}

/// An undercut accounts for a deferred boundary before moving outward.
///
/// Closing the inner range defers `A - m = 50`. The next emission lowers `m`
/// by `2^34`, so the drop propagated to the outer boundary must be `m - v`,
/// not the larger `A - v`. Probes before and after closing verify both minima.
#[test]
fn undercut_subtracts_the_deferred_distance_before_propagating() {
    const D: u64 = 1 << 36;
    const E: u64 = 1 << 34;
    let mut minima: RangeMinima<()> = RangeMinima::new();
    minima.open(2);
    minima.emit_here(); // both ranges arm at v = 0
    minima.fold_height(&BigInt::from(D));
    minima.open(1);
    minima.emit_here(); // the middle range arms at v = D
    minima.fold_height(&BigInt::from(50u64)); // h = D + 50
    minima.open(1);
    minima.emit_here(); // the inner range arms at v = D + 50
    minima.close(); // Λ = 50, A = D + 50, m = D
    assert!(minima.deferred_live(), "the close defers the boundary");
    minima.emit_offset(&below(50 + E)); // v = D - E
    assert!(
        !minima.deferred_live(),
        "the undercut consumes the deferred distance"
    );
    // The undercut seated the innermost minimum at v = D − E exactly.
    assert_eq!(
        minima.compare_above(&below(50 + E)),
        Ordering::Equal,
        "the probe at the undercut emission reads exact"
    );
    // Closing the dropped range exposes the outer minimum at zero. The
    // surviving boundary is correct only if propagation used m - v.
    minima.close();
    assert_eq!(
        minima.compare_above(&below(D + 50)),
        Ordering::Equal,
        "the probe at the outer minimum reads exact"
    );
    assert_eq!(
        minima.compare_above(&below(D + 49)),
        Ordering::Greater,
        "a probe above the outer minimum reads above"
    );
    assert_eq!(
        minima.compare_above(&below(D + 51)),
        Ordering::Less,
        "a probe below the outer minimum reads below"
    );
}

/// A value between the anchor and true minimum does not lower the minimum.
///
/// The deferred distance is `2^36`, while the value is only `50` below the
/// anchor. The deferred state remains and three probes establish that the
/// minimum is still zero.
#[test]
fn a_word_sized_value_between_anchor_and_minimum_preserves_the_minimum() {
    const D: u64 = 1 << 36;
    let mut minima: RangeMinima<()> = RangeMinima::new();
    minima.open(2);
    minima.emit_here(); // both ranges arm at v = 0
    minima.open(1);
    minima.fold_height(&BigInt::from(D)); // h = D
    minima.emit_here(); // the inner range arms at v = D
    minima.close(); // Λ = D, A = D, m = 0
    assert!(minima.deferred_live(), "the close defers the boundary");
    minima.fold_height(&BigInt::from(-50)); // h = D − 50
    minima.emit_here(); // v = D − 50: below the anchor, above the minimum
    assert!(
        minima.deferred_live(),
        "the comparison leaves the deferred distance intact"
    );
    // h = D - 50 and m = 0: the deferred distance still reaches the minimum.
    assert_eq!(
        minima.compare_above(&below(D - 50)),
        Ordering::Equal,
        "the probe at the true minimum reads exact"
    );
    assert_eq!(
        minima.compare_above(&below(D - 51)),
        Ordering::Greater,
        "a probe above the minimum reads above"
    );
    assert_eq!(
        minima.compare_above(&below(D - 49)),
        Ordering::Less,
        "a probe below the minimum reads below"
    );
}

/// The wide-accumulator path also preserves a minimum below the value.
///
/// A deferred distance of `2^200` forces wide storage, while a drop of `50`
/// keeps the expected ordering simple. Three probes establish the result.
#[test]
fn a_wide_deferred_distance_preserves_the_minimum() {
    let lambda = BigUint::from(1u8) << 200usize;
    let mut minima: RangeMinima<()> = RangeMinima::new();
    minima.open(2);
    minima.emit_here(); // both ranges arm at v = 0
    minima.open(1);
    minima.fold_height(&BigInt::from(lambda.clone())); // h = Λ
    minima.emit_here(); // the inner range arms at v = Λ
    minima.close(); // Λ = 2^200, A = Λ, m = 0
    assert!(minima.deferred_live(), "the close defers the boundary");
    minima.fold_height(&BigInt::from(-50)); // h = Λ − 50
    minima.emit_here(); // v = Λ − 50: below the anchor, above the minimum
    assert!(
        minima.deferred_live(),
        "the comparison leaves the deferred distance intact"
    );
    let height = &lambda - BigUint::from(50u8);
    assert_eq!(
        minima.compare_above(&below_magnitude(&height)),
        Ordering::Equal,
        "the probe at the true minimum reads exact"
    );
    assert_eq!(
        minima.compare_above(&below_magnitude(&(&height - BigUint::from(1u8)))),
        Ordering::Greater,
        "a probe above the minimum reads above"
    );
    assert_eq!(
        minima.compare_above(&below_magnitude(&(&height + BigUint::from(1u8)))),
        Ordering::Less,
        "a probe below the minimum reads below"
    );
}

proptest! {
    /// A wide undercut moves every follower by exactly `m - v`.
    ///
    /// One range has minimum zero and a follower at a known offset. The height
    /// falls by `2^b`, then an emission arrives another `k` lower. For these
    /// widths the leading digits decide the undercut without folding the small
    /// offset into the wide gap. Varying both widths and offsets verifies the
    /// resulting follower's sign and magnitude.
    #[test]
    fn a_wide_undercut_moves_live_followers_by_the_exact_drop(
        b in 128usize..=300,
        k in 1u64..=u64::from(u32::MAX),
        start in 0u64..=1_000_000,
    ) {
        const SLOT: usize = 0;
        let drop = BigUint::from(1u8) << b;
        let mut minima: RangeMinima<()> = RangeMinima::new();
        minima.open(1);
        minima.emit_here(); // A = m = 0
        let mut follower = Accumulator::new();
        accumulator::fold(&mut follower, &BigUint::from(start), 0, false);
        minima.follower_set(SLOT, follower); // m - X = start
        minima.fold_height(&-BigInt::from(drop.clone())); // h = −2^b
        minima.emit_offset(&below(k)); // v = -2^b - k
        let taken = minima.follower_take(SLOT);
        let moved = accumulator::into_signed_value(taken);
        let expected_drop = &drop + BigUint::from(k);
        prop_assert_eq!(
            moved.sign(),
            Sign::Minus,
            "the undercut moves the follower downward"
        );
        prop_assert_eq!(
            moved.magnitude(),
            &(expected_drop - BigUint::from(start)),
            "the follower moved by exactly m - v"
        );
    }
}

proptest! {
    /// A value between the true minimum and anchor preserves the minimum and
    /// permits a later undercut.
    ///
    /// Magnitudes cover word-sized and wide accumulators. Probes verify the
    /// first value, then a true undercut verifies that the deferred state
    /// remains usable.
    #[test]
    fn values_between_anchor_and_minimum_preserve_the_minimum(
        b in 34usize..=260,
        d in 1u64..=(1u64 << 33),
    ) {
        let lambda = BigUint::from(1u8) << b;
        let mut minima: RangeMinima<()> = RangeMinima::new();
        minima.open(2);
        minima.emit_here(); // both ranges arm at v = 0
        minima.open(1);
        minima.fold_height(&BigInt::from(lambda.clone())); // h = Λ
        minima.emit_here(); // the inner range arms at v = Λ
        minima.close(); // A = Λ, m = 0
        prop_assert!(minima.deferred_live(), "the close defers the boundary");
        minima.fold_height(&-BigInt::from(d)); // h = Λ − d
        minima.emit_here(); // v = Λ − d: strictly inside (m, A)
        // The minimum is still 0, whichever arm answered.
        let height = &lambda - BigUint::from(d);
        prop_assert_eq!(
            minima.compare_above(&below_magnitude(&height)),
            Ordering::Equal,
            "the probe at the true minimum reads exact"
        );
        prop_assert_eq!(
            minima.compare_above(&below_magnitude(&(&height - BigUint::from(1u8)))),
            Ordering::Greater,
            "a probe above the minimum reads above"
        );
        prop_assert_eq!(
            minima.compare_above(&below_magnitude(&(&height + BigUint::from(1u8)))),
            Ordering::Less,
            "a probe below the minimum reads below"
        );
        // The earlier comparison left enough state for a later undercut.
        minima.fold_height(&-BigInt::from(&height + BigUint::from(1u8))); // h = −1
        minima.emit_here(); // v = −1: past m = 0, a true undercut
        // Closing the dropped range exposes the outer range, whose minimum was
        // also lowered to -1 by the undercut.
        minima.close();
        minima.fold_height(&BigInt::from(100u64)); // h = 99
        prop_assert_eq!(
            minima.compare_above(&below(100)),
            Ordering::Equal,
            "the outer minimum equals the undercut emission"
        );
        prop_assert_eq!(
            minima.compare_above(&below(99)),
            Ordering::Greater,
            "a probe above the outer minimum reads above"
        );
        prop_assert_eq!(
            minima.compare_above(&below(101)),
            Ordering::Less,
            "a probe below the outer minimum reads below"
        );
    }
}

proptest! {
    /// Ranges armed by one emission still close one at a time.
    ///
    /// One emission gives `n` nested ranges the same minimum, represented by a
    /// zero run above one positive outer boundary. Each close consumes exactly
    /// one range, and the outer minimum remains unchanged.
    #[test]
    fn batch_armed_closes_consume_exactly_one_range_record(n in 1usize..40) {
        let mut minima: RangeMinima<()> = RangeMinima::new();
        minima.open(1);
        minima.emit_here(); // the outer range arms at v = 0
        minima.fold_height(&BigInt::from(7u64)); // h = 7
        minima.open(n as u64);
        minima.emit_here(); // all n inner ranges arm at v = 7: one boundary, n − 1 zeros
        for i in 0..n {
            if i < n - 1 {
                prop_assert!(
                    matches!(minima.close(), Close::Equal),
                    "each equal inner minimum closes separately"
                );
            } else {
                prop_assert!(
                    matches!(minima.close(), Close::Lower(())),
                    "the final inner close exposes the lower outer minimum"
                );
            }
        }
        prop_assert!(!minima.has_pending(), "the arming emission left nothing pending");
        prop_assert!(minima.armed(), "the outer range survives its inner ranges' closes");
        prop_assert_eq!(
            minima.compare_above(&below(7)),
            Ordering::Equal,
            "the probe at the outer minimum reads exact"
        );
        prop_assert_eq!(
            minima.compare_above(&below(6)),
            Ordering::Greater,
            "a probe above the outer minimum reads above"
        );
        prop_assert_eq!(
            minima.compare_above(&below(8)),
            Ordering::Less,
            "a probe below the outer minimum reads below"
        );
    }
}
