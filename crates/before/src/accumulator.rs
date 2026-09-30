//! Interoperation between [`Accumulator`] and `num_bigint` integers.
//!
//! Accumulation needs no big-integer allocation: callers stream unsigned
//! magnitudes with `iter_u64_digits()`, and signed inputs select addition or
//! subtraction here. Output methods construct `BigUint` or `BigInt` only when
//! an algorithm needs a normalized result. Values that fit in `u128` cross
//! this boundary without allocating an intermediate limb buffer.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan::Accumulator;

/// Signed input and normalized output at the big-integer boundary.
pub(crate) trait BigIntAccumulator {
    /// Read the value as a sign and unsigned magnitude.
    fn biguint_parts(&self) -> (Ordering, BigUint);

    /// Read the value as a sign, magnitude, and retained power-of-two scale.
    fn scaled_biguint_parts(&self) -> (Ordering, BigUint, u64);

    /// Read the value as a normalized signed integer.
    fn to_bigint(&self) -> BigInt;

    /// Normalize and consume the value as a signed integer.
    fn into_bigint(self) -> BigInt;

    /// Add a signed integer.
    fn add_bigint(&mut self, value: &BigInt);

    /// Subtract a signed integer.
    fn sub_bigint(&mut self, value: &BigInt);
}

/// A magnitude's width in the accumulator's base-2^32 digits (minimum one).
pub(crate) fn digit_len(magnitude: &BigUint) -> usize {
    usize::try_from(magnitude.bits().div_ceil(32))
        .expect("digit counts fit usize")
        .max(1)
}

/// Convert directly between limb streams and big-integer values.
impl BigIntAccumulator for Accumulator {
    /// Build an owned magnitude from the accumulator's borrowed limb output.
    fn biguint_parts(&self) -> (Ordering, BigUint) {
        let (sign, words) = Accumulator::signed_magnitude(self);
        (sign, magnitude(words.as_ref()))
    }

    /// Retain the known power-of-two scale instead of allocating its low zeros.
    fn scaled_biguint_parts(&self) -> (Ordering, BigUint, u64) {
        let (sign, words, shift) = Accumulator::scaled_signed_magnitude(self);
        (sign, magnitude(words.as_ref()), shift)
    }

    /// Attach the exact sign to the normalized magnitude.
    fn to_bigint(&self) -> BigInt {
        let (sign, magnitude) = self.biguint_parts();
        BigInt::from_biguint(
            if sign == Ordering::Less {
                Sign::Minus
            } else {
                Sign::Plus
            },
            magnitude,
        )
    }

    /// Collapse cancellation while the consumed accumulator is still mutable.
    fn into_bigint(mut self) -> BigInt {
        // Collapse a redundant high prefix before allocating the normalized
        // output, so conversion visits only the value's final width.
        self.cmp_zero();
        self.to_bigint()
    }

    /// Stream the magnitude through the operation selected by its sign.
    fn add_bigint(&mut self, value: &BigInt) {
        if value.sign() == Sign::Minus {
            self.sub_shifted_limbs(0, value.magnitude().iter_u64_digits());
        } else {
            self.add_shifted_limbs(0, value.magnitude().iter_u64_digits());
        }
    }

    /// Reverse the signed input's operation without allocating its negation.
    fn sub_bigint(&mut self, value: &BigInt) {
        if value.sign() == Sign::Minus {
            self.add_shifted_limbs(0, value.magnitude().iter_u64_digits());
        } else {
            self.sub_shifted_limbs(0, value.magnitude().iter_u64_digits());
        }
    }
}

/// Convert suanpan's little-endian 64-bit words into `BigUint`'s 32-bit digits.
fn magnitude(words: &[u64]) -> BigUint {
    match words {
        [] => BigUint::ZERO,
        &[low] => BigUint::from(low),
        &[low, high] => BigUint::from(u128::from(low) | (u128::from(high) << 64)),
        _ => BigUint::new(
            words
                .iter()
                .flat_map(|&word| [word as u32, (word >> 32) as u32])
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests;
