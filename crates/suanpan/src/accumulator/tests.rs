//! Compare exact arithmetic with an independent big-integer oracle.
//!
//! Starting each schedule in both representations protects the scalar fast
//! path as well as digit arithmetic. Inspecting the private invariants catches
//! a stale written set that might corrupt a later query even when the current
//! value still reads correctly.

mod copies;
mod differential;
#[cfg(feature = "num-bigint")]
mod interop;
#[cfg(feature = "touch-meter")]
mod metered;
mod one_sign;
mod pool;
mod primitives;
mod representation;
mod same_cell;
mod standard;
mod surface;
mod witnesses;

use core::cmp::Ordering;

use num_bigint::{BigInt as IBig, BigUint as UBig, Sign};

use super::value::Value;
use super::Accumulator;

/// Check a value's representation for the tests.
impl Value {
    /// Check the digit invariants after `trace`, including that a small value
    /// leaves every retained digit zero.
    fn assert_invariants(&self, trace: &[u8]) {
        self.digits.assert_invariants(self.small.is_some(), trace);
    }
}

/// A test-only copy that leaves the source's stored form untouched.
impl Accumulator {
    /// Copy the stored form exactly, without reading or normalizing the
    /// source, so that observing a copy never changes the tested history.
    ///
    /// [`Clone`] normalizes its source in place; tests that inspect a value
    /// mid-history read this copy instead.
    fn stored_copy(&self) -> Accumulator {
        Accumulator::from_value(self.peek(Value::clone))
    }
}

/// Start at zero in either the scalar or digit representation.
///
/// Forcing digits lets each schedule exercise both paths even when all its
/// values would fit in the scalar.
fn fresh(digits: bool) -> Accumulator {
    let mut acc = Accumulator::new();
    if digits {
        acc.value.get_mut().ensure_digits();
    }
    acc
}

/// The oracle's sign as the accumulator reports it.
fn oracle_sign(oracle: &IBig) -> Ordering {
    if *oracle == IBig::ZERO {
        Ordering::Equal
    } else {
        match oracle.sign() {
            Sign::Minus => Ordering::Less,
            Sign::Plus => Ordering::Greater,
            Sign::NoSign => Ordering::Equal,
        }
    }
}

/// Assert that both limb readouts denote the oracle's exact signed value, and
/// that each read keeps that value in the value's own working width.
///
/// Each read runs on its own stored copy, so the caller's stored form is unchanged
/// for later steps. A read keeps the normalized form it computes, so reading
/// the copy again checks that written-back form: it must hold the same value,
/// in a working width that is the value's own, with every digit invariant.
fn assert_value(acc: &Accumulator, oracle: &IBig) {
    let own_width = own_digits(oracle);
    for scaled in [false, true] {
        let mut read = acc.stored_copy();
        for repeated in [false, true] {
            assert_eq!(
                &read_value(&mut read, scaled),
                oracle,
                "value (scaled read: {scaled}, repeated: {repeated})"
            );
        }
        assert_eq!(
            read.stored_digit_count(),
            own_width,
            "a read leaves the value's own working width (scaled read: {scaled})"
        );
        read.stored().assert_invariants(&[]);
    }
}

/// The value's own working width in digits: the 32-bit digits of its
/// normalized magnitude, at least one.
fn own_digits(oracle: &IBig) -> usize {
    usize::try_from(oracle.magnitude().bits().div_ceil(32).max(1))
        .expect("tested values have addressable widths")
}

/// Read the exact signed value through one of the two limb readouts.
///
/// The readout's sign must be the value's, and its limbs must have no high
/// zero word.
fn read_value(acc: &mut Accumulator, scaled: bool) -> IBig {
    let (sign, limbs, shift) = if scaled {
        let (sign, limbs, shift) = acc.scaled_signed_magnitude();
        (sign, limbs.as_ref().to_vec(), shift)
    } else {
        let (sign, limbs) = acc.signed_magnitude();
        (sign, limbs.as_ref().to_vec(), 0)
    };
    assert_ne!(
        limbs.last(),
        Some(&0),
        "no high zero limb (scaled: {scaled})"
    );
    let magnitude = IBig::from(from_limbs(&limbs)) << usize::try_from(shift).unwrap();
    let value = if sign == Ordering::Less {
        -magnitude
    } else {
        magnitude
    };
    assert_eq!(
        sign,
        oracle_sign(&value),
        "sign agrees with the limbs (scaled: {scaled})"
    );
    value
}

/// A wide magnitude from little-endian 64-bit limbs.
fn from_limbs(limbs: &[u64]) -> UBig {
    let bytes: Vec<u8> = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
    UBig::from_bytes_le(&bytes)
}

/// Big-integer adapters that drive the public word and limb entry points.
trait TestBig {
    /// Add a normalized oracle value through the limb-stream path.
    fn add_limb_value(&mut self, value: &UBig);
    /// Subtract a normalized oracle value through the limb-stream path.
    fn sub_limb_value(&mut self, value: &UBig);
    /// Add a shifted oracle value through the limb-stream path.
    fn add_limb_value_shl(&mut self, value: &UBig, shift: u64);
    /// Subtract a shifted oracle value through the limb-stream path.
    fn sub_limb_value_shl(&mut self, value: &UBig, shift: u64);
    /// Add a shifted oracle value through the narrowest applicable path.
    fn add_value_shl(&mut self, value: &UBig, shift: u64);
    /// Subtract a shifted oracle value through the narrowest applicable path.
    fn sub_value_shl(&mut self, value: &UBig, shift: u64);
    /// Read a stored copy of the held value into the big-integer oracle
    /// representation, leaving the stored form unchanged.
    fn sign_biguint(&self) -> (Ordering, UBig);
}

/// Keep the oracle library outside the production arithmetic paths.
impl TestBig for Accumulator {
    /// Stream an unscaled magnitude without copying its limbs.
    fn add_limb_value(&mut self, value: &UBig) {
        self.add_shifted_limbs(0, value.iter_u64_digits());
    }

    /// Subtract an unscaled magnitude without copying its limbs.
    fn sub_limb_value(&mut self, value: &UBig) {
        self.sub_shifted_limbs(0, value.iter_u64_digits());
    }

    /// Stream a magnitude directly into its shifted positions.
    fn add_limb_value_shl(&mut self, value: &UBig, shift: u64) {
        self.add_shifted_limbs(shift, value.iter_u64_digits());
    }

    /// Deposit negative contributions directly at the requested shift.
    fn sub_limb_value_shl(&mut self, value: &UBig, shift: u64) {
        self.sub_shifted_limbs(shift, value.iter_u64_digits());
    }

    /// Exercise array-backed word input whenever the oracle fits one limb.
    fn add_value_shl(&mut self, value: &UBig, shift: u64) {
        match oracle_word(value) {
            Some(word) => self.add_shifted_limbs(shift, [word]),
            None => self.add_limb_value_shl(value, shift),
        }
    }

    /// Exercise array-backed negative input whenever the oracle fits one limb.
    fn sub_value_shl(&mut self, value: &UBig, shift: u64) {
        match oracle_word(value) {
            Some(word) => self.sub_shifted_limbs(shift, [word]),
            None => self.sub_limb_value_shl(value, shift),
        }
    }

    /// Build the oracle from a stored copy's normalized limbs.
    ///
    /// Cloning keeps the tested history independent of where the test reads:
    /// an in-place read would normalize the value for every later step.
    fn sign_biguint(&self) -> (Ordering, UBig) {
        let (sign, limbs) = self.stored_copy().signed_magnitude();
        (sign, from_limbs(limbs.as_ref()))
    }
}

/// Return an oracle value as a word when it fits.
fn oracle_word(value: &UBig) -> Option<u64> {
    if value.bits() <= 64 {
        Some(value.iter_u64_digits().next().unwrap_or(0))
    } else {
        None
    }
}

/// Store zero as a top that cancels across two digits: `1` at `position` and
/// `−2^32` at `position − 1`.
///
/// The digits stay stored until a comparison with zero compacts them, so any
/// value added below `position − 1` sits under a working width that exceeds
/// its own. Both writes are single digits above a gap of unwritten zeros, so
/// a compacting scan crosses that gap in one skip.
///
/// # Panics
///
/// Panics if `position` is zero.
fn cancelled_top(position: u64) -> Accumulator {
    assert!(position >= 1, "the cancelled top needs a digit below it");
    let mut acc = Accumulator::new();
    acc.add_shifted_limbs(32 * position, [1]);
    acc.sub_shifted_limbs(32 * (position - 1), [1 << 32]);
    acc
}

/// Put the smallest permitted coefficient, `−(2^33 − 1)`, at `index`.
///
/// Both the intermediate and final coefficients stay strictly above `−2^33`,
/// so these word inputs do not carry. The resulting coefficient is useful
/// for testing the largest possible cancellation below a deciding high digit.
fn park_extreme_negative_digit(acc: &mut Accumulator, index: u64) {
    // Store both updates as digits so their sum remains one extreme signed
    // coefficient instead of being combined in the scalar.
    acc.value.get_mut().ensure_digits();
    acc.sub_shifted_limbs(32 * index, [1u64 << 32]);
    acc.sub_shifted_limbs(32 * index, [(1u64 << 32) - 1]);
}
