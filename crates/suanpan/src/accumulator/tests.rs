//! Differential and structural tests for the accumulator, against an
//! exact `IBig` oracle.
//!
//! The shared helpers construct either accumulator representation, compare
//! both readouts with an independent big-integer oracle, and build values
//! from limb streams.

mod differential;
mod ledger;
#[cfg(feature = "touch-meter")]
mod metered;
mod witnesses;

use core::cmp::Ordering;

use num_bigint::{BigInt as IBig, BigUint as UBig, Sign};

use super::Accumulator;

/// A fresh accumulator in the requested mode: the quick register, or
/// the digit engine armed by a forced spill — so every schedule drives
/// both starting modes and neither path's coverage goes vacuous.
fn fresh(engine: bool) -> Accumulator {
    let mut acc = Accumulator::new();
    if engine {
        acc.spill();
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
    let (limb_sign, limbs) = acc.sign_limbs();
    assert_eq!(limb_sign, oracle_sign(oracle), "sign_limbs sign");
    assert_ne!(
        limbs.last(),
        Some(&0),
        "sign_limbs limbs are minimal: no high zero limb"
    );
    let rebuilt = match limb_sign {
        Ordering::Less => -IBig::from(from_limbs(&limbs)),
        _ => IBig::from(from_limbs(&limbs)),
    };
    assert_eq!(&rebuilt, oracle, "sign_limbs magnitude");
    // The scaled read denotes the same value: ±magnitude · 2^shift.
    let (shl_sign, shl_limbs, shift) = acc.sign_limbs_shl();
    assert_eq!(shl_sign, limb_sign, "sign_limbs_shl sign");
    let scaled = IBig::from(from_limbs(&shl_limbs)) << usize::try_from(shift).unwrap();
    let rebuilt = match shl_sign {
        Ordering::Less => -scaled,
        _ => scaled,
    };
    assert_eq!(&rebuilt, oracle, "sign_limbs_shl magnitude at scale");
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

impl TestBig for Accumulator {
    fn add_limb_value(&mut self, value: &UBig) {
        self.add_limbs_shl(value.iter_u64_digits(), 0);
    }

    fn sub_limb_value(&mut self, value: &UBig) {
        self.sub_limbs_shl(value.iter_u64_digits(), 0);
    }

    fn add_limb_value_shl(&mut self, value: &UBig, shift: u64) {
        self.add_limbs_shl(value.iter_u64_digits(), shift);
    }

    fn sub_limb_value_shl(&mut self, value: &UBig, shift: u64) {
        self.sub_limbs_shl(value.iter_u64_digits(), shift);
    }

    fn add_value_shl(&mut self, value: &UBig, shift: u64) {
        match oracle_word(value) {
            Some(word) => self.add_u64_shl(word, shift),
            None => self.add_limb_value_shl(value, shift),
        }
    }

    fn sub_value_shl(&mut self, value: &UBig, shift: u64) {
        match oracle_word(value) {
            Some(word) => self.sub_u64_shl(word, shift),
            None => self.sub_limb_value_shl(value, shift),
        }
    }

    fn sign_biguint(&self) -> (Ordering, UBig) {
        let (sign, limbs) = self.sign_limbs();
        (sign, from_limbs(&limbs))
    }

    fn sign_biguint_shl(&self) -> (Ordering, UBig, u64) {
        let (sign, limbs, shift) = self.sign_limbs_shl();
        (sign, from_limbs(&limbs), shift)
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

/// Deposit `−(2^33 − 1)` — the lazy zone's most negative digit — at
/// digit `index` through the public word-scale entry points, without
/// triggering a recenter.
///
/// Two deposits of `−2^32` and `−(2^32 − 1)` land in one digit because
/// each intermediate total stays inside the zone; a single deposit of
/// the full value would recenter. This is the construction behind the
/// extreme-cancellation witnesses and the differential suite's
/// accumulator-operand probes: any digit can be parked one unit inside the
/// zone boundary.
fn park_extreme_negative_digit(acc: &mut Accumulator, index: u64) {
    // The construction is a digit-engine spelling: arm the engine so
    // the register cannot fuse the two deposits into one exact value.
    acc.spill();
    acc.sub_u64_shl(1u64 << 32, 32 * index);
    acc.sub_u64_shl((1u64 << 32) - 1, 32 * index);
}
