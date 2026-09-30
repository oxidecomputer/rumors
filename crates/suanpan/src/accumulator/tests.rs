//! Compare exact arithmetic with an independent big-integer oracle.
//!
//! Starting each schedule in both representations protects the scalar fast
//! path as well as digit arithmetic. Inspecting the private invariants catches
//! stale zero ranges that might corrupt a later query even when the current
//! value still reads correctly.

mod differential;
#[cfg(feature = "touch-meter")]
mod metered;
mod primitives;
mod representation;
mod surface;
mod witnesses;

use core::cmp::Ordering;

use num_bigint::{BigInt as IBig, BigUint as UBig, Sign};

use super::Accumulator;

/// Start at zero in either the scalar or digit representation.
///
/// Forcing digits lets each schedule exercise both paths even when all its
/// values would fit in the scalar.
fn fresh(digits: bool) -> Accumulator {
    let mut acc = Accumulator::new();
    if digits {
        acc.ensure_digits();
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

/// Assert that both limb readouts denote the oracle's exact signed value.
fn assert_value(acc: &Accumulator, oracle: &IBig) {
    let expected_sign = oracle_sign(oracle);
    let (sign, limbs) = acc.signed_magnitude();
    assert_eq!(sign, expected_sign, "unscaled sign");
    assert_ne!(limbs.as_ref().last(), Some(&0), "no high zero limb");
    let magnitude = IBig::from(from_limbs(limbs.as_ref()));
    let rebuilt = if sign == Ordering::Less {
        -magnitude
    } else {
        magnitude
    };
    assert_eq!(&rebuilt, oracle, "unscaled value");

    let (sign, limbs, shift) = acc.scaled_signed_magnitude();
    assert_eq!(sign, expected_sign, "scaled sign");
    assert_ne!(
        limbs.as_ref().last(),
        Some(&0),
        "no high zero limb at scale"
    );
    let magnitude = IBig::from(from_limbs(limbs.as_ref())) << usize::try_from(shift).unwrap();
    let rebuilt = if sign == Ordering::Less {
        -magnitude
    } else {
        magnitude
    };
    assert_eq!(&rebuilt, oracle, "scaled value");
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
    /// Read the held value into the big-integer oracle representation.
    fn sign_biguint(&self) -> (Ordering, UBig);
    /// Read the held value and retained scale into the oracle representation.
    fn sign_biguint_shl(&self) -> (Ordering, UBig, u64);
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

    /// Build the oracle directly from the borrowed normalized limbs.
    fn sign_biguint(&self) -> (Ordering, UBig) {
        let (sign, limbs) = self.signed_magnitude();
        (sign, from_limbs(limbs.as_ref()))
    }

    /// Preserve the returned scale while converting the borrowed magnitude.
    fn sign_biguint_shl(&self) -> (Ordering, UBig, u64) {
        let (sign, limbs, shift) = self.scaled_signed_magnitude();
        (sign, from_limbs(limbs.as_ref()), shift)
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

/// Put the smallest permitted coefficient, `−(2^33 − 1)`, at `index`.
///
/// Both the intermediate and final coefficients stay strictly above `−2^33`,
/// so these word inputs do not carry. The resulting coefficient is useful
/// for testing the largest possible cancellation below a deciding high digit.
fn park_extreme_negative_digit(acc: &mut Accumulator, index: u64) {
    // Store both updates as digits so their sum remains one extreme signed
    // coefficient instead of being combined in the scalar.
    acc.ensure_digits();
    acc.sub_shifted_limbs(32 * index, [1u64 << 32]);
    acc.sub_shifted_limbs(32 * index, [(1u64 << 32) - 1]);
}
