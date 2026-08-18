//! Differential and dispatch tests for the `Base` seam into the
//! accumulator.
//!
//! The accumulator's own suite (in `suanpan`) covers the representation;
//! this file covers what this crate adds — the `Magnitude`
//! implementation on [`Base`]. The proptest streams drive the
//! width-dispatched entry points with stored magnitudes (inline and
//! spilled both) against an exact `IBig` oracle, the sign compared
//! after every operation and the full value at the end; the pointwise
//! pins hold the dispatch read itself at its contract edges — `to_word`
//! answers at word scale exactly and, under `limb-meter`, touches no
//! digits.

use core::cmp::Ordering;

use dashu_int::{IBig, Sign, UBig};
use proptest::prelude::*;
use suanpan::{Accumulator, Magnitude};

#[cfg(feature = "limb-meter")]
use suanpan::touch_meter;

use super::Base;

/// `Magnitude::to_word` on `Base` answers the width dispatch: a
/// word-scale magnitude reports its word, a spilled one defers to the
/// wide path, and both agree with `as_wide` on the value.
///
/// The semantic half of the dispatch pin; the touch pricing of the same
/// reads is the limb-meter-gated companion below. Adequacy: the two
/// arms answer differently, so a dispatch stuck on either path fails
/// one of them.
#[test]
fn base_dispatch_answers_at_word_scale() {
    let word_held = Base::from(7u64);
    let spilled = Base::from(UBig::ONE << 200usize);
    assert_eq!(
        Magnitude::to_word(&word_held),
        Some(7),
        "a word-scale magnitude reports its word"
    );
    assert_eq!(
        Magnitude::to_word(&spilled),
        None,
        "a spilled magnitude defers to the wide path"
    );
    // The trait's self-agreement rule: as_wide denotes the same value
    // to_word reports (the Magnitude rustdoc).
    assert_eq!(Magnitude::as_wide(&word_held), &UBig::from(7u8));
    assert_eq!(Magnitude::as_wide(&spilled), &(UBig::ONE << 200usize));
}

/// `Magnitude::to_word` on `Base` answers the width dispatch with zero
/// digit touches, word-held and spilled both: the O(1) dispatch read
/// the accumulator's small-path cost accounting assumes free.
///
/// The `Magnitude` rustdoc makes O(1) dispatch a contract on
/// implementors; this pin holds `Base`'s implementation to it in the
/// touch denomination. The touch counter is process-global, so the
/// reading is meaningful under the workspace runner's process-per-test
/// isolation (nextest), like every metered reading in this crate.
/// Adequacy leg (meter liveness): the same counter reads nonzero across
/// an actual digit-touching operation, so a dead meter cannot satisfy
/// the zero.
#[cfg(feature = "limb-meter")]
#[test]
fn base_dispatch_read_touches_no_digits() {
    let word_held = Base::from(7u64);
    let spilled = Base::from(UBig::ONE << 200usize);
    touch_meter::reset();
    assert_eq!(Magnitude::to_word(&word_held), Some(7));
    assert_eq!(Magnitude::to_word(&spilled), None);
    assert_eq!(
        touch_meter::touches(),
        0,
        "the dispatch read is word-scale: no digit touched on either arm"
    );
    // Meter liveness: the counter counts when digits actually move.
    let mut acc = Accumulator::new();
    acc.add_magnitude(&spilled);
    assert!(
        touch_meter::touches() > 0,
        "meter liveness: a wide add must touch digits"
    );
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
/// magnitude both.
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
}

/// A wide magnitude from little-endian 64-bit limbs.
fn from_limbs(limbs: &[u64]) -> UBig {
    let bytes: Vec<u8> = limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
    UBig::from_le_bytes(&bytes)
}

proptest! {
    /// A stream applied through `add_magnitude`/`sub_magnitude` matches the oracle
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
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for (negative, limbs) in &ops {
            let value = from_limbs(limbs);
            // One to three limbs per value, so the stream exercises the
            // word-sized dispatch path and the wide one both.
            let base = Base::from(value.clone());
            if *negative {
                acc.sub_magnitude(&base);
                oracle -= IBig::from(value);
            } else {
                acc.add_magnitude(&base);
                oracle += IBig::from(value);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// The shifted `Base` entry points hold `±x · 2^s` exactly.
    ///
    /// A stream mixing `add_magnitude_shl` and `sub_magnitude_shl` at arbitrary
    /// sub-digit and multi-digit shifts matches the oracle's explicitly
    /// shifted value at every sign and at the final value.
    #[test]
    fn shifted_base_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for (negative, limbs, shift) in &ops {
            let value = from_limbs(limbs);
            let base = Base::from(value.clone());
            if *negative {
                acc.sub_magnitude_shl(&base, *shift);
                oracle -= IBig::from(value << *shift as usize);
            } else {
                acc.add_magnitude_shl(&base, *shift);
                oracle += IBig::from(value << *shift as usize);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }
}
