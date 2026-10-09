//! Primitive boundaries and ownership-sensitive arithmetic traits.

use core::cmp::Ordering;

use num_bigint::BigInt as IBig;
use proptest::prelude::*;

use super::{assert_value, fresh, Accumulator};
use crate::accumulator::small::SMALL_MAX;

/// Exercise every primitive conversion boundary and both update representations.
macro_rules! integer_boundaries {
    ($($test:ident: $integer:ty),* $(,)?) => {$(
        /// Construction, arithmetic, and extraction preserve both range endpoints;
        /// extraction rejects the neighboring integers without changing their value.
        #[test]
        fn $test() {
            let minimum = <$integer>::MIN;
            let maximum = <$integer>::MAX;
            for value in [minimum, minimum + 1, !0, 0, 1, maximum - 1, maximum] {
                let oracle = IBig::from(value);
                for spilled in [false, true] {
                    let mut constructed = Accumulator::from(value);
                    if spilled {
                        constructed.ensure_digits();
                    }
                    assert_value(&constructed, &oracle);
                    assert_eq!(<$integer>::try_from(constructed).unwrap(), value);

                    // A fitting result may retain a much wider cancelling
                    // prefix. Conversion must inspect its value, not its width.
                    let mut cancelled = Accumulator::from(value);
                    cancelled.add_shifted_limbs(256, [1]);
                    cancelled.sub_shifted_limbs(0, [u64::MAX; 4]);
                    cancelled -= 1;
                    assert!(cancelled.stored_digit_count() > 4);
                    assert_eq!(<$integer>::try_from(cancelled).unwrap(), value);

                    // Both register edges exercise full-width addition overflow
                    // and sign reversal; the digit arm exercises direct deposits.
                    for start in [-(SMALL_MAX as i128), -1, 0, 1, SMALL_MAX as i128] {
                        let mut total = Accumulator::from(start);
                        if spilled {
                            total.ensure_digits();
                        }
                        total += value;
                        assert_value(&total, &(IBig::from(start) + &oracle));
                        total -= value;
                        assert_value(&total, &IBig::from(start));
                        total -= value;
                        assert_value(&total, &(IBig::from(start) - &oracle));
                    }
                }
                assert_value(&(Accumulator::new() + value), &oracle);
                assert_value(&(Accumulator::new() - value), &(-&oracle));
            }

            let outside: [(Accumulator, IBig); 2] = [
                (Accumulator::from(minimum) - 1, IBig::from(minimum) - 1),
                (Accumulator::from(maximum) + 1, IBig::from(maximum) + 1),
            ];
            for (value, oracle) in outside {
                for spilled in [false, true] {
                    let mut value = value.clone();
                    if spilled {
                        value.ensure_digits();
                    }
                    let representation = format!("{value:?}");
                    let rejected = <$integer>::try_from(value).unwrap_err();
                    assert_eq!(format!("{rejected:?}"), representation, "failure retains representation");
                    assert_value(&rejected, &oracle);
                }
            }

            let values = [minimum, maximum, 1];
            let sum: Accumulator = values.into_iter().sum();
            let expected = values.into_iter().map(IBig::from).sum::<IBig>();
            assert_value(&sum, &expected);
            assert_value(&core::iter::empty::<$integer>().sum(), &IBig::from(0));
        }
    )*};
}

integer_boundaries!(
    signed_8: i8,
    signed_16: i16,
    signed_32: i32,
    signed_64: i64,
    signed_128: i128,
    signed_pointer: isize,
    unsigned_8: u8,
    unsigned_16: u16,
    unsigned_32: u32,
    unsigned_64: u64,
    unsigned_128: u128,
    unsigned_pointer: usize,
);

proptest! {
    /// Full-width signed and unsigned updates preserve arbitrary values through
    /// overflow of the signed intermediate and later exact cancellation.
    #[test]
    fn full_width_updates_match_the_oracle(
        signed: i128,
        unsigned: u128,
        start: i128,
        spilled: bool,
    ) {
        let mut total = Accumulator::from(start);
        if spilled {
            total.ensure_digits();
        }
        let mut oracle = IBig::from(start);
        total += signed;
        oracle += signed;
        assert_value(&total, &oracle);
        total -= unsigned;
        oracle -= unsigned;
        assert_value(&total, &oracle);
        total -= signed;
        oracle -= signed;
        assert_value(&total, &oracle);
        total += unsigned;
        assert_eq!(i128::try_from(total).unwrap(), start);
    }

    /// Non-digit-aligned requested widths round conservatively: every issued
    /// bound still exceeds a random adjustment below the requested bound.
    #[test]
    fn bit_bounds_cover_requested_adjustments(
        value: i128,
        bits in 0u64..=128,
        adjustment: u128,
        spilled: bool,
    ) {
        let mut held = Accumulator::from(value);
        if spilled {
            held.ensure_digits();
        }
        if let Some(sign) = held.cmp_zero_stable_under(bits) {
            prop_assert_eq!(sign, value.cmp(&0));
            let magnitude = adjustment & u128::MAX.checked_shr(128 - bits as u32).unwrap_or(0);
            prop_assert!(value.unsigned_abs() > magnitude);
        }
    }
}

/// Every primitive count type agrees on valid shifts. A negative count, or a
/// count beyond `u64::MAX` applied to a nonzero value, panics before changing
/// the receiver, while zero shifts by any nonnegative count.
#[test]
fn primitive_shift_counts_are_checked() {
    /// Cover every supported count type at identity and register/digit transitions.
    macro_rules! valid_counts {
        ($($integer:ty),* $(,)?) => {$(
            for shift in [0, 1, 30, 31, 64, 127] {
                let operand = Accumulator::from(-7);
                let shifted = operand << shift as $integer;
                assert_value(&shifted, &(IBig::from(-7) << shift as usize));
            }
        )*};
    }
    valid_counts!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

    /// Invalid counts must not partially mutate a nonzero receiver.
    macro_rules! invalid_counts {
        ($($count:expr),* $(,)?) => {$(
            let mut value = Accumulator::from(7);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| value <<= $count));
            assert!(result.is_err());
            assert_eq!(i64::try_from(value).unwrap(), 7);
        )*};
    }
    invalid_counts!(
        -1i8,
        -1i16,
        -1i32,
        -1i64,
        -1i128,
        -1isize,
        u128::from(u64::MAX) + 1,
        i128::from(u64::MAX) + 1
    );

    // A zero needs no digit positions at any shift, whether it is a known zero
    // or stored as digits that cancel.
    for count in [u128::from(u64::MAX) + 1, u128::MAX] {
        let mut known = Accumulator::new();
        known <<= count;
        assert!(known.is_known_zero());

        let mut cancelled = Accumulator::new();
        cancelled.add_shifted_limbs(32, [1, 0]);
        cancelled.sub_shifted_limbs(0, [1 << 32]);
        assert!(!cancelled.is_known_zero());
        cancelled <<= count;
        assert_eq!(cancelled.cmp_zero(), Ordering::Equal);
    }
    let mut zero = Accumulator::new();
    zero <<= i128::MAX;
    assert!(zero.is_known_zero());
}

/// Owned arithmetic keeps the selected buffer, preserves subtraction order,
/// and agrees with borrowed and owned iterator sums.
#[test]
fn owned_arithmetic_reuses_storage_and_preserves_order() {
    let mut scalar = Accumulator::new();
    scalar.add_shifted_limbs(256, [1]);
    scalar.reset();
    scalar += 7;
    let retained = scalar.digits.stored_digits().as_ptr();
    let scalar = -scalar;
    assert_eq!(scalar.digits.stored_digits().as_ptr(), retained);
    assert_eq!(i64::try_from(scalar).unwrap(), -7);

    for left_shift in [64, 128, 192] {
        for right_shift in [64, 128, 192] {
            let mut left = fresh(true);
            left.add_shifted_limbs(left_shift, [7]);
            let mut right = fresh(true);
            right.add_shifted_limbs(right_shift, [11]);
            let expected =
                (IBig::from(7) << left_shift as usize) + (IBig::from(11) << right_shift as usize);
            let selected = if right.stored_digit_count() > left.stored_digit_count() {
                right.digits.stored_digits().as_ptr()
            } else {
                left.digits.stored_digits().as_ptr()
            };
            let sum = left + right;
            assert_eq!(sum.digits.stored_digits().as_ptr(), selected);
            assert_value(&sum, &expected);

            let address = sum.digits.stored_digits().as_ptr();
            let negative = -sum;
            assert_eq!(negative.digits.stored_digits().as_ptr(), address);
            assert_value(&negative, &(-&expected));

            let values = [negative.clone(), Accumulator::from(13)];
            assert_value(&values.iter().sum(), &(-&expected + 13));
            assert_value(&values.into_iter().sum(), &(-&expected + 13));
            assert_value(&(Accumulator::from(13) + &negative), &(-&expected + 13));
            assert_value(&(Accumulator::from(13) - &negative), &(&expected + 13));
            assert_value(
                &(Accumulator::from(13) - negative.clone()),
                &(&expected + 13),
            );
            let mut difference = Accumulator::from(13);
            difference -= negative;
            assert_value(&difference, &(&expected + 13));
        }
    }
    assert_value(&core::iter::empty::<Accumulator>().sum(), &IBig::from(0));
    assert_value(&core::iter::empty::<&Accumulator>().sum(), &IBig::from(0));
}
