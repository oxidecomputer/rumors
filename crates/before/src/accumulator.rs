//! Interoperation between [`Accumulator`] and `num_bigint` integers.
//!
//! `before` stores exact integers with `num_bigint` and uses `suanpan` for
//! running sums. This module keeps that dependency boundary in one place:
//! magnitudes stream between the representations without a shifted copy, and
//! algorithms operate on the accumulator through integer-shaped methods.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan::Accumulator;

/// `num_bigint` operations on a `suanpan` accumulator.
pub(crate) trait BigIntAccumulator {
    /// Read the value as a sign and unsigned magnitude.
    fn signed_magnitude(&self) -> (Ordering, BigUint);

    /// Read the value as a sign, magnitude, and retained power-of-two scale.
    fn scaled_signed_magnitude(&self) -> (Ordering, BigUint, u64);

    /// Read the value as a normalized signed integer.
    fn to_bigint(&self) -> BigInt;

    /// Normalize and consume the value as a signed integer.
    fn into_bigint(self) -> BigInt;

    /// Add a signed integer.
    fn add_bigint(&mut self, value: &BigInt);

    /// Subtract a signed integer.
    fn sub_bigint(&mut self, value: &BigInt);

    /// Add `magnitude * 2^shift`.
    fn add_biguint_shl(&mut self, magnitude: &BigUint, shift: u64);

    /// Subtract `magnitude * 2^shift`.
    fn sub_biguint_shl(&mut self, magnitude: &BigUint, shift: u64);
}

/// A magnitude's width in the accumulator's base-2^32 digits (minimum one).
pub(crate) fn digit_len(magnitude: &BigUint) -> usize {
    usize::try_from(magnitude.bits().div_ceil(32))
        .expect("digit counts fit usize")
        .max(1)
}

/// Streams big-integer magnitudes through suanpan and materializes them only
/// when a caller needs an owned result.
impl BigIntAccumulator for Accumulator {
    fn signed_magnitude(&self) -> (Ordering, BigUint) {
        self.with_sign_limbs(|sign, words| (sign, magnitude(words)))
    }

    fn scaled_signed_magnitude(&self) -> (Ordering, BigUint, u64) {
        self.with_sign_limbs_shl(|sign, words, shift| (sign, magnitude(words), shift))
    }

    fn to_bigint(&self) -> BigInt {
        let (sign, magnitude) = self.signed_magnitude();
        BigInt::from_biguint(
            if sign == Ordering::Less {
                Sign::Minus
            } else {
                Sign::Plus
            },
            magnitude,
        )
    }

    fn into_bigint(mut self) -> BigInt {
        // Collapse a redundant high prefix before allocating the normalized
        // output, so conversion visits only the value's final width.
        self.sign();
        self.to_bigint()
    }

    fn add_bigint(&mut self, value: &BigInt) {
        if value.sign() == Sign::Minus {
            self.sub_biguint_shl(value.magnitude(), 0);
        } else {
            self.add_biguint_shl(value.magnitude(), 0);
        }
    }

    fn sub_bigint(&mut self, value: &BigInt) {
        if value.sign() == Sign::Minus {
            self.add_biguint_shl(value.magnitude(), 0);
        } else {
            self.sub_biguint_shl(value.magnitude(), 0);
        }
    }

    fn add_biguint_shl(&mut self, magnitude: &BigUint, shift: u64) {
        self.add_limbs_shl(magnitude.iter_u64_digits(), shift);
    }

    fn sub_biguint_shl(&mut self, magnitude: &BigUint, shift: u64) {
        self.sub_limbs_shl(magnitude.iter_u64_digits(), shift);
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
