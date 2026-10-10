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

proptest! {
    /// Adding and subtracting signed big integers, borrowed and owned, agrees
    /// with exact arithmetic, and the result converts back to the same value.
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
            prop_assert_eq!(BigInt::from(&acc), oracle.clone());
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
}

/// Converting to `BigUint` reads a nonnegative value, including one under a
/// cancelled top, and refuses a negative one.
#[test]
fn biguint_conversion_refuses_negative_values() {
    let mut positive = cancelled_top(10);
    positive += 7;
    assert_eq!(BigUint::try_from(&positive), Ok(BigUint::from(7u8)));
    assert_eq!(BigUint::try_from(&cancelled_top(10)), Ok(BigUint::ZERO));
    let mut negative = cancelled_top(10);
    negative -= 7;
    assert_eq!(BigUint::try_from(&negative), Err(NegativeValueError));
    assert_eq!(BigInt::from(&negative), BigInt::from(-7));
}
