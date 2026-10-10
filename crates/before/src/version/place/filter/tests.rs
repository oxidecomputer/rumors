//! Numeric comparison properties for the fused query filter.

use num_bigint::BigUint;
use proptest::prelude::*;
use suanpan::Accumulator;

use super::{close, Comparison, MAX_CLOSE_LEAD_BITS};

/// The working width a difference may add beyond its wider operand's for a
/// carry: a constant, so a difference stays proportional to its operands.
const CARRY_ALLOWANCE_BITS: u64 = 64;

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
    /// Height comparison agrees with exact nonnegative integer order, and keeps
    /// an exact difference exactly when the heights' working widths are
    /// close.
    ///
    /// Independent cancellation patterns make either side start wider, make
    /// the first refusal expose the opposite side as wider, and cross the
    /// cache boundary. A retained difference obeys the memory bound: built
    /// from operands at most [`MAX_CLOSE_LEAD_BITS`] apart in working width,
    /// it is at most that lead plus [`CARRY_ALLOWANCE_BITS`] wider than the
    /// narrower height.
    #[test]
    fn height_comparison_matches_biguint(
        (probe_value, probe_extra, probe_subtracts_first) in arb_height(),
        (bound_value, bound_extra, bound_subtracts_first) in arb_height(),
    ) {
        let probe = height(&probe_value, probe_extra, probe_subtracts_first);
        let bound = height(&bound_value, bound_extra, bound_subtracts_first);
        let narrower = probe.stored_bits().min(bound.stored_bits());
        let was_close = close(&probe, &bound);
        let (actual, difference) = Comparison::compare_heights(&probe, &bound);
        let expected = probe_value.cmp(&bound_value);

        prop_assert_eq!(actual, expected);
        prop_assert_eq!(difference.is_some(), was_close);
        if let Some(difference) = &difference {
            prop_assert!(
                difference.stored_bits() <= narrower + MAX_CLOSE_LEAD_BITS + CARRY_ALLOWANCE_BITS,
                "a retained difference of {} bits exceeds the memory bound over \
                 a narrower height of {narrower} bits",
                difference.stored_bits()
            );
        }
    }
}
