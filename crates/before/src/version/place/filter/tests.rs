//! Numeric comparison properties for the fused query filter.

use core::cmp::Ordering;

use num_bigint::BigUint;
use proptest::prelude::*;
use suanpan::Accumulator;

use super::{Comparison, HeightWidths};

/// Build a nonnegative height with an optional cancelling high prefix.
///
/// Adding `2^shift` and subtracting `2^shift - value` leaves `value` while
/// forcing the signed-digit representation to carry through the intervening
/// positions. Reversing the operations reaches the same value from below.
fn height(value: &BigUint, extra_digits: u64, subtract_first: bool) -> Accumulator {
    let mut height = Accumulator::new();
    if extra_digits == 0 {
        height.add_shifted_limbs(0, value.iter_u64_digits());
        return height;
    }

    let shift = value.bits().div_ceil(32).saturating_add(extra_digits) * 32;
    let cliff = BigUint::from(1u8) << usize::try_from(shift).expect("test shifts fit usize");
    let complement = &cliff - value;
    if subtract_first {
        height.sub_shifted_limbs(0, complement.iter_u64_digits());
        height.add_shifted_limbs(0, cliff.iter_u64_digits());
    } else {
        height.add_shifted_limbs(0, cliff.iter_u64_digits());
        height.sub_shifted_limbs(0, complement.iter_u64_digits());
    }
    height
}

/// Generate mathematical heights and cancellation patterns across the scalar
/// boundary, nearby widths, and widely separated stored widths.
fn arb_height() -> impl Strategy<Value = (BigUint, u64, bool)> {
    (
        proptest::collection::vec(any::<u32>(), 0..=16).prop_map(BigUint::new),
        prop_oneof![3 => 0u64..=3, 1 => 4u64..=24],
        any::<bool>(),
    )
}

proptest! {
    /// The bounded no-copy path agrees with exact nonnegative integer order.
    ///
    /// Independent cancellation patterns make either side start wider, make
    /// the first refusal expose the opposite side as wider, and cross the
    /// two-digit cache boundary. Whenever an exact difference is retained, its
    /// operands must have the width relation promised by the memory bound.
    #[test]
    fn height_comparison_matches_biguint(
        (probe_value, probe_extra, probe_subtracts_first) in arb_height(),
        (bound_value, bound_extra, bound_subtracts_first) in arb_height(),
    ) {
        let mut probe = height(&probe_value, probe_extra, probe_subtracts_first);
        let mut bound = height(&bound_value, bound_extra, bound_subtracts_first);
        let (actual, difference) = Comparison::compare_heights(&mut probe, &mut bound);
        let expected = probe_value.cmp(&bound_value);

        prop_assert_eq!(actual, expected);
        if difference.is_some() {
            prop_assert_eq!(HeightWidths::between(&probe, &bound), HeightWidths::Close);
        }
        if expected == Ordering::Equal {
            prop_assert!(difference.is_some(), "equal heights require an exact difference");
        }
    }
}
