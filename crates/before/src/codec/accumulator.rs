//! Conversion between arbitrary-precision magnitudes and [`Accumulator`].
//!
//! The two integer crates remain independent. This adapter streams 64-bit
//! words between them and preserves their word-sized fast paths.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan::Accumulator;

/// A magnitude's width in the accumulator's base-2^32 digits (minimum one).
pub(crate) fn digit_len(magnitude: &BigUint) -> usize {
    usize::try_from(magnitude.bits().div_ceil(32))
        .expect("digit counts fit usize")
        .max(1)
}

/// The accumulator's occupied prefix in bits, rounded to its base-2^32 digit
/// boundary.
pub(crate) fn bit_span(acc: &Accumulator) -> u64 {
    u64::try_from(acc.digit_count())
        .expect("an allocated accumulator's digit count fits u64")
        .checked_mul(32)
        .expect("an allocated accumulator's bit span fits u64")
}

/// Read an accumulator into a `BigUint` magnitude.
pub(crate) fn value(acc: &Accumulator) -> (Ordering, BigUint) {
    acc.with_sign_limbs(|sign, words| (sign, magnitude(words)))
}

/// Read an accumulator as a magnitude with a retained power-of-two scale.
pub(crate) fn value_shl(acc: &Accumulator) -> (Ordering, BigUint, u64) {
    acc.with_sign_limbs_shl(|sign, words, shift| (sign, magnitude(words), shift))
}

/// Read an accumulator as a normalized signed integer.
pub(crate) fn signed_value(acc: &Accumulator) -> BigInt {
    let (sign, magnitude) = value(acc);
    BigInt::from_biguint(
        if sign == Ordering::Less {
            Sign::Minus
        } else {
            Sign::Plus
        },
        magnitude,
    )
}

/// Normalize and consume an accumulator as a signed integer.
///
/// The sign read first collapses redundant leading digits. Converting the
/// magnitude then visits only the normalized value's width.
pub(crate) fn into_signed_value(mut acc: Accumulator) -> BigInt {
    acc.sign();
    signed_value(&acc)
}

/// Add a signed integer to an accumulator.
pub(crate) fn fold_signed(acc: &mut Accumulator, value: &BigInt) {
    fold(acc, value.magnitude(), 0, value.sign() == Sign::Minus);
}

/// Subtract a signed integer from an accumulator.
pub(crate) fn subtract_signed(acc: &mut Accumulator, value: &BigInt) {
    fold(acc, value.magnitude(), 0, value.sign() != Sign::Minus);
}

/// Fold `magnitude * 2^shift` into an accumulator.
pub(crate) fn fold(acc: &mut Accumulator, magnitude: &BigUint, shift: u64, subtract: bool) {
    // The limb-stream methods spill suanpan's inline accumulator even for one
    // word. Preserve that representation when both integers fit their inline
    // paths; wider magnitudes stream without an intermediate copy.
    match u64::try_from(magnitude) {
        Ok(word) if subtract => acc.sub_u64_shl(word, shift),
        Ok(word) => acc.add_u64_shl(word, shift),
        Err(_) if subtract => acc.sub_limbs_shl(magnitude.iter_u64_digits(), shift),
        Err(_) => acc.add_limbs_shl(magnitude.iter_u64_digits(), shift),
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
