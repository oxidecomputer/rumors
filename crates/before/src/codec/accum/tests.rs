//! Differential tests for the `Base` seam into the accumulator.
//!
//! The accumulator's own suite (in `suanpan`) covers the representation;
//! these streams cover what this crate adds — the `Magnitude`
//! implementation on [`Base`] — by driving the width-dispatched entry
//! points with stored magnitudes (inline and spilled both) against an
//! exact `IBig` oracle, the sign compared after every operation and the
//! full value at the end.

use core::cmp::Ordering;

use dashu_int::{IBig, Sign, UBig};
use proptest::prelude::*;

use super::Accum;
use crate::codec::Base;

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
/// magnitude both.
fn assert_value(acc: &Accum, oracle: &IBig) {
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(sign, oracle_sign(oracle), "sign_magnitude sign");
    // The sign was just asserted, so signing the magnitude with it makes
    // the magnitude comparison exact.
    let rebuilt = match sign {
        Ordering::Less => -IBig::from(magnitude),
        _ => IBig::from(magnitude),
    };
    assert_eq!(&rebuilt, oracle, "sign_magnitude magnitude");
}

/// A wide magnitude from little-endian 64-bit limbs.
fn from_limbs(limbs: &[u64]) -> UBig {
    let bytes: Vec<u8> = limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
    UBig::from_le_bytes(&bytes)
}

proptest! {
    /// A stream applied through `add_base`/`sub_base` matches the oracle
    /// at every sign and at the final value.
    ///
    /// Spilled and inline magnitudes both, so both arms of `Base`'s width
    /// dispatch fire.
    #[test]
    fn base_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3)),
            1..200,
        ),
    ) {
        let mut acc = Accum::new();
        let mut oracle = IBig::from(0);
        for (negative, limbs) in &ops {
            let value = from_limbs(limbs);
            // One to three limbs per value, so the stream exercises the
            // word-sized dispatch path and the wide one both.
            let base = Base::from(value.clone());
            if *negative {
                acc.sub_base(&base);
                oracle -= IBig::from(value);
            } else {
                acc.add_base(&base);
                oracle += IBig::from(value);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// The shifted `Base` entry points hold `±x · 2^s` exactly.
    ///
    /// A stream mixing `add_base_shl` and `sub_base_shl` at arbitrary
    /// sub-digit and multi-digit shifts matches the oracle's explicitly
    /// shifted value at every sign and at the final value.
    #[test]
    fn shifted_base_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accum::new();
        let mut oracle = IBig::from(0);
        for (negative, limbs, shift) in &ops {
            let value = from_limbs(limbs);
            let base = Base::from(value.clone());
            if *negative {
                acc.sub_base_shl(&base, *shift);
                oracle -= IBig::from(value << *shift as usize);
            } else {
                acc.add_base_shl(&base, *shift);
                oracle += IBig::from(value << *shift as usize);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }
}
