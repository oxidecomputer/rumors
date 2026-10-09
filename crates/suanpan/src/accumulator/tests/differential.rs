//! Differential streams against the exact `IBig` oracle.
//!
//! Randomized mixed word/limb operation streams compare the sign after
//! every operation — the read a caller's interleaved sweeps depend on —
//! and the full value at periodic snapshots; deterministic streams pin
//! the difficult shapes the representation exists to survive: the
//! boundary-comb ±1 oscillation across a high carry cliff, limb-stream teeth
//! across a higher cliff, and cancelling-prefix chains that force the
//! sign fold below the top digit.

use core::cmp::Ordering;

use num_bigint::{BigInt as IBig, BigUint as UBig};
use proptest::prelude::*;

use super::{
    assert_value, fresh, from_limbs, oracle_sign, park_extreme_negative_digit, Accumulator,
    TestBig as _,
};
use crate::accumulator::operand::Update;
use crate::accumulator::small::SMALL_SHIFT_MAX;

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
    /// A wide delta entering as a raw little-endian limb stream — the
    /// wider-than-the-backend entry — optionally padded with high zero
    /// limbs (value-neutral by contract).
    LimbsShl {
        negative: bool,
        limbs: Vec<u64>,
        shift: u64,
    },
    /// An allocation-shaping reservation: value-neutral by contract.
    Reserve(u64),
}

/// Apply one operation to the accumulator and the oracle in lockstep.
fn apply(acc: &mut Accumulator, oracle: &mut IBig, op: &Op) {
    match op {
        Op::Small(delta) => {
            *acc += *delta;
            *oracle += *delta;
        }
        Op::Wide { negative, value } => {
            if *negative {
                acc.sub_limb_value(value);
                *oracle -= IBig::from(value.clone());
            } else {
                acc.add_limb_value(value);
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
                acc.sub_limb_value_shl(value, *shift);
                *oracle -= scaled;
            } else {
                acc.add_limb_value_shl(value, *shift);
                *oracle += scaled;
            }
        }
        Op::LimbsShl {
            negative,
            limbs,
            shift,
        } => {
            let scaled = IBig::from(from_limbs(limbs)) << usize::try_from(*shift).unwrap();
            if *negative {
                acc.sub_shifted_limbs(*shift, limbs.iter().copied());
                *oracle -= scaled;
            } else {
                acc.add_shifted_limbs(*shift, limbs.iter().copied());
                *oracle += scaled;
            }
        }
        Op::Reserve(bits) => {
            acc.reserve_bits(*bits);
        }
    }
}

/// A mixed operation stream: mostly small deltas of varying width, some
/// dense random wide deltas, and some all-ones/all-zeros "cliffy" wide
/// deltas whose application sits exactly on carry boundaries.
fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (any::<i64>(), 0u32..60).prop_map(|(value, narrowing)| Op::Small(value >> narrowing)),
        1 => (proptest::collection::vec(any::<u64>(), 1..=6), any::<bool>()).prop_map(
            |(limbs, negative)| Op::Wide {
                negative,
                value: from_limbs(&limbs),
            }
        ),
        1 => (proptest::collection::vec(any::<bool>(), 1..=6), any::<bool>()).prop_map(
            |(mask, negative)| {
                let mut limbs: Vec<u64> =
                    mask.iter().map(|&saturated| if saturated { u64::MAX } else { 0 }).collect();
                if limbs.iter().all(|&limb| limb == 0) {
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
        1 => (
            proptest::collection::vec(any::<u64>(), 1..=4),
            0usize..3,
            any::<bool>(),
            0u64..512,
        )
            .prop_map(|(mut limbs, zero_pad, negative, shift)| {
                // High zero limbs are contractually value-neutral: pad
                // some streams so the padding arm stays exercised.
                limbs.extend(std::iter::repeat_n(0, zero_pad));
                Op::LimbsShl {
                    negative,
                    limbs,
                    shift,
                }
            }),
        1 => (0u64..2_048).prop_map(Op::Reserve),
    ]
}

/// How the stability property's larger value is built.
#[derive(Debug, Clone)]
enum StabilityValue {
    /// A mixed stream that varies where comparisons become decisive.
    Stream { ops: Vec<Op>, digits_first: bool },
    /// A small `m · 2^(32·(floor + 1) − 31) + delta`, with either sign.
    ///
    /// The value reaches its scale either through in-place shifts or by adding
    /// the previous value into a new accumulator at the same shift. Both paths
    /// cross `SMALL_SHIFT_MAX`, and `spill` repeats the comparison after an
    /// explicit move to signed digits.
    Small {
        m: u64,
        delta: u64,
        negative: bool,
        spill: bool,
        fold_shift: bool,
    },
}

/// A larger value paired with the digit position through which adjustments fit.
///
/// Stream-built values draw any floor; small values
/// concentrate `m` (the multiplier in units of `2^31`) around the
/// small-value path's decision boundary `3 · 2^(32·(floor + 1))` and
/// the redundant-operand bound just above `2 · 2^(32·(floor + 1))`,
/// spanning multipliers 1.5..4 in between. Small-value floors stop at 1
/// because a wider floor puts the boundary beyond `2^96`.
fn arb_stability_value() -> impl Strategy<Value = (StabilityValue, usize)> {
    prop_oneof![
        1 => (proptest::collection::vec(arb_op(), 1..60), any::<bool>(), 0usize..8).prop_map(
            |(ops, digits_first, floor)| (StabilityValue::Stream { ops, digits_first }, floor)
        ),
        1 => (
            prop_oneof![
                2 => Just(1u64 << 32),           // multiplier 2: the operand bound
                1 => Just(3u64 << 31),           // multiplier 3: the decision boundary
                3 => (3u64 << 30)..(1u64 << 33), // multipliers spanning 1.5..4
            ],
            prop_oneof![
                2 => Just(0u64),
                3 => 0u64..(1 << 32),
                1 => 0u64..(1 << 34),
            ],
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            0usize..=1,
        )
            .prop_map(|(m, delta, negative, spill, fold_shift, floor)| {
                (
                    StabilityValue::Small {
                        m,
                        delta,
                        negative,
                        spill,
                        fold_shift,
                    },
                    floor,
                )
            }),
    ]
}

/// Materialize a stability input and its exact oracle value.
fn build_stability_value(value: &StabilityValue, floor: usize) -> (Accumulator, IBig) {
    match value {
        StabilityValue::Stream { ops, digits_first } => {
            let mut acc = fresh(*digits_first);
            let mut oracle = IBig::from(0);
            for op in ops {
                apply(&mut acc, &mut oracle, op);
            }
            (acc, oracle)
        }
        StabilityValue::Small {
            m,
            delta,
            negative,
            spill,
            fold_shift,
        } => {
            let mut acc = Accumulator::new();
            acc += *m;
            // Shift in chunks the small representation accepts, either in
            // place or by adding the previous value into a fresh accumulator.
            let mut remaining = 32 * (floor as u64 + 1) - 31;
            while remaining > 0 {
                let step = remaining.min(SMALL_SHIFT_MAX);
                if *fold_shift {
                    let operand = core::mem::take(&mut acc);
                    acc.add_shifted(step, &operand);
                } else {
                    acc <<= step;
                }
                remaining -= step;
            }
            acc += *delta;
            if *negative {
                acc = -acc;
            }
            assert!(
                acc.small.is_some(),
                "the small-value arm remains small until its requested spill"
            );
            let mut oracle = (IBig::from(*m) << (32 * (floor + 1) - 31)) + IBig::from(*delta);
            if *negative {
                oracle = -oracle;
            }
            if *spill {
                acc.ensure_digits();
            }
            (acc, oracle)
        }
    }
}

/// One digit of an accumulator-operand probe.
#[derive(Debug, Clone, Copy)]
enum ProbeDigit {
    /// Parked at the lazy digit limit: `−(2^33 − 1)`.
    Extreme,
    /// A single word deposit `−w`, `w < 2^32`.
    Word(u64),
    /// No deposit at this index.
    Absent,
}

/// A probe digit, biased toward the digit limit — the representations whose
/// magnitude exceeds any plain probe under the same floor.
fn arb_probe_digit() -> impl Strategy<Value = ProbeDigit> {
    prop_oneof![
        3 => Just(ProbeDigit::Extreme),
        2 => (1u64..(1 << 32)).prop_map(ProbeDigit::Word),
        1 => Just(ProbeDigit::Absent),
    ]
}

/// Materialize an accumulator operand stored in digits
/// `0..=floor`, as the accumulator and its exact oracle.
///
/// `all_extreme` overrides every digit spec to the digit limit — the
/// extremal operand of the floor, up to `(2^33 − 1)` per digit —
/// so the strategy carries a point mass at the strongest probe.
fn build_probe(
    digits: &[ProbeDigit],
    all_extreme: bool,
    negate: bool,
    floor: usize,
) -> (Accumulator, IBig) {
    let mut probe = Accumulator::new();
    // Force the signed-digit representation so the contributions remain
    // separate coefficients rather than combining as one small integer.
    probe.ensure_digits();
    let mut oracle = IBig::from(0);
    for (index, spec) in digits[..=floor].iter().enumerate() {
        let spec = if all_extreme {
            ProbeDigit::Extreme
        } else {
            *spec
        };
        match spec {
            ProbeDigit::Extreme => {
                park_extreme_negative_digit(&mut probe, index as u64);
                oracle -= IBig::from((1u64 << 33) - 1) << (32 * index);
            }
            ProbeDigit::Word(w) => {
                probe.sub_value_shl(&UBig::from(w), 32 * index as u64);
                oracle -= IBig::from(w) << (32 * index);
            }
            ProbeDigit::Absent => {}
        }
    }
    if negate {
        probe = -probe;
        oracle = -oracle;
    }
    (probe, oracle)
}

proptest! {
    /// Reset restores the small representation, and a later wide stream
    /// reuses the retained digit buffer without observing the old value.
    ///
    /// Two independent streams with a reset between them: after the
    /// reset the accumulator equals fresh zero, and the second stream —
    /// whose wide operands re-arm the digit representation over the buffer the
    /// first stream grew — matches an oracle that starts from zero.
    #[test]
    fn pooled_reuse_after_reset_matches_the_oracle(
        first in proptest::collection::vec(arb_op(), 1..60),
        second in proptest::collection::vec(arb_op(), 1..60),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for op in &first {
            apply(&mut acc, &mut oracle, op);
        }
        acc.reset();
        let mut oracle = IBig::from(0);
        assert_value(&acc, &oracle);
        for (step, op) in second.iter().enumerate() {
            apply(&mut acc, &mut oracle, op);
            prop_assert_eq!(acc.cmp_zero(), oracle_sign(&oracle), "sign at step {}", step);
        }
        assert_value(&acc, &oracle);
    }

    /// Stability answers cover both normalized magnitudes and signed-digit operands.
    ///
    /// For each requested digit-aligned width, check the reported sign and apply
    /// both signs of an operand within that width. The larger magnitude must exceed
    /// the operand's magnitude. Small values additionally pin the exact
    /// 3·2^bits threshold; constructed signed digits exercise the extra magnitude
    /// allowed by redundant coefficients. A refusal must compact the receiver
    /// to within two stored digits of the requested width. If the other operand
    /// is then wider, checking it in turn bounds both sides before an exact
    /// subtraction.
    #[test]
    fn stored_width_stability_is_sound(
        (value, floor) in arb_stability_value(),
        probe_limbs in proptest::collection::vec(any::<u64>(), 1..=4),
        probe_negative: bool,
        probe_digits in proptest::collection::vec(arb_probe_digit(), 8),
        probe_all_extreme: bool,
        probe_negate: bool,
    ) {
        let (mut acc, oracle) = build_stability_value(&value, floor);
        let was_small = acc.small.is_some();
        let stable = acc.cmp_zero_stable_under((floor as u64 + 1) * 32);
        let sign = oracle_sign(&oracle);
        if let Some(reported) = stable {
            prop_assert_eq!(reported, sign);
        } else {
            prop_assert!(
                acc.stored_digit_count() <= floor + 3,
                "a refusal leaves at most two digits above the operand width"
            );
        }
        assert_value(&acc, &oracle);
        if was_small {
            // The small path's contract is exact, not merely sound:
            // the bound is the direct magnitude comparison.
            let (_, magnitude) = acc.sign_biguint();
            prop_assert_eq!(
                stable.is_some(),
                magnitude >= UBig::from(3u8) << (32 * (floor + 1)),
                "a small-value verdict is exactly the comparison \
                 against 3 · 2^(32·(floor + 1))"
            );
        }
        if stable.is_some() {
            // The largest plain operand the verdict covers: top digit
            // index at most `floor`, so at most 32·(floor + 1) bits.
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
            // The accumulator-operand clause: a redundant signed
            // representation in digits 0..=floor can exceed every plain
            // magnitude under the floor, and the verdict covers it too.
            let (operand, operand_oracle) =
                build_probe(&probe_digits, probe_all_extreme, probe_negate, floor);
            prop_assert!(
                operand.stored_digit_count() <= floor + 1,
                "the probe operand sits at or below the floor"
            );
            assert_value(&operand, &operand_oracle);
            let (_, larger_magnitude) = acc.sign_biguint();
            let (_, operand_magnitude) = operand.sign_biguint();
            prop_assert!(
                larger_magnitude > operand_magnitude,
                "a decided verdict implies the larger magnitude strictly \
                 exceeds any operand stored in digits 0..=floor"
            );
            let mut folded = acc.clone();
            folded -= &operand;
            prop_assert_eq!(
                folded.cmp_zero(), sign,
                "a decided verdict survives subtracting an operand stored \
                 in digits 0..=floor"
            );
            let mut folded = acc.clone();
            folded += &operand;
            prop_assert_eq!(
                folded.cmp_zero(), sign,
                "a decided verdict survives adding an operand stored in \
                 digits 0..=floor"
            );
        }
    }

    /// Alternating stability refusals converge to a cheap exact comparison.
    ///
    /// Both operands range across the scalar and signed-digit representations,
    /// including shifted updates, cancellation, and extreme coefficients. The
    /// test follows the comparison protocol used by clients: inspect the wider
    /// operand, reverse roles whenever compaction exposes the other as wider,
    /// and subtract only once their stored widths differ by at most two digits.
    /// Every refusal must strictly reduce the combined retained width.
    #[test]
    fn two_sided_stability_converges_to_exact_comparison(
        (left_value, left_floor) in arb_stability_value(),
        (right_value, right_floor) in arb_stability_value(),
    ) {
        let (mut left, left_oracle) = build_stability_value(&left_value, left_floor);
        let (mut right, right_oracle) = build_stability_value(&right_value, right_floor);
        let expected = left_oracle.cmp(&right_oracle);
        let close = |left: &Accumulator, right: &Accumulator| {
            left.stored_digit_count().abs_diff(right.stored_digit_count()) <= 2
        };

        let initial_width = left.stored_digit_count() + right.stored_digit_count();
        let mut decided = None;
        let mut refusals = 0;
        while !close(&left, &right) {
            let previous_width = left.stored_digit_count() + right.stored_digit_count();
            if left.stored_digit_count() > right.stored_digit_count() {
                match left.cmp_zero_stable_under(right.stored_bits()) {
                    Some(ordering) => {
                        decided = Some(ordering);
                        break;
                    }
                    None => refusals += 1,
                }
            } else {
                match right.cmp_zero_stable_under(left.stored_bits()) {
                    Some(ordering) => {
                        decided = Some(ordering.reverse());
                        break;
                    }
                    None => refusals += 1,
                }
            }
            prop_assert!(
                left.stored_digit_count() + right.stored_digit_count() < previous_width,
                "every refusal must compact the wider operand"
            );
        }

        prop_assert!(refusals < initial_width);
        let actual = if let Some(ordering) = decided {
            ordering
        } else {
            prop_assert!(
                close(&left, &right),
                "all far-wider cancellation must be compacted before subtraction"
            );
            let mut difference = left.clone();
            difference -= &right;
            difference.cmp_zero()
        };
        prop_assert_eq!(actual, expected);
        assert_value(&left, &left_oracle);
        assert_value(&right, &right_oracle);
    }

    /// `cmp_zero_stable_under(64)` never lies: the sign always matches the
    /// oracle's, and a `Some` result implies the current magnitude
    /// exceeds any machine word — folding any `u64` afterward cannot
    /// flip the sign.
    #[test]
    fn word_width_stability_is_sound(
        ops in proptest::collection::vec(arb_op(), 1..60),
        probe: u64,
        probe_negative: bool,
        digits_first: bool,
    ) {
        let mut acc = fresh(digits_first);
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
        }
        let stable = acc.cmp_zero_stable_under(64);
        let sign = oracle_sign(&oracle);
        if let Some(reported) = stable {
            prop_assert_eq!(reported, sign);
        }
        assert_value(&acc, &oracle);
        if stable.is_some() {
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

    /// Digit-aligned updates normalize exactly when carries leave zero remainders.
    ///
    /// Updates by multiples of 2^32 force recentered digits to zero. Reading their
    /// magnitude must propagate the complement carry across those zero positions,
    /// at every generated offset and for both signs. Read before asking for the
    /// comparison so it cannot first simplify the representation under test.
    #[test]
    fn carry_tie_streams_match_the_oracle(
        ops in proptest::collection::vec(
            (
                prop_oneof![3 => 1u64..=4, 1 => 1u64..(1 << 32)],
                0u64..6,
                any::<bool>(),
            ),
            1..40,
        ),
    ) {
        let mut acc = Accumulator::new();
        // This boundary exists only in signed digits; a small value is read
        // directly and never recentered.
        acc.ensure_digits();
        let mut oracle = IBig::from(0);
        for (multiple, index, negative) in &ops {
            let delta = UBig::from(multiple << 32);
            let scaled =
                IBig::from(delta.clone()) << usize::try_from(32 * index).unwrap();
            if *negative {
                acc.sub_value_shl(&delta, 32 * index);
                oracle -= scaled;
            } else {
                acc.add_value_shl(&delta, 32 * index);
                oracle += scaled;
            }
            // Raw representation first: the read-out samples the complement
            // boundary before comparison with zero compacts the digits.
            assert_value(&acc, &oracle);
            prop_assert_eq!(acc.cmp_zero(), oracle_sign(&oracle));
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
    let cliff_bits = 512u32;
    let below_cliff = (UBig::from(1u8) << cliff_bits as usize) - 1u8;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_limb_value(&below_cliff);
    oracle += IBig::from(below_cliff);
    for _ in 0..2_000 {
        acc += 1_i64;
        oracle += 1;
        assert_eq!(acc.cmp_zero(), Ordering::Greater, "above the cliff");
        acc -= 1_i64;
        oracle -= 1;
        assert_eq!(acc.cmp_zero(), Ordering::Greater, "back below the cliff");
    }
    assert_value(&acc, &oracle);
}

/// The wide-tooth stream: ±2^w teeth oscillating across a `2^k` cliff
/// (w far past any machine-word window) stay sign-correct at every step
/// and value-correct at the end.
#[test]
fn wide_teeth_across_the_cliff_match_the_oracle() {
    let (cliff_bits, tooth_bits) = (512u32, 192u32);
    let cliff = UBig::from(1u8) << cliff_bits as usize;
    let tooth = UBig::from(1u8) << tooth_bits as usize;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_limb_value(&cliff);
    oracle += IBig::from(cliff);
    for _ in 0..500 {
        acc.sub_limb_value(&tooth);
        oracle -= IBig::from(tooth.clone());
        assert_eq!(acc.cmp_zero(), Ordering::Greater, "below the cliff");
        acc.add_limb_value(&tooth);
        oracle += IBig::from(tooth.clone());
        assert_eq!(acc.cmp_zero(), Ordering::Greater, "back at the cliff");
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
    let peak_bits = 512u32;
    let peak = UBig::from(1u8) << peak_bits as usize;
    let descent = (UBig::from(1u8) << peak_bits as usize) - 1u8;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_limb_value(&peak);
    oracle += IBig::from(peak);
    assert_eq!(acc.cmp_zero(), Ordering::Greater);
    for cycle in 0..200 {
        acc.sub_limb_value(&descent);
        oracle -= IBig::from(descent.clone());
        assert_eq!(acc.cmp_zero(), Ordering::Greater, "down at 1");
        acc.add_limb_value(&descent);
        oracle += IBig::from(descent.clone());
        assert_eq!(acc.cmp_zero(), Ordering::Greater, "back at the peak");
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
    acc.add_limb_value(&wide);
    oracle += IBig::from(wide.clone());
    acc.sub_limb_value(&wide);
    oracle -= IBig::from(wide);
    assert_eq!(
        acc.cmp_zero(),
        Ordering::Equal,
        "exact cancellation is zero"
    );
    assert_value(&acc, &oracle);
    acc -= 1_i64;
    oracle -= 1;
    assert_eq!(acc.cmp_zero(), Ordering::Less, "one below zero");
    assert_value(&acc, &oracle);
    acc += 2_i64;
    oracle += 2;
    assert_eq!(acc.cmp_zero(), Ordering::Greater, "one above zero");
    assert_value(&acc, &oracle);
}

/// Negative limb streams read back correctly with either a nonzero low part or
/// an exact multiple of the digit base.
#[test]
fn negative_limb_values_read_back_exactly() {
    // Nonzero low part: −(2^192 − 5).
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    let wide = (UBig::from(1u8) << 192usize) - 5u8;
    acc.sub_limb_value(&wide);
    oracle -= IBig::from(wide);
    assert_eq!(acc.cmp_zero(), Ordering::Less);
    assert_value(&acc, &oracle);
    // Exact multiple of 2^32: −2^96.
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    let aligned = UBig::from(1u8) << 96usize;
    acc.sub_limb_value(&aligned);
    oracle -= IBig::from(aligned);
    assert_eq!(acc.cmp_zero(), Ordering::Less);
    assert_value(&acc, &oracle);
}

/// The unsigned machine-word entry points cover the full `u64` range,
/// including values past `i64::MAX`, in both directions.
#[test]
fn u64_entry_points_cover_the_full_range() {
    for digits in [false, true] {
        let mut acc = fresh(digits);
        let mut oracle = IBig::from(0);
        acc += u64::MAX;
        oracle += u64::MAX;
        assert_eq!(acc.cmp_zero(), Ordering::Greater);
        assert_value(&acc, &oracle);
        acc -= u64::MAX;
        oracle -= u64::MAX;
        assert_eq!(acc.cmp_zero(), Ordering::Equal);
        assert_value(&acc, &oracle);
        acc -= u64::MAX;
        oracle -= u64::MAX;
        assert_eq!(acc.cmp_zero(), Ordering::Less);
        assert_value(&acc, &oracle);
    }
}

/// Cancelling digits `[−2^32, 1]` are not visibly zero until comparison
/// collapses them, although normalized readout already reports exact zero.
#[test]
fn redundant_zero_reads_nonzero_until_collapsed() {
    // Start in the digit representation deliberately: the test needs two cancelling
    // digits, not a particular public operation's dispatch choice.
    let mut acc = fresh(true);
    acc.digits.apply_limbs(core::iter::once(1), Update::Add, 32);
    acc -= 1_i64 << 32;
    let (sign, magnitude) = acc.sign_biguint();
    assert_eq!((sign, magnitude), (Ordering::Equal, UBig::ZERO));
    assert!(
        !acc.is_known_zero(),
        "cancelling digits spell zero redundantly"
    );
    assert_eq!(acc.cmp_zero(), Ordering::Equal);
    assert!(
        acc.is_known_zero(),
        "comparison with zero compacts the representation to visible zero"
    );
}

/// A fresh accumulator (and its `Default`) holds exactly zero.
#[test]
fn new_and_default_hold_zero() {
    for mut acc in [Accumulator::new(), Accumulator::default()] {
        assert_eq!(acc.cmp_zero(), Ordering::Equal);
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!(sign, Ordering::Equal);
        assert_eq!(magnitude, UBig::from(0u8));
    }
}
