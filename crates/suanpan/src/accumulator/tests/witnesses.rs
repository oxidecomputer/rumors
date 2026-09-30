//! Constructed boundary cases that random input generation is unlikely to reach.

use core::cmp::Ordering;

use num_bigint::{BigInt as IBig, BigUint as UBig};

use super::{assert_value, park_extreme_negative_digit, Accumulator, TestBig as _};
use crate::accumulator::digit_index;
use crate::accumulator::small::{SMALL_MAX, SMALL_SHIFT_MAX};

/// The last digit position whose required buffer length fits `usize` is accepted.
#[test]
fn last_addressable_digit_is_accepted() {
    let position = usize::MAX as u128 - 1;
    assert_eq!(digit_index(position), usize::MAX - 1);
}

/// The next digit position panics because its required buffer length cannot fit.
#[test]
#[should_panic(expected = "a nonzero contribution needs an addressable digit position")]
fn first_unaddressable_digit_panics() {
    digit_index(usize::MAX as u128);
}

/// A running partial of exactly 2 does not decide the comparison's sign.
///
/// Lower digits can still overturn that partial, so the scan must continue.
/// The fixture parks digits `[−(2^33 − 1), −(2^33 − 1), 2]`: its top partial
/// is 2, but its full value is `−(2^32 − 1)`. The mirrored fixture checks the
/// opposite-sign boundary.
#[test]
fn comparison_threshold_survives_extreme_cancellation() {
    let mut acc = Accumulator::new();
    park_extreme_negative_digit(&mut acc, 0);
    park_extreme_negative_digit(&mut acc, 1);
    acc.add_value_shl(&UBig::from(2u8), 64);
    // Independent read-out first: the low-to-high carry pass does not
    // share the fold's threshold, so the two paths cross-check.
    let (sign, magnitude) = acc.sign_biguint();
    assert_eq!(
        (sign, magnitude),
        (Ordering::Less, UBig::from((1u64 << 32) - 1)),
        "the exact value is −(2^32 − 1)"
    );
    assert_eq!(
        acc.cmp_zero(),
        Ordering::Less,
        "a top partial of 2 must not decide"
    );
    // The mirrored representation: digits [2^33 − 1, 2^33 − 1, −2] denote
    // +(2^32 − 1), and a partial of −2 at the top must not decide either.
    acc = -acc;
    let (sign, magnitude) = acc.sign_biguint();
    assert_eq!(
        (sign, magnitude),
        (Ordering::Greater, UBig::from((1u64 << 32) - 1)),
        "the mirrored value is +(2^32 − 1)"
    );
    assert_eq!(acc.cmp_zero(), Ordering::Greater);
}

/// A sign decided at digit two is not enough to cover every 64-bit adjustment.
///
/// Digits [−(2^33−1), −(2^33−1), 3] represent 2^64−2^32+1: positive,
/// but smaller than u64::MAX. Subtracting that permitted adjustment flips the sign.
#[test]
fn stability_needs_two_digits_of_clearance() {
    let mut acc = Accumulator::new();
    park_extreme_negative_digit(&mut acc, 0);
    park_extreme_negative_digit(&mut acc, 1);
    acc.add_value_shl(&UBig::from(3u8), 64);
    let stable = acc.cmp_zero_stable_under(64);
    assert_eq!(acc.cmp_zero(), Ordering::Greater, "the value is positive");
    assert_eq!(
        stable, None,
        "deciding one digit lower would claim stability against an \
         adjustment larger than the current value"
    );
    // Why it must be false: a u64 adjustment (covered by floor = 1)
    // flips the sign.
    acc -= u64::MAX;
    assert_eq!(
        acc.cmp_zero(),
        Ordering::Less,
        "u64::MAX exceeds 2^64 − 2^32 + 1"
    );
}

/// A stability guarantee covers signed-digit operands wider in magnitude than
/// their nominal 32-bit positions suggest.
///
/// The operand places 2^33−1 at both covered positions. The larger value is only
/// just large enough to decide one position higher; adding or subtracting the
/// extreme operand must preserve its sign.
#[test]
fn stability_covers_extreme_accumulator_operands() {
    let floor = 1usize;
    // larger: digits [−(2^33 − 1), −(2^33 − 1), −(2^33 − 1), 3], deciding
    // at index 3 = floor + 2 with partial exactly 3.
    let mut larger = Accumulator::new();
    park_extreme_negative_digit(&mut larger, 0);
    park_extreme_negative_digit(&mut larger, 1);
    park_extreme_negative_digit(&mut larger, 2);
    larger.add_value_shl(&UBig::from(3u8), 96);
    let stable = larger.cmp_zero_stable_under((floor as u64 + 1) * 32);
    assert_eq!(
        stable,
        Some(Ordering::Greater),
        "partial 3 at index floor + 2 is the decision edge"
    );
    // operand: an accumulator stored in digits 0..=floor at the digit limit
    // edge — magnitude (2^33 − 1)(2^32 + 1), far beyond any u64.
    let mut operand = Accumulator::new();
    park_extreme_negative_digit(&mut operand, 0);
    park_extreme_negative_digit(&mut operand, 1);
    operand = -operand;
    assert_eq!(
        operand.stored_digit_count() - 1,
        floor,
        "the operand sits at the floor"
    );
    // |larger| > |operand|, so adding or subtracting it cannot flip the sign.
    let mut probe = larger.clone();
    probe -= &operand;
    assert_eq!(
        probe.cmp_zero(),
        Ordering::Greater,
        "larger − operand remains positive"
    );
    let mut probe = larger.clone();
    probe += &operand;
    assert_eq!(
        probe.cmp_zero(),
        Ordering::Greater,
        "larger + operand remains positive"
    );
}

/// A scalar 2^65 cannot guarantee stability against every two-digit accumulator.
///
/// Two coefficients of 2^33−1 represent 2^65+2^32−1. That operand fits the
/// requested stored width but exceeds 2^65 and reverses its sign on subtraction.
#[test]
fn small_value_stability_threshold_is_tight_from_below() {
    let mut larger = Accumulator::new();
    larger += 1_u64 << 35;
    larger <<= 30;
    assert!(
        larger.small.is_some(),
        "2^65 remains in the small representation"
    );
    assert_value(&larger, &(IBig::from(1) << 65usize));
    // operand: every digit 0..=floor parked at the digit limit —
    // magnitude 2^65 + 2^32 − 1, strictly above the larger value 2^65.
    let mut operand = Accumulator::new();
    park_extreme_negative_digit(&mut operand, 0);
    park_extreme_negative_digit(&mut operand, 1);
    operand = -operand;
    assert_eq!(
        operand.stored_digit_count() - 1,
        1,
        "the operand sits at the floor"
    );
    let operand_value = (IBig::from(1) << 65usize) + ((IBig::from(1) << 32usize) - 1);
    assert_value(&operand, &operand_value);
    let stable = larger.cmp_zero_stable_under(64);
    assert_eq!(
        larger.cmp_zero(),
        Ordering::Greater,
        "the value is positive"
    );
    assert_eq!(
        stable, None,
        "2^65 < 3 · 2^64: returning Some would cover a stored operand \
         that is larger than the current value"
    );
    // Why false is required: the in-scope operand flips the sign.
    let mut probe = larger.clone();
    probe -= &operand;
    assert_eq!(
        probe.cmp_zero(),
        Ordering::Less,
        "the extreme-digit representation (2^33 − 1)(2^32 + 1) exceeds 2^65"
    );
}

/// An unrepresentably wide adjustment never yields a stability guarantee.
///
/// The same 2^2048 value can still prove stability for a smaller bit width.
#[test]
fn maximum_adjustment_width_never_decides() {
    let mut acc = Accumulator::new();
    acc.add_limb_value(&(UBig::from(1u8) << 2_048usize));
    let stable = acc.cmp_zero_stable_under(u64::MAX);
    assert_eq!(acc.cmp_zero(), Ordering::Greater, "the value is positive");
    assert_eq!(
        stable, None,
        "no value dominates an adjustment bound wider than the address space"
    );
    // The same value still certifies every in-range floor its decision
    // index covers: the saturation changes nothing below the overflow
    // boundary.
    assert_eq!(acc.cmp_zero_stable_under(1984), Some(Ordering::Greater));
}

/// The exact scalar comparison can prove stability without a high deciding digit.
///
/// u64::MAX exceeds 3·2^32 even though it occupies only two digit positions.
#[test]
fn small_value_can_decide_before_a_digit_scan_could() {
    let mut acc = Accumulator::new();
    acc += u64::MAX;
    assert!(
        acc.small.is_some(),
        "a word-scale add stays in the small representation"
    );
    assert_eq!(acc.stored_digit_count(), 2, "u64::MAX spans two digits");
    assert_eq!(
        acc.cmp_zero_stable_under(32),
        Some(Ordering::Greater),
        "the exact small value proves that u64::MAX ≥ 3 · 2^32"
    );
}

/// Equal values can give different stability answers without contradicting the contract.
///
/// The scalar 2^80 exceeds 3·2^64. Stored as digits, its sign scan stops at
/// index two, which is not high enough to establish the stronger operand bound.
#[test]
fn stability_answer_can_depend_on_representation() {
    let mut acc = Accumulator::new();
    acc += 1_u64 << 20;
    acc <<= 30;
    acc <<= 30;
    assert!(
        acc.small.is_some(),
        "2^80 remains in the small representation"
    );
    assert_eq!(
        acc.cmp_zero_stable_under(64),
        Some(Ordering::Greater),
        "the exact small value proves 2^80 ≥ 3 · 2^64"
    );
    acc.ensure_digits();
    assert_eq!(
        acc.cmp_zero_stable_under(64),
        None,
        "signed-digit comparison stops at index 2, below the required index 3"
    );
}

/// Removing leading cancellation tightens stored width enough for a useful stability test.
///
/// The value 1 initially occupies eleven positions. Comparing it with zero removes that
/// cancellation, after which its reported width lets 5·2^128 prove it is larger.
#[test]
fn comparison_compaction_tightens_the_stored_width() {
    // Adding 2^320 and subtracting 2^320 − 1 leaves value 1 across eleven
    // digits (indices 0 through 10; 320 = 32 · 10). Both operands exceed
    // the small representation's 2^96 limit, so the value uses signed digits.
    let mut cancelled = Accumulator::new();
    cancelled.add_limb_value(&(UBig::ONE << 320usize));
    cancelled.sub_limb_value(&((UBig::ONE << 320usize) - UBig::ONE));
    let stale = cancelled.stored_digit_count();
    assert_eq!(
        stale, 11,
        "the cancelling prefix leaves the stale top at the added operand's width"
    );
    // A comparand with top digit 5 at index 4: decision-bound (5 ≥ 3,
    // the comparison threshold) and too wide for the small representation.
    let mut comparand = Accumulator::new();
    comparand.add_limb_value(&(UBig::from(5u8) << 128usize));
    // Before compaction, the stale width asks for more clearance than the
    // true value needs. The comparison correctly declines to answer.
    assert_eq!(
        comparand.cmp_zero_stable_under(stale as u64 * 32),
        None,
        "a stability width derived from the stale count must return None"
    );
    // Exact comparison compacts the cancellation, making the width tight.
    assert_eq!(cancelled.cmp_zero(), Ordering::Greater);
    assert_eq!(
        cancelled.stored_digit_count(),
        1,
        "after compaction, stored_digit_count reports the true stored top"
    );
    // The collapsed count arms the decision: floor 0 sits two or more
    // digit indexes under the comparand's top, so the read decides.
    assert_eq!(
        comparand.cmp_zero_stable_under(cancelled.stored_bits()),
        Some(Ordering::Greater),
        "the compacted width is small enough for a stable comparison"
    );
}

/// Comparison compaction may write below the input's lowest position.
///
/// For 2^1280, it combines the top two positions and deposits the result one
/// position lower. The scaled read consequently returns (2^32, 1248), not
/// (1, 1280); either pair denotes the same magnitude.
#[test]
fn comparison_compaction_lowers_the_scaled_read_shift() {
    // One unit parked at digit 40: the 1280-bit shift exceeds the
    // small representation's 30-bit shift bound, so the value lives in the digit
    // representation with a one-digit written span at index 40.
    let mut acc = Accumulator::new();
    acc.add_limb_value_shl(&UBig::ONE, 1280);
    // Before comparison, the scaled read prices the written span and
    // returns the whole never-written prefix as the shift.
    let (sign, magnitude, shift) = acc.sign_biguint_shl();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(
        (magnitude, shift),
        (UBig::ONE, 1280),
        "untouched, the scaled read returns the full written-span shift"
    );
    // A top coefficient of 1 cannot determine the sign without reading lower
    // digits. Combining the next position writes the partial sum one digit
    // lower, so the lowest-written position moves with it.
    assert_eq!(acc.cmp_zero(), Ordering::Greater);
    let (sign, magnitude, shift) = acc.sign_biguint_shl();
    assert_eq!(sign, Ordering::Greater);
    assert!(
        shift < 1280,
        "comparison compaction must lower the returned shift"
    );
    assert_eq!(
        (&magnitude, shift),
        (&(UBig::ONE << 32usize), 1248),
        "the collapse re-deposits one digit down: shift 32 · 39, magnitude 2^32"
    );
    // The pair is one valid representation of the unchanged value.
    assert_eq!(
        magnitude << usize::try_from(shift).expect("the shift fits the address space"),
        UBig::ONE << 1280usize,
        "the collapse is value-preserving"
    );
}

/// The representation `[0, −2]` normalizes to `−2^33` across a zero low digit.
///
/// Two deposits of `−2^32` put digit zero at its negative carry boundary,
/// leaving remainder zero and carry −2. Normalization must propagate the
/// complement's carry through that low zero to produce the exact magnitude.
/// Direct assertions on the stored digits ensure this conversion path runs. The
/// property `carry_tie_streams_match_the_oracle` covers surrounding values.
#[test]
fn flush_right_carry_tie_converts_exactly() {
    let mut acc = Accumulator::new();
    acc.ensure_digits();
    acc -= 1_u64 << 32;
    acc -= 1_u64 << 32;
    assert!(
        acc.small.is_none(),
        "the boundary under test lives in the digit representation"
    );
    assert_eq!(
        acc.digits.stored_digits(),
        &[0, -2],
        "the recenter tie leaves remainder 0 and carry −2"
    );
    let (sign, magnitude) = acc.sign_biguint();
    assert_eq!((sign, magnitude), (Ordering::Less, UBig::from(1u64 << 33)));
    assert_eq!(acc.cmp_zero(), Ordering::Less);
}

/// Word updates use the scalar until a wide operand requires digits.
///
/// Subsequent small updates retain the digit representation; reset returns to the scalar.
#[test]
fn small_value_engages_and_retires() {
    let mut acc = Accumulator::new();
    acc += u64::MAX;
    acc -= i64::MIN;
    assert!(
        acc.small.is_some(),
        "word-scale streams stay in the small representation"
    );
    acc.add_limb_value(&(UBig::from(1u8) << 200usize));
    assert!(
        acc.small.is_none(),
        "a wide operand arms the digit representation"
    );
    acc += 1_u64;
    assert!(
        acc.small.is_none(),
        "small updates retain digits until reset"
    );
    acc.reset();
    assert!(
        acc.small.is_some(),
        "reset restores the small representation"
    );
}

/// Empty and one-word limb streams preserve the small-value path, while a
/// two-word stream enters the digit representation without changing the exact value.
#[test]
fn limb_stream_selects_the_narrowest_representation() {
    let mut acc = Accumulator::new();
    acc.add_shifted_limbs(17, []);
    assert!(acc.small.is_some(), "an empty stream changes no state");

    acc.add_shifted_limbs(0, [u64::MAX]);
    assert!(
        acc.small.is_some(),
        "a machine-word stream uses the small-value path"
    );
    assert_value(&acc, &IBig::from(u64::MAX));

    acc.add_shifted_limbs(0, [1, 1]);
    assert!(
        acc.small.is_none(),
        "a two-word stream enters the digit representation"
    );
    assert_value(
        &acc,
        &(IBig::from(u64::MAX) + IBig::from(1u8) + (IBig::from(1u8) << 64usize)),
    );
}

/// A small value at its ceiling, `±2^96`.
fn full_small_value(negative: bool) -> (Accumulator, IBig) {
    let mut acc = Accumulator::new();
    acc += 1_u64 << 36;
    acc <<= 30;
    acc <<= 30;
    if negative {
        acc = -acc;
    }
    assert!(acc.small.is_some(), "the ceiling remains a small value");
    let mut oracle = IBig::from(UBig::ONE << 96usize);
    if negative {
        oracle = -oracle;
    }
    (acc, oracle)
}

/// Every bounded scalar-input path handles its largest intermediate without overflow.
///
/// Apply extreme words and maximally shifted scalar operands at both ±2^96
/// boundaries. Each result must match the independent arbitrary-width oracle.
#[test]
fn small_value_extremes_spill_exactly() {
    // The headroom derivation itself, pinned executable: a full
    // small value plus the widest shifted operand the small path accepts stays
    // strictly inside i128.
    let ceiling = i128::try_from(SMALL_MAX).expect("the ceiling fits i128");
    let widest_fold = ceiling
        .checked_shl(SMALL_SHIFT_MAX as u32)
        .expect("the widest shifted fold fits i128");
    ceiling
        .checked_add(widest_fold)
        .expect("the largest small-path sum fits i128");

    // Word-scale extremes against the parked ceiling, both signs.
    for negative in [false, true] {
        let (mut acc, mut oracle) = full_small_value(negative);
        acc += u64::MAX;
        oracle += u64::MAX;
        assert_value(&acc, &oracle);

        let (mut acc, mut oracle) = full_small_value(negative);
        acc -= u64::MAX;
        oracle -= u64::MAX;
        assert_value(&acc, &oracle);

        let (mut acc, mut oracle) = full_small_value(negative);
        acc += i64::MIN;
        oracle += i64::MIN;
        assert_value(&acc, &oracle);

        let (mut acc, mut oracle) = full_small_value(negative);
        acc -= i64::MIN;
        oracle -= i64::MIN;
        assert_value(&acc, &oracle);

        // The widest shifted word the small path accepts.
        let (mut acc, mut oracle) = full_small_value(negative);
        acc.add_value_shl(&UBig::from(u64::MAX), SMALL_SHIFT_MAX);
        oracle += IBig::from(u64::MAX) << SMALL_SHIFT_MAX as usize;
        assert_value(&acc, &oracle);

        // The widest small-to-small updates, in both directions.
        for (fold_negative, subtract) in
            [(false, false), (false, true), (true, false), (true, true)]
        {
            let (mut acc, mut oracle) = full_small_value(negative);
            let (operand, operand_oracle) = full_small_value(fold_negative);
            let scaled = operand_oracle << SMALL_SHIFT_MAX as usize;
            if subtract {
                acc.sub_shifted(SMALL_SHIFT_MAX, &operand);
                oracle -= scaled;
            } else {
                acc.add_shifted(SMALL_SHIFT_MAX, &operand);
                oracle += scaled;
            }
            assert_value(&acc, &oracle);
        }

        // The widest in-place shift of the largest small value.
        let (mut acc, mut oracle) = full_small_value(negative);
        acc <<= SMALL_SHIFT_MAX;
        oracle <<= SMALL_SHIFT_MAX as usize;
        assert!(acc.small.is_none(), "the shifted ceiling spills");
        assert_value(&acc, &oracle);

        // Negation at the ceiling stays in the small representation.
        let (mut acc, mut oracle) = full_small_value(negative);
        acc = -acc;
        oracle = -oracle;
        assert!(acc.small.is_some(), "the zone is symmetric about zero");
        assert_value(&acc, &oracle);
    }
}

/// Scalar and digit reads agree at zero and across the 64-bit limb boundary.
///
/// Both signs are checked at u64::MAX, 2^64, and a two-limb composite.
/// High zero limbs disappear; the low zero limb of 2^64 must remain.
#[test]
fn normalized_limb_conversion_corners() {
    // Zero: empty limbs in both representations.
    let mut zero = Accumulator::new();
    assert_eq!(
        {
            let (sign, limbs) = zero.signed_magnitude();
            (sign, limbs.as_ref().to_vec())
        },
        (Ordering::Equal, vec![])
    );
    zero.ensure_digits();
    assert_eq!(
        {
            let (sign, limbs) = zero.signed_magnitude();
            (sign, limbs.as_ref().to_vec())
        },
        (Ordering::Equal, vec![])
    );

    // (value, expected LE limbs): the u64 ceiling, the limb boundary at
    // 2^64 (interior zero limb kept), and a two-limb composite.
    let corners: [(u128, Vec<u64>); 3] = [
        (u128::from(u64::MAX), vec![u64::MAX]),
        (1u128 << 64, vec![0, 1]),
        ((7u128 << 64) | 5, vec![5, 7]),
    ];
    for (value, limbs) in corners {
        for negative in [false, true] {
            for spill in [false, true] {
                // Word deposits and bounded shifts keep the unspilled case in
                // the small representation (30 + 30 + 4 covers one limb).
                let mut acc = Accumulator::new();
                acc += (value >> 64) as u64;
                acc <<= 30;
                acc <<= 30;
                acc <<= 4;
                acc += value as u64;
                if negative {
                    acc = -acc;
                }
                if spill {
                    acc.ensure_digits();
                } else {
                    assert!(acc.small.is_some(), "the construction stays small");
                }
                let sign = if negative {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
                assert_eq!(
                    {
                        let (sign, limbs) = acc.signed_magnitude();
                        (sign, limbs.as_ref().to_vec())
                    },
                    (sign, limbs.clone()),
                    "value {value}, negative {negative}, spilled {spill}"
                );
            }
        }
    }
}

/// Reserving capacity preserves values in both scalar and digit representations.
///
/// Reservations below the stored width change nothing, and later writes behave
/// identically whether or not their destination was reserved.
#[test]
fn reserve_digits_is_value_neutral() {
    // For a small value, the reservation prepares the inactive digit buffer
    // without changing the active representation or value.
    let mut acc = Accumulator::new();
    acc += 7_i64;
    acc.reserve_digits(100);
    assert!(
        acc.small.is_some(),
        "reserving storage does not change representation"
    );
    let mut oracle = IBig::from(7);
    assert_value(&acc, &oracle);

    // In the digit representation, before and after the covered writes — and a
    // reservation smaller than the stored width is a no-op.
    acc.add_limb_value(&(UBig::from(1u8) << 3_200usize));
    oracle += IBig::from(UBig::from(1u8) << 3_200usize);
    acc.reserve_digits(500);
    assert_value(&acc, &oracle);
    acc.reserve_digits(1);
    assert_value(&acc, &oracle);
    acc.sub_limb_value(&(UBig::from(1u8) << 12_800usize));
    oracle -= IBig::from(UBig::from(1u8) << 12_800usize);
    assert_value(&acc, &oracle);
}
