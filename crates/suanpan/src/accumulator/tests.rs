//! Differential tests for the accumulator against an exact `IBig` oracle.
//!
//! Every stream compares the sign after every operation — the read a
//! consumer's sweeps depend on — and snapshots the full value periodically
//! through [`Accumulator::sign_magnitude`]. The deterministic streams are
//! the adversarial shapes the representation exists to survive: the
//! boundary-comb ±1 oscillation across a high carry cliff, wide teeth
//! across a higher cliff, and cancelling-prefix chains that force the sign
//! fold below the top digit.

use core::cmp::Ordering;

use dashu_int::ops::BitTest;
use dashu_int::{IBig, Sign, UBig};
use proptest::prelude::*;

use super::{Accumulator, QUICK_MAX, QUICK_SHIFT_MAX};

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

/// One accumulator operation, oracle-applicable.
#[derive(Debug, Clone)]
enum Op {
    /// A signed machine-word delta.
    Small(i64),
    /// A wide delta with an explicit sign.
    Wide { negative: bool, value: UBig },
    /// A wide delta scaled by `2^shift`, entering above digit zero.
    WideShl {
        negative: bool,
        value: UBig,
        shift: u64,
    },
}

/// Apply one operation to the accumulator and the oracle in lockstep.
fn apply(acc: &mut Accumulator, oracle: &mut IBig, op: &Op) {
    match op {
        Op::Small(delta) => {
            acc.add_small(*delta);
            *oracle += *delta;
        }
        Op::Wide { negative, value } => {
            if *negative {
                acc.sub_wide(value);
                *oracle -= IBig::from(value.clone());
            } else {
                acc.add_wide(value);
                *oracle += IBig::from(value.clone());
            }
        }
        Op::WideShl {
            negative,
            value,
            shift,
        } => {
            let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();
            if *negative {
                acc.sub_wide_shl(value, *shift);
                *oracle -= scaled;
            } else {
                acc.add_wide_shl(value, *shift);
                *oracle += scaled;
            }
        }
    }
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
    let bytes: Vec<u8> = limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
    UBig::from_le_bytes(&bytes)
}

/// A mixed operation stream: mostly small deltas of varying width, some
/// dense random wide deltas, and some all-ones/all-zeros "cliffy" wide
/// deltas whose application sits exactly on carry boundaries.
fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (any::<i64>(), 0u32..60).prop_map(|(v, s)| Op::Small(v >> s)),
        1 => (proptest::collection::vec(any::<u64>(), 1..=6), any::<bool>()).prop_map(
            |(limbs, negative)| Op::Wide {
                negative,
                value: from_limbs(&limbs),
            }
        ),
        1 => (proptest::collection::vec(any::<bool>(), 1..=6), any::<bool>()).prop_map(
            |(mask, negative)| {
                let mut limbs: Vec<u64> =
                    mask.iter().map(|&m| if m { u64::MAX } else { 0 }).collect();
                if limbs.iter().all(|&l| l == 0) {
                    limbs[0] = 1;
                }
                Op::Wide {
                    negative,
                    value: from_limbs(&limbs),
                }
            }
        ),
        1 => (
            proptest::collection::vec(any::<u64>(), 1..=4),
            any::<bool>(),
            0u64..512,
        )
            .prop_map(|(limbs, negative, shift)| Op::WideShl {
                negative,
                value: from_limbs(&limbs),
                shift,
            }),
    ]
}

proptest! {
    /// On random mixed small/wide streams the accumulator's sign matches an
    /// exact `IBig` oracle after every single operation, and the full
    /// value matches at periodic snapshots and at the end.
    #[test]
    fn mixed_streams_match_the_bigint_oracle(
        ops in proptest::collection::vec(arb_op(), 1..300),
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        for (step, op) in ops.iter().enumerate() {
            apply(&mut acc, &mut oracle, op);
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle), "sign at step {}", step);
            if step % 64 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }

    /// The width-dispatching entry points agree with the raw wide/small
    /// entry points against the oracle.
    ///
    /// A stream applied through `add_magnitude`/`sub_magnitude` — via the
    /// [`Magnitude`](crate::Magnitude) implementation on `UBig`,
    /// word-scale and wide values both — matches the oracle at every
    /// sign and at the final value.
    #[test]
    fn magnitude_entry_points_match_the_oracle(
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
            if *negative {
                acc.sub_magnitude(&value);
                oracle -= IBig::from(value);
            } else {
                acc.add_magnitude(&value);
                oracle += IBig::from(value);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// The shifted entry points hold `x · 2^s` exactly.
    ///
    /// A stream applied through `add_magnitude_shl` at arbitrary sub-digit and
    /// multi-digit shifts, mixed with unshifted subtractions, matches the
    /// oracle's explicitly shifted value at every sign and at the final
    /// value.
    #[test]
    fn shifted_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for (negative, limbs, shift) in &ops {
            let value = from_limbs(limbs);
            if *negative {
                acc.sub_wide(&value);
                oracle -= IBig::from(value);
            } else {
                acc.add_magnitude_shl(&value, *shift);
                oracle += IBig::from(value << *shift as usize);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// The shifted subtraction entry points hold `−x · 2^s` exactly: a
    /// stream mixing `sub_magnitude_shl` and `sub_wide_shl` at arbitrary shifts
    /// with unshifted additions matches the oracle at every sign and at
    /// the final value.
    #[test]
    fn shifted_subtraction_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (0u8..3, proptest::collection::vec(any::<u64>(), 1..=3), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for (arm, limbs, shift) in &ops {
            let value = from_limbs(limbs);
            match arm {
                0 => {
                    acc.sub_magnitude_shl(&value, *shift);
                    oracle -= IBig::from(value << *shift as usize);
                }
                1 => {
                    acc.sub_wide_shl(&value, *shift);
                    oracle -= IBig::from(value << *shift as usize);
                }
                _ => {
                    acc.add_wide(&value);
                    oracle += IBig::from(value);
                }
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// Accumulator-into-accumulator merges preserve the value: building two
    /// operands from independent streams and merging one into the other at
    /// an arbitrary shift, added or subtracted, equals the oracle's
    /// `x ± y · 2^s`.
    #[test]
    fn merges_match_the_oracle(
        x_ops in proptest::collection::vec(arb_op(), 1..60),
        y_ops in proptest::collection::vec(arb_op(), 1..60),
        merge_shift in 0u64..200,
        subtract: bool,
    ) {
        let mut x = Accumulator::new();
        let mut x_oracle = IBig::from(0);
        for op in &x_ops {
            apply(&mut x, &mut x_oracle, op);
        }
        let mut y = Accumulator::new();
        let mut y_oracle = IBig::from(0);
        for op in &y_ops {
            apply(&mut y, &mut y_oracle, op);
        }
        if subtract {
            x.sub_accum_shl(&y, merge_shift);
            x_oracle -= y_oracle << merge_shift as usize;
        } else {
            x.add_accum_shl(&y, merge_shift);
            x_oracle += y_oracle << merge_shift as usize;
        }
        assert_value(&x, &x_oracle);
    }

    /// `digit_count` covers the held width: after any stream, the value's
    /// magnitude fits inside the counted digits (each digit spans 32 bits
    /// plus one lazy-zone bit of overhang).
    #[test]
    fn size_probe_covers_the_value(
        ops in proptest::collection::vec(arb_op(), 1..120),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
            let (_, magnitude) = acc.sign_magnitude();
            prop_assert!(
                u64::try_from(acc.digit_count()).expect("digit counts fit u64") * 32 + 33
                    >= magnitude.bit_len() as u64,
                "digit_count misses value width"
            );
        }
    }
}

/// The boundary-comb stream: a ±1 oscillation across the `2^k` carry cliff
/// stays sign-correct at every step and value-correct at the end.
///
/// This is the stream on which a normalized representation pays a full
/// k-bit carry/borrow per delta.
#[test]
fn boundary_comb_oscillation_matches_the_oracle() {
    let k = 512u32;
    let below_cliff = (UBig::from(1u8) << k as usize) - 1u8;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&below_cliff);
    oracle += IBig::from(below_cliff);
    for _ in 0..2_000 {
        acc.add_small(1);
        oracle += 1;
        assert_eq!(acc.sign(), Ordering::Greater, "above the cliff");
        acc.sub_small(1);
        oracle -= 1;
        assert_eq!(acc.sign(), Ordering::Greater, "back below the cliff");
    }
    assert_value(&acc, &oracle);
}

/// The wide-tooth stream: ±2^w teeth oscillating across a `2^k` cliff
/// (w far past any machine-word window) stay sign-correct at every step
/// and value-correct at the end.
#[test]
fn wide_teeth_across_the_cliff_match_the_oracle() {
    let (k, w) = (512u32, 192u32);
    let cliff = UBig::from(1u8) << k as usize;
    let tooth = UBig::from(1u8) << w as usize;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&cliff);
    oracle += IBig::from(cliff);
    for _ in 0..500 {
        acc.sub_wide(&tooth);
        oracle -= IBig::from(tooth.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "below the cliff");
        acc.add_wide(&tooth);
        oracle += IBig::from(tooth.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "back at the cliff");
    }
    assert_value(&acc, &oracle);
}

/// The cancelling-prefix chain: repeatedly dropping from `2^k` to 1 and
/// back stays sign-correct at every step and value-correct at snapshots.
///
/// Each drop builds a wide cancelling prefix that the next sign fold must
/// scan below the top digit and collapse.
#[test]
fn cancelling_prefix_chain_matches_the_oracle() {
    let k = 512u32;
    let peak = UBig::from(1u8) << k as usize;
    let drop = (UBig::from(1u8) << k as usize) - 1u8;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&peak);
    oracle += IBig::from(peak);
    assert_eq!(acc.sign(), Ordering::Greater);
    for cycle in 0..200 {
        acc.sub_wide(&drop);
        oracle -= IBig::from(drop.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "down at 1");
        acc.add_wide(&drop);
        oracle += IBig::from(drop.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "back at the peak");
        if cycle % 16 == 0 {
            assert_value(&acc, &oracle);
        }
    }
    assert_value(&acc, &oracle);
}

/// Exact wide cancellation lands on sign `Equal`, and unit nudges off zero
/// read `Less`/`Greater` — the near-zero discrimination the `|s| ≥ 3` fold
/// threshold must not blur.
#[test]
fn exact_cancellation_and_unit_nudges_read_correctly() {
    let wide = UBig::from(1u8) << 512usize;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&wide);
    oracle += IBig::from(wide.clone());
    acc.sub_wide(&wide);
    oracle -= IBig::from(wide);
    assert_eq!(acc.sign(), Ordering::Equal, "exact cancellation is zero");
    assert_value(&acc, &oracle);
    acc.sub_small(1);
    oracle -= 1;
    assert_eq!(acc.sign(), Ordering::Less, "one below zero");
    assert!(acc.is_negative(), "is_negative agrees with sign");
    assert_value(&acc, &oracle);
    acc.add_small(2);
    oracle += 2;
    assert_eq!(acc.sign(), Ordering::Greater, "one above zero");
    assert!(!acc.is_negative(), "is_negative agrees with sign");
    assert_value(&acc, &oracle);
}

/// Negative values convert correctly through both magnitude arms: a
/// negative with a nonzero low part (the complement path) and a negative
/// that is an exact multiple of the digit base (the untouched-zeros path).
#[test]
fn negative_magnitudes_convert_through_both_arms() {
    // Nonzero low part: −(2^192 − 5).
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    let wide = (UBig::from(1u8) << 192usize) - 5u8;
    acc.sub_wide(&wide);
    oracle -= IBig::from(wide);
    assert_eq!(acc.sign(), Ordering::Less);
    assert_value(&acc, &oracle);
    // Exact multiple of 2^32: −2^96.
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    let aligned = UBig::from(1u8) << 96usize;
    acc.sub_wide(&aligned);
    oracle -= IBig::from(aligned);
    assert_eq!(acc.sign(), Ordering::Less);
    assert_value(&acc, &oracle);
}

/// The unsigned machine-word entry points cover the full `u64` range,
/// including values past `i64::MAX`, in both directions.
#[test]
fn u64_entry_points_cover_the_full_range() {
    for engine in [false, true] {
        let mut acc = fresh(engine);
        let mut oracle = IBig::from(0);
        acc.add_u64(u64::MAX);
        oracle += u64::MAX;
        assert_eq!(acc.sign(), Ordering::Greater);
        assert_value(&acc, &oracle);
        acc.sub_u64(u64::MAX);
        oracle -= u64::MAX;
        assert_eq!(acc.sign(), Ordering::Equal);
        assert_value(&acc, &oracle);
        acc.sub_u64(u64::MAX);
        oracle -= u64::MAX;
        assert_eq!(acc.sign(), Ordering::Less);
        assert_value(&acc, &oracle);
    }
}

/// A redundantly spelled zero reads `is_literally_zero() == false` until
/// a sign read collapses it: the one-sided contract the method's name and
/// rustdoc carry, pinned so any change to it is deliberate.
#[test]
fn redundant_zero_reads_nonzero_until_collapsed() {
    let mut acc = Accumulator::new();
    acc.add_wide(&(UBig::from(1u8) << 32usize));
    acc.sub_small(1 << 32);
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!((sign, magnitude), (Ordering::Equal, UBig::ZERO));
    assert!(
        !acc.is_literally_zero(),
        "cancelling digits spell zero redundantly"
    );
    assert_eq!(acc.sign(), Ordering::Equal);
    assert!(
        acc.is_literally_zero(),
        "the sign read collapses to the canonical zero"
    );
}

/// The scaled read costs the written span, not the scale: a narrow
/// value parked at digit ~1000 reads out through `sign_magnitude_shl`
/// in O(1)-ish touches, where the plain `sign_magnitude` pays the full
/// held width.
///
/// This is the liveness pin on the write-watermark skip: without it the
/// scaled read's O(written span) claim would be decoration a full-scan
/// implementation also satisfies (values agree either way — only the
/// touch counts separate them). A reset re-arms the watermark, so the
/// cleared accumulator reads zero at scale zero.
#[cfg(feature = "touch-meter")]
#[test]
fn scaled_read_costs_the_written_span() {
    use crate::touch_meter;

    let mut acc = Accumulator::new();
    // Park a two-limb value at digit 1000 (bit shift 32_000).
    acc.add_wide_shl(&from_limbs(&[7, 9]), 32_000);
    touch_meter::reset();
    let (sign, magnitude, shift) = acc.sign_magnitude_shl();
    let scaled_read = touch_meter::touches();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(
        IBig::from(magnitude) << usize::try_from(shift).unwrap(),
        IBig::from(from_limbs(&[7, 9])) << 32_000usize,
    );
    assert!(
        scaled_read <= 16,
        "the scaled read scanned {scaled_read} digits: the write watermark \
         is not skipping the never-written prefix"
    );
    touch_meter::reset();
    let (_, full) = acc.sign_magnitude();
    assert!(
        touch_meter::touches() > 1000,
        "the full-width control read the whole span; if this dropped, the \
         separation this pin demonstrates needs re-deriving"
    );
    assert_eq!(
        IBig::from(full),
        IBig::from(from_limbs(&[7, 9])) << 32_000usize
    );
    acc.reset();
    assert!(acc.is_literally_zero());
    let (sign, magnitude, shift) = acc.sign_magnitude_shl();
    assert_eq!(
        (sign, magnitude, shift),
        (Ordering::Equal, UBig::ZERO, 0),
        "a reset accumulator reads zero at scale zero"
    );
}

/// Green pin: an
/// alternating shifted pair costs its operand, not the zero run under
/// it — exact totals, identical across a shift doubling.
///
/// A `sub_wide_shl`/`add_wide_shl` pair of a one-limb operand parked
/// at digit `shift/32` costs exactly 5 touches: the sub pays one
/// operand limb read, one deposit, and one settlement step whose
/// zero-run certificate skip crosses the whole never-written run under
/// the landing site in a single touch; the add pays one limb read and
/// one deposit (re-certifying the run it jumps is ledger bookkeeping —
/// no digit is read or written). Both shifts pin the same total, which
/// is the crate page's `*_shl` rows ("independent of the shift") made
/// exact at this schedule; the exactness doubles as the skip's
/// metering liveness floor — an uncounted skip would read 4 per pair,
/// a per-digit run walk would read shift/32 + 4.
///
/// The second scenario parks a second value on digit 0 first: the
/// schedule a single global write watermark cannot price (a watermark
/// pinned to digit 0 says nothing about the run under digit
/// `shift/32`), pinning that the ledger certifies runs individually.
/// The word-scale magnitude path pays the same shape minus the limb
/// reads: exactly 3 per pair.
#[cfg(feature = "touch-meter")]
#[test]
fn alternating_shifted_writes_cost_the_operand_not_the_gap() {
    use crate::touch_meter;

    let one = UBig::from(1u8);
    for shift in [32_000u64, 64_000] {
        let mut acc = Accumulator::new();
        acc.add_wide_shl(&one, shift);
        touch_meter::reset();
        for _ in 0..1_000 {
            acc.sub_wide_shl(&one, shift);
            acc.add_wide_shl(&one, shift);
        }
        assert_eq!(
            touch_meter::touches(),
            5_000,
            "1,000 alternating one-limb pairs at shift {shift}: 5 touches per \
             pair (2 limb reads + 2 deposits + 1 certificate skip), whatever \
             the shift"
        );
        // The oscillation is value-neutral: the held value is still 2^shift.
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            UBig::from(1u8) << usize::try_from(shift).unwrap()
        );
    }
    // The word-scale shifted entry points run the same schedule at the
    // same flat cost: one deposit per write, no limb read (the operand
    // is already a word), certificate skip included.
    for shift in [32_000u64, 64_000] {
        let mut acc = Accumulator::new();
        acc.add_u64_shl(1, shift);
        touch_meter::reset();
        for _ in 0..1_000 {
            acc.sub_u64_shl(1, shift);
            acc.add_u64_shl(1, shift);
        }
        assert_eq!(
            touch_meter::touches(),
            3_000,
            "1,000 alternating word pairs at shift {shift}: 3 touches per \
             pair (2 deposits + 1 certificate skip), whatever the shift"
        );
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            UBig::from(1u8) << usize::try_from(shift).unwrap()
        );
    }
    // A value parked on digit 0 does not re-price the oscillation above
    // it: the run's certificate, not a global watermark, funds the skip.
    for shift in [32_000u64, 64_000] {
        let mut acc = Accumulator::new();
        acc.add_small(5);
        acc.add_wide_shl(&one, shift);
        touch_meter::reset();
        for _ in 0..1_000 {
            acc.sub_wide_shl(&one, shift);
            acc.add_wide_shl(&one, shift);
        }
        assert_eq!(
            touch_meter::touches(),
            5_000,
            "the occupied digit 0 changes nothing: 5 touches per pair at \
             shift {shift}"
        );
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            (UBig::from(1u8) << usize::try_from(shift).unwrap()) + 5u8
        );
    }
    // The magnitude word path pays the same shape without the limb reads.
    let five = UBig::from(5u8);
    let mut acc = Accumulator::new();
    acc.add_magnitude_shl(&five, 32_000);
    touch_meter::reset();
    for _ in 0..1_000 {
        acc.sub_magnitude_shl(&five, 32_000);
        acc.add_magnitude_shl(&five, 32_000);
    }
    assert_eq!(
        touch_meter::touches(),
        3_000,
        "1,000 alternating word-scale magnitude pairs at shift 32,000: \
         3 touches per pair (2 deposits + 1 certificate skip)"
    );
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(magnitude, UBig::from(5u8) << 32_000usize);
}

/// Meter liveness for the top-settlement scan: the steps that lower
/// `top` past written-then-zeroed digits are counted, one touch each.
///
/// A value with digits 5..=10 nonzero is built over a never-written
/// run below, then a subtraction zeroes digits 6..=10: the sub costs
/// exactly 16 touches — 6 operand limb reads + 5 deposits + 5
/// settlement steps walking the top from digit 10 down onto digit 5.
/// If the settlement scan stopped counting, the total would read 11:
/// the exact pin is the scan's liveness floor (a ceiling over a
/// counter that can silently stop counting is decoration).
#[cfg(feature = "touch-meter")]
#[test]
fn top_settlement_steps_are_metered() {
    use crate::touch_meter;

    // Digits 5..=10 hold 1 each; digits 0..=4 are never written.
    let six_high = from_limbs(&[0, 0, 1 << 32, (1 << 32) | 1, (1 << 32) | 1, 1]);
    // Digits 6..=10 hold 1 each.
    let five_high = from_limbs(&[0, 0, 0, (1 << 32) | 1, (1 << 32) | 1, 1]);
    let mut acc = Accumulator::new();
    acc.add_wide(&six_high);
    touch_meter::reset();
    acc.sub_wide(&five_high);
    assert_eq!(
        touch_meter::touches(),
        16,
        "6 limb reads + 5 deposits + 5 settlement steps: the settlement \
         scan must count one touch per digit it steps past"
    );
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(magnitude, UBig::from(1u8) << 160usize);
}

/// The sign fold skips certified zero runs whole when its partial is
/// zero.
///
/// A cancellation spelled across the two digits above a 1,000-digit
/// never-written run reads `Equal` in exactly 6 touches (3 fold
/// reads plus 3 collapse zeroes) instead of walking the run, and the
/// collapse still canonicalizes the spelling to the literal zero.
#[cfg(feature = "touch-meter")]
#[test]
fn sign_fold_skips_certified_runs() {
    use crate::touch_meter;

    let mut acc = Accumulator::new();
    acc.add_wide_shl(&UBig::from(1u8), 32 * 1_001);
    // Deposit −2^32 in digit 1,000: the value is now zero, spelled
    // across digits 1,000 and 1,001 above the never-written run.
    acc.sub_magnitude_shl(&UBig::from(1u64 << 32), 32 * 1_000);
    touch_meter::reset();
    assert_eq!(acc.sign(), Ordering::Equal);
    assert_eq!(
        touch_meter::touches(),
        6,
        "3 fold reads + 3 collapse zeroes: a zero partial crosses the \
         certified run in one skip, touching none of its digits"
    );
    assert!(acc.is_literally_zero(), "the collapse canonicalized zero");
}

/// The accumulator-operand rows cost the operand's held digits, not
/// the receiver's width or the shift.
///
/// Exact totals for `add_accum`, `sub_accum`, `add_accum_shl`,
/// `sub_accum_shl`, and `merge_into_wider` on a two-digit operand —
/// 4 touches each (one read plus one deposit per operand digit) —
/// unchanged when the receiver's held width doubles and when the merge
/// shift doubles.
///
/// These are the cost table's "amortized O(operand's held digits),
/// whatever the held width / independent of the shift" rows, pinned
/// exact at their canonical shape.
#[cfg(feature = "touch-meter")]
#[test]
fn accumulator_operand_rows_cost_the_operand() {
    use crate::touch_meter;

    // A two-digit operand: digits 0 and 1 hold 1 each.
    let narrow = || {
        let mut acc = Accumulator::new();
        acc.add_wide(&from_limbs(&[(1 << 32) | 1]));
        acc
    };
    // Receivers of 64 and 128 held digits: 2^k − 1 fills every digit.
    for k in [2_048u32, 4_096] {
        let wide_value = (UBig::from(1u8) << k as usize) - 1u8;
        let op = narrow();

        let mut receiver = Accumulator::new();
        receiver.add_wide(&wide_value);
        touch_meter::reset();
        receiver.add_accum(&op);
        assert_eq!(
            touch_meter::touches(),
            4,
            "add_accum of 2 digits into {} held digits: 2 reads + 2 deposits",
            k / 32,
        );

        let mut receiver = Accumulator::new();
        receiver.add_wide(&wide_value);
        touch_meter::reset();
        receiver.sub_accum(&op);
        assert_eq!(touch_meter::touches(), 4, "sub_accum twin at {k} bits");

        let mut receiver = Accumulator::new();
        receiver.add_wide(&wide_value);
        let mut spare = op;
        touch_meter::reset();
        spare = receiver.merge_into_wider(spare);
        assert_eq!(
            touch_meter::touches(),
            4,
            "merge_into_wider reads the narrower operand only"
        );
        spare.reset();
        let (sign, magnitude) = receiver.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            (UBig::from(1u8) << k as usize) - 1u8 + UBig::from((1u64 << 32) | 1)
        );
    }
    // The scaled merges are shift-independent at the same exact total,
    // in both signs.
    for shift in [32_000u64, 64_000] {
        let op = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_wide(&((UBig::from(1u8) << 2_048usize) - 1u8));
        touch_meter::reset();
        receiver.add_accum_shl(&op, shift);
        assert_eq!(
            touch_meter::touches(),
            4,
            "add_accum_shl of 2 digits at shift {shift}: 2 reads + 2 deposits"
        );
        let (sign, magnitude) = receiver.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            (UBig::from(1u8) << 2_048usize) - 1u8
                + (UBig::from((1u64 << 32) | 1) << usize::try_from(shift).unwrap())
        );

        let op = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_wide(&((UBig::from(1u8) << 2_048usize) - 1u8));
        touch_meter::reset();
        receiver.sub_accum_shl(&op, shift);
        assert_eq!(
            touch_meter::touches(),
            4,
            "sub_accum_shl of 2 digits at shift {shift}: 2 reads + 2 deposits"
        );
        // The shifted operand towers over the receiver, so the
        // difference is negative: the honest value is operand-first.
        let (sign, magnitude) = receiver.sign_magnitude();
        assert_eq!(sign, Ordering::Less);
        assert_eq!(
            magnitude,
            (UBig::from((1u64 << 32) | 1) << usize::try_from(shift).unwrap())
                - ((UBig::from(1u8) << 2_048usize) - 1u8)
        );
    }
}

/// The per-call O(held digits) rows read exact totals at 64 and 128
/// held digits, and `shl`'s total is independent of the shift.
///
/// `negate` and `reset` touch each held digit once (d, and d + 1 after
/// a shift grew the span by one), `shl` reads each digit and
/// re-deposits it (2d) — the same 2d at a thousandfold larger shift,
/// the digit-touch shift-independence the crate page's footnote
/// claims — and `sign_magnitude` carries once through the span (d).
/// Exact equality across the width doubling is the linearity claim
/// with no slack for a hidden second pass.
#[cfg(feature = "touch-meter")]
#[test]
fn held_width_rows_cost_the_held_digits() {
    use crate::touch_meter;

    for bits in [2_048u32, 4_096] {
        let d = u64::from(bits / 32);
        let wide_value = (UBig::from(1u8) << bits as usize) - 1u8;
        let mut acc = Accumulator::new();
        acc.add_wide(&wide_value);

        touch_meter::reset();
        acc.negate();
        assert_eq!(
            touch_meter::touches(),
            d,
            "negate at {d} held digits: one touch per digit"
        );
        acc.negate();

        touch_meter::reset();
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(
            touch_meter::touches(),
            d,
            "sign_magnitude at {d} held digits: one carry pass over the span"
        );
        assert_eq!((sign, magnitude), (Ordering::Greater, wide_value.clone()));

        touch_meter::reset();
        acc.shl(32);
        assert_eq!(
            touch_meter::touches(),
            2 * d,
            "shl at {d} held digits: one read and one re-deposit per digit"
        );
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(magnitude, wide_value.clone() << 32usize);

        touch_meter::reset();
        acc.reset();
        assert_eq!(
            touch_meter::touches(),
            d + 1,
            "reset at {d} held digits: one touch per digit (the shift grew \
             the span by one)"
        );
        assert!(acc.is_literally_zero());
    }
    // The same in-place scale at a thousandfold larger shift costs the
    // same touches: shl is priced by the held digits alone (memory, not
    // digit work, covers the shifted positions).
    let wide_value = (UBig::from(1u8) << 2_048usize) - 1u8;
    let mut acc = Accumulator::new();
    acc.add_wide(&wide_value);
    touch_meter::reset();
    acc.shl(32_000);
    assert_eq!(
        touch_meter::touches(),
        128,
        "shl(32_000) at 64 held digits: the same 2 touches per held digit \
         as shl(32)"
    );
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(magnitude, wide_value << 32_000usize);
}

/// The unsigned machine-word rows are amortized O(1) at the top of the
/// `u64` range: exact totals, flat across a doubling of both the cliff
/// height and the stream length.
///
/// A `u64::MAX` delta is the widest input the word rows accept and the
/// deepest per-call carry they can start (its deposit recenters digit 0
/// and carries into digit 1 every time). Parked on the `2^k − 1` cliff
/// with a sign read after every delta, the stream costs exactly
/// `6n + 1` touches for `n` add/sub pairs — 2 per delta, 1 per sign
/// read, plus one extra on the first crossing (its carry run reaches
/// digit 2 once; every later pair repays exactly what it disturbs) —
/// at `k = 4096` and `k = 8192` alike. Measured
/// (deterministic counter; word-pairing keeps counts
/// target-independent).
#[cfg(feature = "touch-meter")]
#[test]
fn u64_comb_touches_are_flat_and_exact() {
    use crate::touch_meter;

    for (k, n) in [(4_096u32, 50_000u64), (8_192, 100_000)] {
        let mut acc = Accumulator::new();
        acc.add_wide(&((UBig::from(1u8) << k as usize) - 1u8));
        touch_meter::reset();
        for _ in 0..n {
            acc.add_u64(u64::MAX);
            assert_eq!(acc.sign(), Ordering::Greater, "above the cliff");
            acc.sub_u64(u64::MAX);
            assert_eq!(acc.sign(), Ordering::Greater, "back below the cliff");
        }
        assert_eq!(
            touch_meter::touches(),
            6 * n + 1,
            "u64::MAX comb at k = {k}: 2 touches per delta and 1 per sign \
             read, whatever the cliff height and stream length"
        );
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(magnitude, (UBig::from(1u8) << k as usize) - 1u8);
    }
}

/// The wide rows cost the operand's limbs, not the held width: exact
/// equal totals into receivers of 64 and 128 held digits, both signs.
///
/// A two-limb operand costs exactly 4 touches (one read plus one
/// deposit per limb — the high halves here are zero and deposit
/// nothing) whether the receiver holds 64 or 128 digits. This is the
/// "whatever the held width" half of the wide rows; the
/// shift-independence half is
/// `alternating_shifted_writes_cost_the_operand_not_the_gap`.
#[cfg(feature = "touch-meter")]
#[test]
fn wide_writes_cost_the_operand_at_any_held_width() {
    use crate::touch_meter;

    for bits in [2_048u32, 4_096] {
        let held = (UBig::from(1u8) << bits as usize) - 1u8;
        let mut acc = Accumulator::new();
        acc.add_wide(&held);
        touch_meter::reset();
        acc.add_wide(&from_limbs(&[3, 5]));
        assert_eq!(
            touch_meter::touches(),
            4,
            "add_wide of 2 limbs into {} held digits: 2 limb reads + 2 deposits",
            bits / 32
        );
        touch_meter::reset();
        acc.sub_wide(&from_limbs(&[3, 5]));
        assert_eq!(
            touch_meter::touches(),
            4,
            "sub_wide twin at {} held digits",
            bits / 32
        );
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!((sign, magnitude), (Ordering::Greater, held));
    }
}

/// The magnitude entry points cost exactly their dispatched path: a
/// word-scale operand the small path's touches, a wide operand the
/// wide path's, in both signs.
///
/// This pins the dispatch itself as free in the digit denomination
/// (`to_word` is the trait's O(1) obligation), so the table's
/// "word-scale: amortized O(1); wide: amortized O(operand limbs)" row
/// inherits the raw rows' evidence verbatim.
#[cfg(feature = "touch-meter")]
#[test]
fn magnitude_dispatch_costs_its_width_path() {
    use crate::touch_meter;

    let word = UBig::from(5u8);
    let wide = from_limbs(&[3, 5, 7]);
    // The inline tuple type is the documentation; a minted alias would
    // only add a name to track.
    #[allow(clippy::type_complexity)]
    let cases: [(&dyn Fn(&mut Accumulator), &dyn Fn(&mut Accumulator), &str); 4] = [
        (
            &|acc| acc.add_u64(5),
            &|acc| acc.add_magnitude(&word),
            "add word",
        ),
        (
            &|acc| acc.sub_u64(5),
            &|acc| acc.sub_magnitude(&word),
            "sub word",
        ),
        (
            &|acc| acc.add_wide(&wide),
            &|acc| acc.add_magnitude(&wide),
            "add wide",
        ),
        (
            &|acc| acc.sub_wide(&wide),
            &|acc| acc.sub_magnitude(&wide),
            "sub wide",
        ),
    ];
    for (raw, dispatched, what) in cases {
        let mut acc = Accumulator::new();
        touch_meter::reset();
        raw(&mut acc);
        let raw_touches = touch_meter::touches();
        let mut acc = Accumulator::new();
        touch_meter::reset();
        dispatched(&mut acc);
        assert_eq!(
            touch_meter::touches(),
            raw_touches,
            "{what}: the magnitude dispatch must cost exactly its width path"
        );
    }
}

/// A floor within 2 of `usize::MAX` never certifies domination: the
/// decision index saturates instead of wrapping.
///
/// The witness input: a value whose sign fold decides at digit index 63
/// (`2^2048`), probed at `floor = usize::MAX - 1`. The contract requires
/// `decided = false` — no held value dominates every adjustment fitting
/// in digits `0..=usize::MAX - 1` — and a wrapping `floor + 2` computes
/// 0, so an unsaturated decision index either panics the debug build
/// (the add overflows) or certifies domination from 64 digits in
/// release: this test is the committed witness that the index
/// saturates.
#[test]
fn domination_floor_near_usize_max_never_decides() {
    let mut acc = Accumulator::new();
    acc.add_wide(&(UBig::from(1u8) << 2_048usize));
    let (sign, decided) = acc.sign_dominates_at(usize::MAX - 1);
    assert_eq!(sign, Ordering::Greater, "the sign is exact at any floor");
    assert!(
        !decided,
        "no value dominates an adjustment bound wider than the address space"
    );
    // The same value still certifies every in-range floor its decision
    // index covers: the saturation changes nothing below the overflow
    // boundary.
    assert_eq!(acc.sign_dominates_at(61), (Ordering::Greater, true));
}

/// Domination certificates are amortized O(1): the first read may
/// collapse, every later read is a single touch, and the certificate
/// stays decided.
///
/// A value parked at digit 64 answers `sign_dominates_at(3)` in 5
/// touches (a two-step fold, its collapse, and the re-deposit), then
/// exactly one touch per read for a thousand reads — a wide running
/// total is never re-folded across its width by cheap comparisons.
#[cfg(feature = "touch-meter")]
#[test]
fn domination_reads_cost_one_touch_after_the_first() {
    use crate::touch_meter;

    let mut acc = Accumulator::new();
    acc.add_wide(&(UBig::from(1u8) << 2_048usize));
    touch_meter::reset();
    assert_eq!(
        acc.sign_dominates_at(3),
        (Ordering::Greater, true),
        "2^2048 dominates any value under 2^128"
    );
    assert_eq!(
        touch_meter::touches(),
        5,
        "the first read folds two digits, collapses, and re-deposits"
    );
    touch_meter::reset();
    for _ in 0..1_000 {
        assert_eq!(acc.sign_dominates_word(), (Ordering::Greater, true));
    }
    assert_eq!(
        touch_meter::touches(),
        1_000,
        "every read after the collapse is one top-digit touch"
    );
}

/// The scaled read costs the *span* of written digits — watermark to
/// top, never-written interior gaps included — not their count.
///
/// Two writes park digit touches at positions 0 and 1000/1002 (three
/// written digits in all); `sign_magnitude_shl` then costs exactly
/// 1003 touches, one per digit of the span, at scale zero. This is
/// the honest denominator of the crate table's scaled-read row: a
/// reading of that row as "O(number of digits written)" predicts ~3
/// touches here and is refuted by this pin. The skip is exact only
/// over the never-written prefix *below* the watermark
/// (`scaled_read_costs_the_written_span` pins that side).
#[cfg(feature = "touch-meter")]
#[test]
fn scaled_read_costs_the_span_not_the_write_count() {
    use crate::touch_meter;

    let mut acc = Accumulator::new();
    // Limbs [7, 9] at bit 32_000: digit 1000 holds 7, digit 1002 holds 9
    // (the odd-limb high halves are zero and deposit nothing).
    acc.add_wide_shl(&from_limbs(&[7, 9]), 32_000);
    acc.add_small(5);
    touch_meter::reset();
    let (sign, magnitude, shift) = acc.sign_magnitude_shl();
    assert_eq!(
        touch_meter::touches(),
        1_003,
        "the scaled read walks the whole span between the two writes: \
         its cost is the written span, not the written-digit count"
    );
    assert_eq!((sign, shift), (Ordering::Greater, 0));
    assert_eq!(
        IBig::from(magnitude),
        (IBig::from(from_limbs(&[7, 9])) << 32_000usize) + 5
    );
}

/// Adequacy tripwire for the sign rows: a fold that does not collapse
/// re-scans the cancelling prefix on every read, at a cost that grows
/// with the prefix — the flat-per-read criterion reads red on it.
///
/// The known-bad mechanism is committed here as a real fold over the
/// digits (the production decision rule, minus the collapse): on the
/// static prefix it pays `k/32 + 1` touches per read — 65 at
/// `k = 2048`, 129 at `k = 4096`, doubling with the prefix — where
/// the production fold pays `2·(k/32) + 3` once (scan, zero, and
/// re-deposit) and then exactly 1 per read for a thousand reads at
/// both widths. Deleting the collapse cannot keep the amortized-O(1)
/// sign rows green: this pin is the separation.
#[cfg(feature = "touch-meter")]
#[test]
fn no_collapse_fold_re_scans_the_prefix() {
    use crate::touch_meter;

    /// The fold's decision rule without its collapse: the digit
    /// touches a non-collapsing sign read would pay on `acc` as it
    /// stands, every time.
    fn no_collapse_read_touches(acc: &Accumulator) -> u64 {
        let mut touches = 0u64;
        let mut index = acc.top;
        let mut partial: i128 = 0;
        loop {
            touches += 1;
            partial = (partial << 32) + i128::from(acc.digits[index]);
            if partial.abs() >= 3 || index == 0 {
                return touches;
            }
            index -= 1;
        }
    }

    for k in [2_048u32, 4_096] {
        let d = u64::from(k / 32);
        let mut acc = Accumulator::new();
        acc.add_wide(&(UBig::from(1u8) << k as usize));
        acc.sub_wide(&((UBig::from(1u8) << k as usize) - 1u8));
        assert_eq!(
            no_collapse_read_touches(&acc),
            d + 1,
            "the no-collapse fold walks the whole prefix, and would again \
             on every read"
        );
        touch_meter::reset();
        assert_eq!(acc.sign(), Ordering::Greater);
        assert_eq!(
            touch_meter::touches(),
            2 * d + 3,
            "the production fold pays the scan once, with its collapse"
        );
        touch_meter::reset();
        for _ in 0..1_000 {
            assert_eq!(acc.sign(), Ordering::Greater);
        }
        assert_eq!(
            touch_meter::touches(),
            1_000,
            "after the collapse every sign read is one touch, at any k"
        );
    }
}

/// Deposit `−(2^33 − 1)` — the lazy zone's most negative digit — at
/// digit `index` through the public word-scale entry points, without
/// triggering a recenter.
///
/// Two deposits of `−2^32` and `−(2^32 − 1)` land in one digit because
/// each intermediate total stays inside the zone; a single deposit of
/// the full value would recenter. This is the construction behind the
/// extreme-cancellation witnesses below: an adversary (or an unlucky
/// workload) can park any digit one unit inside the zone boundary.
fn park_extreme_negative_digit(acc: &mut Accumulator, index: u64) {
    // The construction is a digit-engine spelling: arm the engine so
    // the register cannot fuse the two deposits into one exact value.
    acc.spill();
    acc.sub_magnitude_shl(&UBig::from(1u64 << 32), 32 * index);
    acc.sub_magnitude_shl(&UBig::from((1u64 << 32) - 1), 32 * index);
}

/// The sign fold's decision threshold is tight: a running partial of
/// exactly 2 can still be overturned by the digits below, so the fold
/// must keep descending — a threshold of 2 would report the wrong sign
/// on this input.
///
/// The witness parks digits `[−(2^33 − 1), −(2^33 − 1), 2]` (built
/// through public entry points; every digit is one unit inside the lazy
/// zone). The suffix partial at the top digit is exactly 2, but the two
/// digits below sum to `−(2^33 − 1)·(2^32 + 1)`, which exceeds
/// `2 · 2^64` by `2^32 − 1`: the true value is `−(2^32 − 1)`, negative.
/// Committed because the mutation `SIGN_DECIDED: 3 → 2` — which reads
/// this value as positive — passed the whole differential suite:
/// random streams essentially never stack two adjacent
/// extreme digits under a partial of exactly 2, so the tight corner
/// needs this constructed pin. The mirrored spelling checks the
/// negative-partial side of the same boundary.
#[test]
fn sign_threshold_survives_extreme_cancellation() {
    let mut acc = Accumulator::new();
    park_extreme_negative_digit(&mut acc, 0);
    park_extreme_negative_digit(&mut acc, 1);
    acc.add_magnitude_shl(&UBig::from(2u8), 64);
    // Independent read-out first: the low-to-high carry pass does not
    // share the fold's threshold, so the two paths cross-check.
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(
        (sign, magnitude),
        (Ordering::Less, UBig::from((1u64 << 32) - 1)),
        "the exact value is −(2^32 − 1)"
    );
    assert_eq!(
        acc.sign(),
        Ordering::Less,
        "a top partial of 2 must not decide"
    );
    // The mirrored spelling: digits [2^33 − 1, 2^33 − 1, −2] denote
    // +(2^32 − 1), and a partial of −2 at the top must not decide either.
    acc.negate();
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(
        (sign, magnitude),
        (Ordering::Greater, UBig::from((1u64 << 32) - 1)),
        "the mirrored value is +(2^32 − 1)"
    );
    assert_eq!(acc.sign(), Ordering::Greater);
}

/// The domination certificate's decision index is tight: a fold that
/// decides at digit index `floor + 1` must answer `decided = false`,
/// because an adjustment under `2^(32·(floor + 1))` can still flip the
/// sign there.
///
/// The witness holds digits `[−(2^33 − 1), −(2^33 − 1), 3]`: the fold
/// decides the (exact, positive) sign at index 2 with partial 3, and
/// the value is `2^64 − 2^32 + 1` — strictly less than `u64::MAX`. At
/// `floor = 1` the contract covers every `u64` adjustment, so `decided`
/// must be false: subtracting `u64::MAX` flips the sign, as the tail of
/// the test demonstrates. Committed because the mutation
/// `floor.saturating_add(2) → floor.saturating_add(1)` — which
/// certifies domination here — passed the entire committed workspace
/// suite (suanpan and before both); the certificate is
/// consumed by before's rank/query sweeps to skip folds entirely, so a
/// wrongly-decided verdict would corrupt values silently.
#[test]
fn domination_decision_index_is_tight_at_floor_plus_two() {
    let mut acc = Accumulator::new();
    park_extreme_negative_digit(&mut acc, 0);
    park_extreme_negative_digit(&mut acc, 1);
    acc.add_magnitude_shl(&UBig::from(3u8), 64);
    let (sign, decided) = acc.sign_dominates_at(1);
    assert_eq!(sign, Ordering::Greater, "the sign itself is exact");
    assert!(
        !decided,
        "deciding at index floor + 1 would certify domination over an \
         adjustment larger than the held value"
    );
    // Why it must be false: a u64 adjustment (covered by floor = 1)
    // flips the sign.
    acc.sub_u64(u64::MAX);
    assert_eq!(
        acc.sign(),
        Ordering::Less,
        "u64::MAX exceeds the held 2^64 − 2^32 + 1"
    );
}

/// A decided domination verdict covers a maximally redundant
/// *accumulator* operand held in digits `0..=floor` — the contract
/// clause consumers lean on, and the one leg the differential proptests
/// cannot reach.
///
/// Consumers read `sign_dominates_at(other.digit_count() - 1)` before
/// `sub_accum(&other)`. The differential proptests' probes are plain
/// magnitudes under `2^(32·(floor + 1))`, while a lazy-zone spelling
/// reaches almost `2.01 · 2^(32·(floor + 1))`.
///
/// The held value decides at exactly index `floor + 2` with the minimum
/// partial 3 over maximally cancelling lower digits — the tightest
/// decided value — and the operand spells every digit `0..=floor` at
/// `+(2^33 − 1)`, about twice the largest plain magnitude the proptest
/// can draw. Folding the operand in (both signs) must preserve the
/// sign, and the operand's magnitude must be strictly smaller.
#[test]
fn decided_domination_covers_extreme_accumulator_operands() {
    let floor = 1usize;
    // v: digits [−(2^33 − 1), −(2^33 − 1), −(2^33 − 1), 3], deciding at
    // index 3 = floor + 2 with partial exactly 3.
    let mut v = Accumulator::new();
    park_extreme_negative_digit(&mut v, 0);
    park_extreme_negative_digit(&mut v, 1);
    park_extreme_negative_digit(&mut v, 2);
    v.add_magnitude_shl(&UBig::from(3u8), 96);
    let (sign, decided) = v.sign_dominates_at(floor);
    assert_eq!(sign, Ordering::Greater);
    assert!(decided, "partial 3 at index floor + 2 is the decision edge");
    // a: an accumulator held in digits 0..=floor at the zone's edge —
    // magnitude (2^33 − 1)(2^32 + 1), far beyond any u64.
    let mut a = Accumulator::new();
    park_extreme_negative_digit(&mut a, 0);
    park_extreme_negative_digit(&mut a, 1);
    a.negate();
    assert_eq!(a.digit_count() - 1, floor, "the operand sits at the floor");
    // |v| > |a| and folding ±a cannot flip the sign.
    let mut probe = v.clone();
    probe.sub_accum(&a);
    assert_eq!(probe.sign(), Ordering::Greater, "v − a keeps v's sign");
    let mut probe = v.clone();
    probe.add_accum(&a);
    assert_eq!(probe.sign(), Ordering::Greater, "v + a keeps v's sign");
}

/// A carry tie that recenters to a zero remainder converts exactly:
/// the read-out's complement pass must ripple through the zero low
/// digit.
///
/// The flush-right deposit `−2^32 − 2^32` lands digit 0 on the recenter
/// boundary (remainder 0, carry −2). Pins the `rem_euclid`/complement
/// seam of `sign_magnitude` on the one shape where the complement's
/// carry crosses a zero digit — the arm the negative-conversion test's
/// operands never exercise.
#[test]
fn flush_right_carry_tie_converts_exactly() {
    let mut acc = Accumulator::new();
    acc.sub_u64(1 << 32);
    acc.sub_u64(1 << 32);
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!((sign, magnitude), (Ordering::Less, UBig::from(1u64 << 33)));
    assert_eq!(acc.sign(), Ordering::Less);
}

/// A fresh accumulator (and its `Default`) holds exactly zero.
#[test]
fn new_and_default_hold_zero() {
    for mut acc in [Accumulator::new(), Accumulator::default()] {
        assert_eq!(acc.sign(), Ordering::Equal);
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Equal);
        assert_eq!(magnitude, UBig::from(0u8));
    }
}

proptest! {
    /// The fold primitives match the oracle.
    ///
    /// `add_accum` and `sub_accum` fold one accumulator's held value
    /// into another at any interleaving, `negate` flips the value
    /// exactly, and `reset` returns to exact zero — all with the sign
    /// readable afterward.
    #[test]
    fn fold_primitives_match_the_oracle(
        x_ops in proptest::collection::vec(arb_op(), 1..60),
        y_ops in proptest::collection::vec(arb_op(), 1..60),
        subtract: bool,
        flip: bool,
        x_engine: bool,
        y_engine: bool,
    ) {
        let mut x = fresh(x_engine);
        let mut x_oracle = IBig::from(0);
        for op in &x_ops {
            apply(&mut x, &mut x_oracle, op);
        }
        let mut y = fresh(y_engine);
        let mut y_oracle = IBig::from(0);
        for op in &y_ops {
            apply(&mut y, &mut y_oracle, op);
        }
        if flip {
            y.negate();
            y_oracle = -y_oracle;
            assert_value(&y, &y_oracle);
        }
        if subtract {
            x.sub_accum(&y);
            x_oracle -= &y_oracle;
        } else {
            x.add_accum(&y);
            x_oracle += &y_oracle;
        }
        assert_value(&x, &x_oracle);
        assert_value(&y, &y_oracle);
        x.reset();
        assert_eq!(x.sign(), Ordering::Equal);
        assert_value(&x, &IBig::from(0));
    }

    /// `is_literally_zero` is one-sided and a sign read canonicalizes.
    ///
    /// After any stream, `is_literally_zero() == true` implies the value
    /// is zero, and whenever the value is zero a `sign` read collapses
    /// the spelling so `is_literally_zero` reads true afterward.
    #[test]
    fn is_literally_zero_is_sound_and_sign_canonicalizes(
        ops in proptest::collection::vec(arb_op(), 1..120),
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
            if acc.is_literally_zero() {
                prop_assert_eq!(&oracle, &IBig::ZERO);
            }
            if acc.sign() == Ordering::Equal {
                prop_assert_eq!(&oracle, &IBig::ZERO);
                prop_assert!(
                    acc.is_literally_zero(),
                    "a sign read canonicalizes zero"
                );
            }
        }
    }

    /// Run-forming shift streams match the exact oracle at every step.
    ///
    /// Wide-shifted writes at scales that jump far above the held top —
    /// creating, splitting, and consuming zero-run certificates on
    /// every schedule the strategy can draw — mixed with word-scale
    /// deltas and a sign read per step: the ledger never changes the
    /// value the digits denote, and the full value matches at periodic
    /// snapshots and the end.
    #[test]
    fn run_forming_shift_streams_match_the_bigint_oracle(
        ops in proptest::collection::vec(
            (0u8..4, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),
            1..150,
        ),
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        for (step, (arm, limbs, shift)) in ops.iter().enumerate() {
            let value = from_limbs(limbs);
            let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();
            match arm {
                0 => {
                    acc.add_wide_shl(&value, *shift);
                    oracle += scaled;
                }
                1 => {
                    acc.sub_wide_shl(&value, *shift);
                    oracle -= scaled;
                }
                2 => {
                    acc.sub_magnitude_shl(&value, *shift);
                    oracle -= scaled;
                }
                _ => {
                    let delta = limbs[0] as i64;
                    acc.add_small(delta);
                    oracle += delta;
                }
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle), "sign at step {}", step);
            if step % 32 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }

    /// `shl` scales in place exactly: after any stream, shifting the held
    /// value by an arbitrary amount matches the oracle's `x · 2^s`.
    #[test]
    fn in_place_shift_matches_the_oracle(
        ops in proptest::collection::vec(arb_op(), 1..60),
        shift in 0u64..200,
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
        }
        acc.shl(shift);
        oracle <<= shift as usize;
        assert_value(&acc, &oracle);
    }

    /// `merge_into_wider` preserves the sum regardless of which operand is
    /// wider, and hands back a drained buffer whose stale digits never
    /// leak into the result.
    #[test]
    fn width_ordered_merges_match_the_oracle(
        x_ops in proptest::collection::vec(arb_op(), 1..60),
        y_ops in proptest::collection::vec(arb_op(), 1..60),
        x_engine: bool,
        y_engine: bool,
    ) {
        let mut x = fresh(x_engine);
        let mut x_oracle = IBig::from(0);
        for op in &x_ops {
            apply(&mut x, &mut x_oracle, op);
        }
        let mut y = fresh(y_engine);
        let mut y_oracle = IBig::from(0);
        for op in &y_ops {
            apply(&mut y, &mut y_oracle, op);
        }
        let drained = x.merge_into_wider(y);
        x_oracle += &y_oracle;
        assert_value(&x, &x_oracle);
        // The pool contract: a drained buffer re-arms to a clean zero.
        let mut reused = drained;
        reused.reset();
        assert_value(&reused, &IBig::from(0));
    }

    /// `sign_dominates_at` never lies at any floor: the sign matches the
    /// oracle's, and a `decided` verdict implies no operand fitting in
    /// digits `0..=floor` could flip the sign when folded in.
    #[test]
    fn floor_domination_is_sound(
        ops in proptest::collection::vec(arb_op(), 1..60),
        floor in 0usize..8,
        probe_limbs in proptest::collection::vec(any::<u64>(), 1..=4),
        probe_negative: bool,
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
        }
        let (sign, decided) = acc.sign_dominates_at(floor);
        prop_assert_eq!(sign, oracle_sign(&oracle));
        assert_value(&acc, &oracle);
        if decided {
            // The largest operand the verdict covers: top digit index at
            // most `floor`, so at most 32·(floor + 1) bits.
            let mut probe = from_limbs(&probe_limbs);
            let cap = 32 * (floor + 1);
            probe &= (UBig::from(1u8) << cap) - 1u8;
            let mut folded = oracle.clone();
            if probe_negative {
                folded -= IBig::from(probe);
            } else {
                folded += IBig::from(probe);
            }
            prop_assert_eq!(
                oracle_sign(&folded), sign,
                "a decided verdict survives any fold under its floor"
            );
        }
    }

    /// `sign_dominates_word` never lies: the sign always matches the
    /// oracle's, and a `decided` verdict implies the held magnitude
    /// exceeds any machine word — folding any `u64` afterward cannot
    /// flip the sign.
    #[test]
    fn word_domination_is_sound(
        ops in proptest::collection::vec(arb_op(), 1..60),
        probe: u64,
        probe_negative: bool,
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
        }
        let (sign, decided) = acc.sign_dominates_word();
        prop_assert_eq!(sign, oracle_sign(&oracle));
        assert_value(&acc, &oracle);
        if decided {
            let mut folded = oracle.clone();
            if probe_negative {
                folded -= probe;
            } else {
                folded += probe;
            }
            prop_assert_eq!(
                oracle_sign(&folded), sign,
                "a decided verdict survives any word-scale fold"
            );
        }
    }
}

/// The quick register engages on machine-word streams, retires at the
/// first wide operand, and re-arms on reset.
///
/// The liveness pin that the fast path is actually taken and the
/// spill actually spills, so neither mode's coverage is vacuous.
#[test]
fn quick_register_engages_and_retires() {
    let mut acc = Accumulator::new();
    acc.add_u64(u64::MAX);
    acc.sub_small(i64::MIN);
    assert!(
        acc.quick.is_some(),
        "word-scale streams stay in the register"
    );
    acc.add_wide(&(UBig::from(1u8) << 200usize));
    assert!(acc.quick.is_none(), "a wide operand arms the digit engine");
    acc.add_u64(1);
    assert!(
        acc.quick.is_none(),
        "the exit is one-way within an epoch: word traffic stays in the engine"
    );
    acc.reset();
    assert!(acc.quick.is_some(), "a reset re-arms the register");
}

/// A register parked at its ceiling: `±2^96`, built through register
/// entry points alone (the shifts stay at the ceiling, not past it).
fn full_register(negative: bool) -> (Accumulator, IBig) {
    let mut acc = Accumulator::new();
    acc.add_u64(1 << 36);
    acc.shl(30);
    acc.shl(30);
    if negative {
        acc.negate();
    }
    assert!(
        acc.quick.is_some(),
        "the ceiling itself is a register value"
    );
    let mut oracle = IBig::from(UBig::ONE << 96usize);
    if negative {
        oracle = -oracle;
    }
    (acc, oracle)
}

/// The register's headroom is exact at every extreme: no input can
/// drive the fast path into `i128` overflow.
///
/// The widest operand of every register entry point, folded into a
/// register parked at its ceiling, spills and matches the oracle
/// (debug builds would panic on the arithmetic; release builds would
/// silently wrap into a wrong value the oracle comparison catches).
#[test]
fn quick_register_extremes_spill_exactly() {
    // The headroom derivation itself, pinned executable: a full
    // register plus the widest shifted fold the register accepts stays
    // strictly inside i128.
    let ceiling = i128::try_from(QUICK_MAX).expect("the ceiling fits i128");
    let widest_fold = ceiling
        .checked_shl(QUICK_SHIFT_MAX as u32)
        .expect("the widest shifted fold fits i128");
    ceiling
        .checked_add(widest_fold)
        .expect("the worst register sum fits i128");

    // Word-scale extremes against the parked ceiling, both signs.
    for negative in [false, true] {
        let (mut acc, mut oracle) = full_register(negative);
        acc.add_u64(u64::MAX);
        oracle += u64::MAX;
        assert_value(&acc, &oracle);

        let (mut acc, mut oracle) = full_register(negative);
        acc.sub_u64(u64::MAX);
        oracle -= u64::MAX;
        assert_value(&acc, &oracle);

        let (mut acc, mut oracle) = full_register(negative);
        acc.add_small(i64::MIN);
        oracle += i64::MIN;
        assert_value(&acc, &oracle);

        let (mut acc, mut oracle) = full_register(negative);
        acc.sub_small(i64::MIN);
        oracle -= i64::MIN;
        assert_value(&acc, &oracle);

        // The widest shifted word the register path accepts.
        let (mut acc, mut oracle) = full_register(negative);
        acc.add_magnitude_shl(&UBig::from(u64::MAX), QUICK_SHIFT_MAX);
        oracle += IBig::from(u64::MAX) << QUICK_SHIFT_MAX as usize;
        assert_value(&acc, &oracle);

        // The widest register-to-register folds: a full register into a
        // full register at the widest register shift, both directions.
        for (fold_negative, subtract) in
            [(false, false), (false, true), (true, false), (true, true)]
        {
            let (mut acc, mut oracle) = full_register(negative);
            let (operand, operand_oracle) = full_register(fold_negative);
            let scaled = operand_oracle << QUICK_SHIFT_MAX as usize;
            if subtract {
                acc.sub_accum_shl(&operand, QUICK_SHIFT_MAX);
                oracle -= scaled;
            } else {
                acc.add_accum_shl(&operand, QUICK_SHIFT_MAX);
                oracle += scaled;
            }
            assert_value(&acc, &oracle);
        }

        // The widest in-place shift of a full register.
        let (mut acc, mut oracle) = full_register(negative);
        acc.shl(QUICK_SHIFT_MAX);
        oracle <<= QUICK_SHIFT_MAX as usize;
        assert!(acc.quick.is_none(), "the shifted ceiling spills");
        assert_value(&acc, &oracle);

        // Negation at the ceiling round-trips inside the register.
        let (mut acc, mut oracle) = full_register(negative);
        acc.negate();
        oracle = -oracle;
        assert!(acc.quick.is_some(), "the zone is symmetric about zero");
        assert_value(&acc, &oracle);
    }
}

// ─── the zero-run ledger's structural invariants, exhaustively ──────────────
//
// The ledger's certificates are consumed by scans and trusted without
// re-reading the digits they certify, so their soundness (all-zero
// interiors) and structure (disjointness, containment under the
// settled top) are load-bearing for both cost and correctness:
// `crop_runs`' descending early stop is derived from disjointness
// plus containment, and a stale certificate over a written digit
// would corrupt values, not just costs. The checker below reads the
// private state directly and holds the full letter of the invariant
// after every step of every schedule the drivers below explore —
// stronger than the field doc needs, so a weakening of any clause
// fails here first, by name.

/// Every structural invariant of the digit buffer and the zero-run
/// ledger, checked against the private state.
///
/// Clauses: every digit in the lazy zone; `top` exact (zeros above,
/// nonzero at it unless the buffer is all-zero); the write watermark
/// sound (digits below `bottom` all zero); and per certificate a
/// nonempty interior, an all-zero interior (soundness), containment
/// at or below the settled `top`, and disjointness from every other
/// run (in `lo` order, each run starts at or past the previous run's
/// end).
fn assert_ledger_invariants(acc: &Accumulator, schedule: &[u8]) {
    if acc.quick.is_some() {
        assert!(
            acc.digits.iter().all(|&d| d == 0) && acc.zero_runs.is_empty(),
            "a live register leaves the digit engine idle after {schedule:?}"
        );
        return;
    }
    assert!(
        acc.top < acc.digits.len(),
        "top {} outside the digit buffer (len {}) after {schedule:?}",
        acc.top,
        acc.digits.len()
    );
    for (i, &d) in acc.digits.iter().enumerate() {
        assert!(
            i128::from(d).abs() < super::LAZY_LIMIT,
            "digit {i} = {d} outside the lazy zone after {schedule:?}"
        );
    }
    assert!(
        acc.digits[acc.top + 1..].iter().all(|&d| d == 0),
        "nonzero digit above top {} after {schedule:?}",
        acc.top
    );
    assert!(
        acc.top == 0 || acc.digits[acc.top] != 0,
        "top {} rests on a zero digit after {schedule:?}",
        acc.top
    );
    let floor = acc.bottom.min(acc.digits.len());
    assert!(
        acc.digits[..floor].iter().all(|&d| d == 0),
        "nonzero digit below the write watermark {} after {schedule:?}",
        acc.bottom
    );
    let mut prev_hi = 0usize;
    for (&lo, &hi) in &acc.zero_runs {
        assert!(
            lo + 1 < hi,
            "certificate ({lo}, {hi}) has an empty interior after {schedule:?}"
        );
        assert!(
            hi <= acc.top,
            "certificate ({lo}, {hi}) stranded above the settled top {} \
             after {schedule:?}",
            acc.top
        );
        assert!(
            lo >= prev_hi,
            "certificate ({lo}, {hi}) overlaps the run ending at {prev_hi} \
             after {schedule:?}"
        );
        assert!(
            acc.digits[lo + 1..hi].iter().all(|&d| d == 0),
            "certificate ({lo}, {hi}) covers a nonzero digit after \
             {schedule:?}: a stale interior-zero claim corrupts every scan \
             that consumes it"
        );
        prev_hi = hi;
    }
}

/// Precomputed operands of the exhaustive ledger driver's alphabet.
struct LedgerCtx {
    /// `2^32` as a word-scale magnitude.
    ///
    /// `*_magnitude_shl` deposits it raw at a digit position (no
    /// per-digit canonicalization), the one public route to an
    /// adjacent-digit spelling like `(+1, −2^32)` — the shape that
    /// drives the sign fold's running partial to exact zero above a
    /// certified run.
    word32: UBig,
    /// Oracle values of the shifted ops: `2^96`, `2^224`, `u64::MAX`.
    p96: IBig,
    p224: IBig,
    max64: IBig,
}

/// Ops in the exhaustive ledger driver's alphabet.
const LEDGER_OPS: u8 = 11;

/// Apply one alphabet op to the accumulator and the oracle in
/// lockstep.
///
/// The alphabet reaches every ledger transition within a short
/// schedule: word-scale deltas at digit 0 (including `u64::MAX`,
/// whose deposit recenters across digits 0–1 and, repeated, carries
/// into digit 2 — through a certified run's floor), one-limb jumps to
/// digits 3 and 7 in both signs (above-top certificate inserts at two
/// heights, cancelling rewrites, splits when one lands inside the
/// other's run), the raw `−2^32`/`+2^32` deposit at digit 6 (the
/// cancelling under-digit that walks a sign fold into a certified
/// run with a small nonzero partial, or to an exact-zero partial at
/// its edge), and the collapsing sign read itself.
fn ledger_op(ctx: &LedgerCtx, acc: &mut Accumulator, oracle: &mut IBig, op: u8) {
    match op {
        0 => {
            acc.add_small(1);
            *oracle += 1;
        }
        1 => {
            acc.sub_small(1);
            *oracle -= 1;
        }
        2 => {
            acc.add_u64(u64::MAX);
            *oracle += &ctx.max64;
        }
        3 => {
            acc.sub_u64(u64::MAX);
            *oracle -= &ctx.max64;
        }
        4 => {
            acc.add_wide_shl(&UBig::ONE, 96);
            *oracle += &ctx.p96;
        }
        5 => {
            acc.sub_wide_shl(&UBig::ONE, 96);
            *oracle -= &ctx.p96;
        }
        6 => {
            acc.add_wide_shl(&UBig::ONE, 224);
            *oracle += &ctx.p224;
        }
        7 => {
            acc.sub_wide_shl(&UBig::ONE, 224);
            *oracle -= &ctx.p224;
        }
        8 => {
            acc.sub_magnitude_shl(&ctx.word32, 192);
            *oracle -= &ctx.p224;
        }
        9 => {
            acc.add_magnitude_shl(&ctx.word32, 192);
            *oracle += &ctx.p224;
        }
        _ => {
            assert_eq!(acc.sign(), oracle_sign(oracle), "sign read");
        }
    }
}

/// Depth of the exhaustive ledger sweep.
///
/// Every schedule of at most this many alphabet ops runs, with every
/// invariant checked at every step of every schedule (the search is a
/// prefix tree, so each state is reached and checked exactly once).
const LEDGER_DEPTH: usize = 6;

/// Walk the schedule prefix tree: apply each op to a clone of the
/// parent state, check every invariant and the full oracle value,
/// recurse.
fn ledger_dfs(
    ctx: &LedgerCtx,
    acc: &Accumulator,
    oracle: &IBig,
    schedule: &mut Vec<u8>,
    depth: usize,
) {
    for op in 0..LEDGER_OPS {
        schedule.push(op);
        let mut a = acc.clone();
        let mut o = oracle.clone();
        ledger_op(ctx, &mut a, &mut o, op);
        assert_ledger_invariants(&a, schedule);
        assert_value(&a, &o);
        if depth > 1 {
            ledger_dfs(ctx, &a, &o, schedule, depth - 1);
        }
        schedule.pop();
    }
}

/// The zero-run ledger's letter invariant holds after every operation
/// of every schedule at exhaustive small scope.
///
/// The letter: disjoint certificates with all-zero interiors, every
/// one contained at or below the settled top — checked alongside
/// exact value agreement with the `IBig` oracle.
///
/// Exhaustive over all 11-op schedules of length ≤ 6 (1,948,716
/// states, each checked once; ~4 s dev — the length-≤ 7 sweep's
/// 21.4M states also passed once, at pin time): word-scale deltas,
/// recentering
/// `u64::MAX` deltas, one-limb jumps to digits 3 and 7 in both
/// signs, raw `±2^32` deposits at digit 6, and collapsing sign
/// reads — the space containing every collapse-over-certificate
/// interaction: a fold breaking one step inside a certified run
/// (small nonzero partial over zeros decides at the run's first
/// interior digit, and the collapse re-deposit's crop keeps only the
/// lower remnant), a zero partial consuming a run mid-fold, above-top
/// jump inserts stacking and splitting certificates, and carry runs
/// writing through a run's floor. In particular: no schedule strands
/// a certificate above the settled top, so the containment clause is
/// a standing invariant here, not merely a creation-time fact.
#[test]
fn ledger_invariants_hold_exhaustively() {
    let ctx = LedgerCtx {
        word32: UBig::from(1u64 << 32),
        p96: IBig::from(UBig::ONE << 96usize),
        p224: IBig::from(UBig::ONE << 224usize),
        max64: IBig::from(u64::MAX),
    };
    let acc = fresh(true);
    let oracle = IBig::from(0);
    let mut schedule = Vec::with_capacity(LEDGER_DEPTH);
    ledger_dfs(&ctx, &acc, &oracle, &mut schedule, LEDGER_DEPTH);
}

proptest! {
    /// The ledger's structural invariants hold after every step of
    /// randomized run-forming streams.
    ///
    /// The exhaustive sweep's long-schedule, deep-shift complement:
    /// shifts to 4,096 bits, schedules to 150 ops, sign reads
    /// interleaved throughout.
    #[test]
    fn ledger_invariants_hold_on_run_forming_streams(
        ops in proptest::collection::vec(
            (0u8..5, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),
            1..150,
        ),
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        let mut schedule: Vec<u8> = Vec::new();
        for (step, (arm, limbs, shift)) in ops.iter().enumerate() {
            schedule.push(*arm);
            let value = from_limbs(limbs);
            let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();
            match arm {
                0 => {
                    acc.add_wide_shl(&value, *shift);
                    oracle += scaled;
                }
                1 => {
                    acc.sub_wide_shl(&value, *shift);
                    oracle -= scaled;
                }
                2 => {
                    acc.sub_magnitude_shl(&value, *shift);
                    oracle -= scaled;
                }
                3 => {
                    let delta = limbs[0] as i64;
                    acc.add_small(delta);
                    oracle += delta;
                }
                _ => {
                    prop_assert_eq!(
                        acc.sign(),
                        oracle_sign(&oracle),
                        "sign at step {}",
                        step
                    );
                }
            }
            assert_ledger_invariants(&acc, &schedule);
            if step % 32 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }
}
