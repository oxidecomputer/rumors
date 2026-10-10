//! Big-integer helpers for [`Accumulator`] beyond `suanpan`'s `num-bigint`
//! interop.
//!
//! Conversions to and from `BigInt` and `BigUint`, and big-integer operands,
//! come from `suanpan` itself. This module adds a magnitude's width in
//! accumulator digits and the scaled read, which returns a value's
//! power-of-two scale separately so that a known zero prefix is never
//! materialized.

use core::cmp::Ordering;

use num_bigint::BigUint;
use suanpan::Accumulator;

/// A magnitude's width in the accumulator's base-2^32 digits, at least one.
pub(crate) fn digit_len(magnitude: &BigUint) -> usize {
    usize::try_from(magnitude.bits().div_ceil(32))
        .expect("digit counts fit usize")
        .max(1)
}

/// Read a value as a sign, a magnitude, and a power-of-two scale.
///
/// The value is `±magnitude · 2^shift`, as
/// [`Accumulator::scaled_signed_magnitude`] returns it.
pub(crate) fn scaled_biguint_parts(value: &Accumulator) -> (Ordering, BigUint, u64) {
    let (sign, words, shift) = value.scaled_signed_magnitude();
    (sign, magnitude(words.as_ref()), shift)
}

/// Convert little-endian 64-bit words into a `BigUint`.
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
