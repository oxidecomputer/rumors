//! Differential tests for folding [`Base`] values into an accumulator.
//!
//! Suanpan tests its own representation. These tests cover Before's adapter:
//! word-sized values use the word operation, larger values stream their limbs,
//! and both paths agree with an independent big-integer oracle.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;
use suanpan::Accumulator;

use super::Base;

/// Word conversion succeeds exactly through `u64::MAX`.
#[test]
fn base_word_conversion_has_the_right_boundary() {
    assert_eq!(Base::from(u64::MAX).to_u64(), Some(u64::MAX));
    assert_eq!(Base::from(BigUint::from(u64::MAX) + 1u8).to_u64(), None);
}

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
    let (sign, magnitude) = Base::from_accumulator(acc);
    assert_eq!(sign, oracle_sign(oracle), "accumulator sign");
    let rebuilt = match sign {
        Ordering::Less => -BigInt::from(magnitude.0),
        _ => BigInt::from(magnitude.0),
    };
    assert_eq!(&rebuilt, oracle, "accumulator magnitude");
}

/// A wide magnitude from little-endian 64-bit limbs.
fn from_limbs(limbs: &[u64]) -> BigUint {
    let bytes: Vec<u8> = limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
    BigUint::from_bytes_le(&bytes)
}

proptest! {
    /// Folding unshifted `Base` values agrees with exact integer arithmetic.
    ///
    /// Values span one to three limbs, exercising both the word and limb paths.
    #[test]
    fn base_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3)),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = BigInt::from(0);
        for (negative, limbs) in &ops {
            let value = from_limbs(limbs);
            // One to three limbs per value, so the stream exercises the
            // word-sized dispatch path and the wide one both.
            let base = Base::from(value.clone());
            if *negative {
                base.fold_into(&mut acc, 0, true);
                oracle -= BigInt::from(value);
            } else {
                base.fold_into(&mut acc, 0, false);
                oracle += BigInt::from(value);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// Folding shifted `Base` values agrees with exact integer arithmetic.
    ///
    /// Arbitrary sub-digit and multi-digit shifts are checked after every fold
    /// and at the final value.
    #[test]
    fn shifted_base_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = BigInt::from(0);
        for (negative, limbs, shift) in &ops {
            let value = from_limbs(limbs);
            let base = Base::from(value.clone());
            if *negative {
                base.fold_into(&mut acc, *shift, true);
                oracle -= BigInt::from(value << *shift as usize);
            } else {
                base.fold_into(&mut acc, *shift, false);
                oracle += BigInt::from(value << *shift as usize);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }
}
