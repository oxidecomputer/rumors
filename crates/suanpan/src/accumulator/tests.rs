//! Differential and structural tests for the accumulator, against an
//! exact `IBig` oracle.
//!
//! One submodule per category:
//!
//! - `differential`: randomized operation streams and deterministic
//!   adversarial shapes, each compared against the oracle — the sign
//!   after every operation, the full value at periodic snapshots.
//! - `metered` (`touch-meter` builds only): the exact digit-touch pins
//!   behind the claims roster's cost rows, including the adequacy
//!   tripwire that reads red on a sign fold without its collapse.
//! - `witnesses`: constructed corner cases no random stream reaches —
//!   the decision thresholds at their tight edges, the quick register's
//!   headroom extremes, and conversion-path corners.
//! - `ledger`: the zero-run ledger's structural invariants, held after
//!   every step of every schedule at exhaustive small scope and on
//!   randomized deep-shift streams.
//!
//! This file holds the shared harness: mode-forcing construction, the
//! oracle comparison, limb-built wide operands, and zone-edge digit
//! parking.

mod differential;
mod ledger;
#[cfg(feature = "touch-meter")]
mod metered;
mod witnesses;

use core::cmp::Ordering;

use dashu_int::{IBig, Sign, UBig};

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
            Sign::Negative => Ordering::Less,
            Sign::Positive => Ordering::Greater,
        }
    }
}

/// Assert the accumulator's full value equals the oracle's, sign and
/// magnitude both — through the plain read and the scaled one.
fn assert_value(acc: &Accumulator, oracle: &IBig) {
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(sign, oracle_sign(oracle), "sign_magnitude sign");
    // The sign was just asserted, so signing the magnitude with it makes
    // the magnitude comparison exact.
    let rebuilt = match sign {
        Ordering::Less => -IBig::from(magnitude),
        _ => IBig::from(magnitude),
    };
    assert_eq!(&rebuilt, oracle, "sign_magnitude magnitude");
    // The scaled read denotes the same value: ±magnitude · 2^shift.
    let (shl_sign, shl_magnitude, shift) = acc.sign_magnitude_shl();
    assert_eq!(shl_sign, sign, "sign_magnitude_shl sign");
    let scaled = IBig::from(shl_magnitude) << usize::try_from(shift).unwrap();
    let rebuilt = match shl_sign {
        Ordering::Less => -scaled,
        _ => scaled,
    };
    assert_eq!(&rebuilt, oracle, "sign_magnitude_shl magnitude at scale");
}

/// A wide magnitude from little-endian 64-bit limbs.
fn from_limbs(limbs: &[u64]) -> UBig {
    let bytes: Vec<u8> = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
    UBig::from_le_bytes(&bytes)
}

/// Deposit `−(2^33 − 1)` — the lazy zone's most negative digit — at
/// digit `index` through the public word-scale entry points, without
/// triggering a recenter.
///
/// Two deposits of `−2^32` and `−(2^32 − 1)` land in one digit because
/// each intermediate total stays inside the zone; a single deposit of
/// the full value would recenter. This is the construction behind the
/// extreme-cancellation witnesses and the differential suite's
/// accumulator-operand probes: an adversary (or an unlucky workload)
/// can park any digit one unit inside the zone boundary.
fn park_extreme_negative_digit(acc: &mut Accumulator, index: u64) {
    // The construction is a digit-engine spelling: arm the engine so
    // the register cannot fuse the two deposits into one exact value.
    acc.spill();
    acc.sub_magnitude_shl(&UBig::from(1u64 << 32), 32 * index);
    acc.sub_magnitude_shl(&UBig::from((1u64 << 32) - 1), 32 * index);
}
