//! Constructed corner-case witnesses: inputs no random stream reaches,
//! each pinning a decision constant, a headroom bound, or a
//! conversion-path corner at its tight edge.
//!
//! The extreme-cancellation witnesses exist because their mutations
//! passed everything else: `SIGN_DECIDED: 3 → 2` and a decision index
//! of `floor + 1` each survived the whole differential suite, so the
//! tight corners are pinned here by construction.

use core::cmp::Ordering;

use dashu_int::{IBig, UBig};

use super::{assert_value, Accumulator};
use crate::accumulator::{QUICK_MAX, QUICK_SHIFT_MAX};

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
/// certifies domination here — passed the entire committed test suite;
/// a caller consumes the certificate to skip folds entirely, so a
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
/// *accumulator* operand held in digits `0..=floor` — the clause
/// scale-disparate comparisons lean on, and the one leg the
/// differential proptests cannot reach.
///
/// A caller comparing accumulators reads
/// `sign_dominates_at(other.digit_count() - 1)` before
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
    // held: digits [−(2^33 − 1), −(2^33 − 1), −(2^33 − 1), 3], deciding
    // at index 3 = floor + 2 with partial exactly 3.
    let mut held = Accumulator::new();
    park_extreme_negative_digit(&mut held, 0);
    park_extreme_negative_digit(&mut held, 1);
    park_extreme_negative_digit(&mut held, 2);
    held.add_magnitude_shl(&UBig::from(3u8), 96);
    let (sign, decided) = held.sign_dominates_at(floor);
    assert_eq!(sign, Ordering::Greater);
    assert!(decided, "partial 3 at index floor + 2 is the decision edge");
    // operand: an accumulator held in digits 0..=floor at the zone's
    // edge — magnitude (2^33 − 1)(2^32 + 1), far beyond any u64.
    let mut operand = Accumulator::new();
    park_extreme_negative_digit(&mut operand, 0);
    park_extreme_negative_digit(&mut operand, 1);
    operand.negate();
    assert_eq!(
        operand.digit_count() - 1,
        floor,
        "the operand sits at the floor"
    );
    // |held| > |operand| and folding ±operand cannot flip the sign.
    let mut probe = held.clone();
    probe.sub_accum(&operand);
    assert_eq!(
        probe.sign(),
        Ordering::Greater,
        "held − operand keeps held's sign"
    );
    let mut probe = held.clone();
    probe.add_accum(&operand);
    assert_eq!(
        probe.sign(),
        Ordering::Greater,
        "held + operand keeps held's sign"
    );
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
