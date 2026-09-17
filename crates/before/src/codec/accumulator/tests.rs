//! Differential tests for the `suanpan` conversion boundary.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;
use suanpan::Accumulator;

use crate::codec::accumulator;

/// The oracle's sign as the accumulator reports it.
fn oracle_sign(oracle: &BigInt) -> Ordering {
    if *oracle == BigInt::ZERO {
        Ordering::Equal
    } else {
        match oracle.sign() {
            Sign::Minus => Ordering::Less,
            Sign::Plus => Ordering::Greater,
            Sign::NoSign => unreachable!("zero was handled above"),
        }
    }
}

/// Assert the accumulator's full value equals the oracle.
fn assert_value(acc: &Accumulator, oracle: &BigInt) {
    let (sign, magnitude) = accumulator::value(acc);
    assert_eq!(sign, oracle_sign(oracle), "accumulator sign");
    let rebuilt = match sign {
        Ordering::Less => -BigInt::from(magnitude),
        _ => BigInt::from(magnitude),
    };
    assert_eq!(&rebuilt, oracle, "accumulator magnitude");
    assert_eq!(&accumulator::signed_value(acc), oracle, "signed read");
}

proptest! {
    /// Folding unshifted magnitudes agrees with exact integer arithmetic.
    #[test]
    fn accumulator_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u32>(), 1..=6)),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = BigInt::ZERO;
        for (negative, digits) in &ops {
            let value = BigUint::from_slice(digits);
            accumulator::fold(&mut acc, &value, 0, *negative);
            if *negative {
                oracle -= BigInt::from(value);
            } else {
                oracle += BigInt::from(value);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// Folding shifted magnitudes agrees with exact integer arithmetic.
    #[test]
    fn shifted_accumulator_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u32>(), 1..=6), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = BigInt::ZERO;
        for (negative, digits, shift) in &ops {
            let value = BigUint::from_slice(digits);
            accumulator::fold(&mut acc, &value, *shift, *negative);
            if *negative {
                oracle -= BigInt::from(value << *shift as usize);
            } else {
                oracle += BigInt::from(value << *shift as usize);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// Whole signed values add and subtract with the same sign behavior as
    /// exact integer arithmetic.
    #[test]
    fn signed_accumulator_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (
                any::<bool>(),
                any::<bool>(),
                proptest::collection::vec(any::<u32>(), 1..=6),
            ),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = BigInt::ZERO;
        for (negative, subtract, digits) in ops {
            let sign = if negative { Sign::Minus } else { Sign::Plus };
            let value = BigInt::from_biguint(sign, BigUint::from_slice(&digits));
            if subtract {
                accumulator::subtract_signed(&mut acc, &value);
                oracle -= value;
            } else {
                accumulator::fold_signed(&mut acc, &value);
                oracle += value;
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }
}
