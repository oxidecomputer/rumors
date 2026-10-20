//! Conversions to and from `num-bigint`'s integers, and their use as operands.
//!
//! Enabled by the `num-bigint` feature. A big integer enters as a stream of
//! 64-bit limbs, exactly as [`add_shifted_limbs`](Accumulator::add_shifted_limbs)
//! takes it, and leaves through the normalizing
//! [`signed_magnitude`](Accumulator::signed_magnitude) read.

use core::cmp::Ordering;
use core::fmt;
use core::ops::{AddAssign, SubAssign};

use num_bigint::{BigInt, BigUint, Sign};

use super::Accumulator;

/// The error from converting a negative accumulator to a [`BigUint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NegativeValueError;

impl fmt::Display for NegativeValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a negative value has no unsigned representation")
    }
}

impl std::error::Error for NegativeValueError {}

/// Add a magnitude, at the cost of adding its limbs.
impl AddAssign<&BigUint> for Accumulator {
    fn add_assign(&mut self, rhs: &BigUint) {
        self.add_shifted_limbs(0, rhs.iter_u64_digits());
    }
}

/// Subtract a magnitude, at the cost of subtracting its limbs.
impl SubAssign<&BigUint> for Accumulator {
    fn sub_assign(&mut self, rhs: &BigUint) {
        self.sub_shifted_limbs(0, rhs.iter_u64_digits());
    }
}

/// Add a signed integer, at the cost of adding its magnitude's limbs.
impl AddAssign<&BigInt> for Accumulator {
    fn add_assign(&mut self, rhs: &BigInt) {
        match rhs.sign() {
            Sign::Minus => *self -= rhs.magnitude(),
            Sign::NoSign | Sign::Plus => *self += rhs.magnitude(),
        }
    }
}

/// Subtract a signed integer, at the cost of subtracting its magnitude's limbs.
impl SubAssign<&BigInt> for Accumulator {
    fn sub_assign(&mut self, rhs: &BigInt) {
        match rhs.sign() {
            Sign::Minus => *self += rhs.magnitude(),
            Sign::NoSign | Sign::Plus => *self -= rhs.magnitude(),
        }
    }
}

/// Owned big-integer operands, read as borrowed ones.
macro_rules! owned_operands {
    ($($integer:ty),*) => {$(
        /// Add the operand, as with a borrowed one.
        impl AddAssign<$integer> for Accumulator {
            fn add_assign(&mut self, rhs: $integer) {
                *self += &rhs;
            }
        }

        /// Subtract the operand, as with a borrowed one.
        impl SubAssign<$integer> for Accumulator {
            fn sub_assign(&mut self, rhs: $integer) {
                *self -= &rhs;
            }
        }

        /// Construct the same value, at the cost of adding it to zero.
        impl From<&$integer> for Accumulator {
            fn from(value: &$integer) -> Accumulator {
                let mut accumulator = Accumulator::new();
                accumulator += value;
                accumulator
            }
        }

        /// Construct the same value, at the cost of adding it to zero.
        impl From<$integer> for Accumulator {
            fn from(value: $integer) -> Accumulator {
                Accumulator::from(&value)
            }
        }
    )*};
}

owned_operands!(BigInt, BigUint);

/// Read the value, at the cost of
/// [`signed_magnitude`](Accumulator::signed_magnitude).
impl From<&Accumulator> for BigInt {
    fn from(value: &Accumulator) -> BigInt {
        let (sign, magnitude) = value.signed_magnitude();
        let sign = match sign {
            Ordering::Less => Sign::Minus,
            Ordering::Equal => Sign::NoSign,
            Ordering::Greater => Sign::Plus,
        };
        BigInt::from_biguint(sign, biguint(magnitude.as_ref()))
    }
}

/// Read a nonnegative value, at the cost of
/// [`signed_magnitude`](Accumulator::signed_magnitude).
///
/// # Errors
///
/// Returns [`NegativeValueError`] if the value is negative.
impl TryFrom<&Accumulator> for BigUint {
    type Error = NegativeValueError;

    fn try_from(value: &Accumulator) -> Result<BigUint, NegativeValueError> {
        let (sign, magnitude) = value.signed_magnitude();
        if sign == Ordering::Less {
            return Err(NegativeValueError);
        }
        Ok(biguint(magnitude.as_ref()))
    }
}

/// Build a `BigUint` from little-endian 64-bit limbs.
///
/// `num-bigint` has no constructor from 64-bit digits, so a wide magnitude
/// passes through a temporary buffer of 32-bit ones. One and two limbs convert
/// from a primitive instead, which needs no temporary buffer and allocates
/// nothing for a value that fits one of `BigUint`'s inline digits.
fn biguint(limbs: &[u64]) -> BigUint {
    match *limbs {
        [] => BigUint::ZERO,
        [low] => BigUint::from(low),
        [low, high] => BigUint::from(u128::from(low) | (u128::from(high) << 64)),
        _ => BigUint::new(
            limbs
                .iter()
                .flat_map(|&limb| [limb as u32, (limb >> 32) as u32])
                .collect(),
        ),
    }
}
