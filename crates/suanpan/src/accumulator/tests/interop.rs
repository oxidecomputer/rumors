//! Conversions to and from `num-bigint`'s integers, and their use as operands.

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;

use super::{assert_value, cancelled_top, Accumulator};
use crate::NegativeValueError;

/// Generate a signed big integer of up to six 32-bit digits.
fn arb_bigint() -> impl Strategy<Value = BigInt> {
    (
        any::<bool>(),
        proptest::collection::vec(any::<u32>(), 0..=6),
    )
        .prop_map(|(negative, digits)| {
            let sign = if negative { Sign::Minus } else { Sign::Plus };
            BigInt::from_biguint(sign, BigUint::new(digits))
        })
}

/// Generate a big integer and an accumulator holding it, either as its limbs
/// alone or under a top that cancels above them.
fn arb_stored() -> impl Strategy<Value = (Accumulator, BigInt)> {
    (arb_bigint(), proptest::option::of(7u64..=20)).prop_map(|(value, cancelled)| {
        let mut acc = cancelled.map_or_else(Accumulator::new, cancelled_top);
        acc += &value;
        (acc, value)
    })
}

proptest! {
    /// Adding and subtracting signed big integers, borrowed and owned, agrees
    /// with exact arithmetic after every step, and the result converts back to
    /// the same value.
    ///
    /// Each step reads a copy, so redundant digits from earlier operands stay
    /// in the stored form the next operand lands on.
    #[test]
    fn bigint_operands_match_the_oracle(
        ops in proptest::collection::vec((any::<bool>(), any::<bool>(), arb_bigint()), 1..100),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = BigInt::ZERO;
        for (subtract, owned, value) in ops {
            match (subtract, owned) {
                (false, false) => acc += &value,
                (false, true) => acc += value.clone(),
                (true, false) => acc -= &value,
                (true, true) => acc -= value.clone(),
            }
            if subtract {
                oracle -= value;
            } else {
                oracle += value;
            }
            // Reading a copy leaves the history's redundant digits in place.
            prop_assert_eq!(BigInt::from(&acc.clone()), oracle.clone());
        }
        assert_value(&acc, &oracle);
    }

    /// Unsigned big-integer operands and conversions from either big-integer
    /// type construct exactly the converted value.
    #[test]
    fn conversions_from_bigints_are_exact(value in arb_bigint()) {
        assert_value(&Accumulator::from(&value), &value);
        assert_value(&Accumulator::from(value.clone()), &value);
        let magnitude = value.magnitude().clone();
        let unsigned = BigInt::from(magnitude.clone());
        assert_value(&Accumulator::from(&magnitude), &unsigned);
        assert_value(&Accumulator::from(magnitude.clone()), &unsigned);
        let mut acc = Accumulator::from(&value);
        acc -= &magnitude;
        acc += magnitude;
        assert_value(&acc, &value);
    }

    /// Converting to `BigInt` returns the exact value, and converting to
    /// `BigUint` returns the magnitude of every nonnegative value and refuses
    /// every negative one, in either stored form, at every width through six
    /// digits.
    #[test]
    fn conversions_to_bigints_are_exact((acc, value) in arb_stored()) {
        prop_assert_eq!(BigInt::from(&acc.clone()), value.clone());
        let unsigned = BigUint::try_from(&acc);
        if value.sign() == Sign::Minus {
            prop_assert_eq!(unsigned, Err(NegativeValueError));
        } else {
            prop_assert_eq!(unsigned, Ok(value.magnitude().clone()));
        }
        assert_value(&acc, &value);
    }
}
