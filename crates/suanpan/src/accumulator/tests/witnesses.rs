//! Constructed corner-case witnesses: inputs no random stream reaches,
//! each pinning a decision constant, a headroom bound, a
//! conversion-path corner, or a documented collapse side effect at its
//! tight edge.
//!
//! The extreme-cancellation witnesses exist because their mutations
//! passed everything else: `SIGN_DECIDED: 3 → 2` and a decision index
//! of `floor + 1` each survived the whole differential suite, so the
//! tight corners are pinned here by construction.

use core::cmp::Ordering;

use dashu_int::{IBig, UBig};

use super::{assert_value, park_extreme_negative_digit, Accumulator};
use crate::accumulator::{QUICK_MAX, QUICK_SHIFT_MAX};

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

/// The register arm's certification constant is tight from below: a
/// register-held `2^65` at `floor = 1` must read `decided = false`,
/// because an operand held in digits `0..=1` can spell more than `2^65`.
///
/// The bound `3 · 2^(32·(floor + 1))` is the smallest whole multiple
/// clearing the `2.01 · 2^(32·(floor + 1))` redundant-spelling operand
/// bound: an operand parked at the zone's edge in digits `0..=1`
/// carries `(2^33 − 1)(2^32 + 1) = 2^65 + 2^32 − 1 > 2^65`, so a
/// certificate at `2^65` would cover an operand larger than the held
/// value — weakening the constant to 2 certifies exactly here, and the
/// fold at the tail flips the sign. The generalized family is the
/// accumulator-operand probe arm of `floor_domination_is_sound`; this
/// witness is its deterministic tripwire at the gap value.
#[test]
fn register_domination_constant_is_tight_from_below() {
    let mut held = Accumulator::new();
    held.add_u64(1 << 35);
    held.shl(30);
    assert!(
        held.quick.is_some(),
        "2^65 built through register entry points stays register-held"
    );
    assert_value(&held, &(IBig::from(1) << 65usize));
    // operand: every digit 0..=floor parked at the zone's edge —
    // magnitude 2^65 + 2^32 − 1, strictly above the held 2^65.
    let mut operand = Accumulator::new();
    park_extreme_negative_digit(&mut operand, 0);
    park_extreme_negative_digit(&mut operand, 1);
    operand.negate();
    assert_eq!(
        operand.digit_count() - 1,
        1,
        "the operand sits at the floor"
    );
    let operand_value = (IBig::from(1) << 65usize) + ((IBig::from(1) << 32usize) - 1);
    assert_value(&operand, &operand_value);
    let (sign, decided) = held.sign_dominates_at(1);
    assert_eq!(sign, Ordering::Greater, "the sign itself is exact");
    assert!(
        !decided,
        "2^65 < 3 · 2^64: certifying would cover an operand held in \
         digits 0..=floor that exceeds the held value"
    );
    // Why false is required: the in-scope operand flips the sign.
    let mut probe = held.clone();
    probe.sub_accum(&operand);
    assert_eq!(
        probe.sign(),
        Ordering::Less,
        "the zone-edge spelling (2^33 − 1)(2^32 + 1) exceeds 2^65"
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

/// A register-held value certifies domination by direct magnitude
/// comparison, deciding values no fold could.
///
/// `u64::MAX` spans two digits — below any deciding fold's index for
/// `floor = 0` — yet the register reads `(Greater, true)` because
/// `u64::MAX ≥ 3 · 2^32`. Pins the register arm of the contract:
/// `decided` on a register-held value is exactly
/// `|value| ≥ 3 · 2^(32·(floor + 1))`, with no fold mechanics involved.
#[test]
fn register_certifies_domination_below_any_deciding_fold() {
    let mut acc = Accumulator::new();
    acc.add_u64(u64::MAX);
    assert!(
        acc.quick.is_some(),
        "a word-scale add stays in the register"
    );
    assert_eq!(acc.digit_count(), 2, "u64::MAX spans two digits");
    assert_eq!(
        acc.sign_dominates_at(0),
        (Ordering::Greater, true),
        "the direct comparison certifies: u64::MAX ≥ 3 · 2^32"
    );
}

/// The domination certificate is a property of the representation, not
/// the value.
///
/// The same `2^80` certifies at `floor = 1` while register-held
/// (`2^80 ≥ 3 · 2^64`) and reads `decided = false` once spilled — its
/// fold decides at digit index 2, below the required `floor + 2 = 3`.
/// Pins the representation cliff the contract documents: a caller must
/// never treat `decided` as a pure function of the value.
#[test]
fn domination_certificate_depends_on_representation() {
    let mut acc = Accumulator::new();
    acc.add_u64(1 << 20);
    acc.shl(30);
    acc.shl(30);
    assert!(
        acc.quick.is_some(),
        "2^80 built through register entry points stays register-held"
    );
    assert_eq!(
        acc.sign_dominates_at(1),
        (Ordering::Greater, true),
        "register-held: the direct comparison certifies 2^80 ≥ 3 · 2^64"
    );
    acc.spill();
    assert_eq!(
        acc.sign_dominates_at(1),
        (Ordering::Greater, false),
        "spilled: the fold decides at index 2, below floor + 2 = 3"
    );
}

/// After a sign read, `digit_count` reports the collapsed top, and a
/// domination floor derived from that count decides; derived from the
/// stale count instead, the same read refuses.
///
/// The clause callers comparing accumulators lean on: the
/// [`sign_dominates_at`](Accumulator::sign_dominates_at) rustdoc routes
/// them through `floor = its digit_count - 1`, and the fold's collapse
/// (the scanned prefix zeroed, its exact partial re-deposited at the
/// scan's floor) is what makes that count tight after a sign read.
/// Adequacy leg: with the sign read omitted, the stale spelling still
/// counts its cancelling prefix, and the floor it yields is undecidable
/// even for a comparand that dwarfs the true value.
#[test]
fn sign_collapse_tightens_the_top_and_arms_domination() {
    // The crate docs' canonical cancelling prefix: +2^320, then
    // −(2^320 − 1), value 1 spelled across eleven digits (index 10
    // down to 0; 320 = 32 · 10). Both operands exceed the register's
    // 2^96 magnitude bound, so the spelling lives in the digit engine.
    let mut cancelled = Accumulator::new();
    cancelled.add_wide(&(UBig::ONE << 320usize));
    cancelled.sub_wide(&((UBig::ONE << 320usize) - UBig::ONE));
    let stale = cancelled.digit_count();
    assert_eq!(
        stale, 11,
        "the cancelling prefix leaves the stale top at the added operand's width"
    );
    // A comparand with top digit 5 at index 4: decision-bound (5 ≥ 3,
    // the fold's decision bound) and spilled past the register.
    let mut comparand = Accumulator::new();
    comparand.add_wide(&(UBig::from(5u8) << 128usize));
    // Adequacy leg (sign read omitted): a floor derived from the stale
    // count demands clearance no honest comparand of the cancelled
    // value's true scale needs, and the read refuses. The read rewrites
    // nothing here (a decision-bound top answers on its first step), so
    // the decided read below sees the same spelling.
    assert_eq!(
        comparand.sign_dominates_at(stale - 1),
        (Ordering::Greater, false),
        "a domination floor derived from the stale count must refuse"
    );
    // The sign read collapses the spelling; the count is now tight.
    assert_eq!(cancelled.sign(), Ordering::Greater);
    assert_eq!(
        cancelled.digit_count(),
        1,
        "after the sign read, digit_count reports the collapsed top"
    );
    // The collapsed count arms the decision: floor 0 sits two or more
    // digit indexes under the comparand's top, so the read decides.
    assert_eq!(
        comparand.sign_dominates_at(cancelled.digit_count() - 1),
        (Ordering::Greater, true),
        "a domination floor derived from the collapsed count decides"
    );
}

/// An interleaved sign read can lower `sign_magnitude_shl`'s returned
/// shift: the collapse re-deposits its partial through the write path,
/// below every position the caller's own writes touched.
///
/// The [`sign_magnitude_shl`](Accumulator::sign_magnitude_shl)
/// rustdoc's sign-queries-count-as-writers clause, pinned executable: a
/// caller pricing reads by the returned shift keeps sign reads off the
/// accumulator before the scaled read, or surrenders part of the
/// never-written-prefix skip. Adequacy leg: untouched, the same value
/// returns the full written-span shift, exactly.
#[test]
fn collapsing_sign_read_lowers_the_scaled_read_shift() {
    // One unit parked at digit 40: the 1280-bit shift exceeds the
    // register's 30-bit shift bound, so the value lives in the digit
    // engine with a one-digit written span at index 40.
    let mut acc = Accumulator::new();
    acc.add_wide_shl(&UBig::ONE, 1280);
    // Sign read omitted: the scaled read prices the written span and
    // returns the whole never-written prefix as the shift.
    let (sign, magnitude, shift) = acc.sign_magnitude_shl();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(
        (magnitude, shift),
        (UBig::ONE, 1280),
        "untouched, the scaled read returns the full written-span shift"
    );
    // The collapsing read: top digit 1 sits under the decision bound
    // 3, so the fold descends one digit and re-deposits the partial
    // there; sign queries count as writers (the sign_magnitude_shl
    // rustdoc), so the write watermark drops with it.
    assert_eq!(acc.sign(), Ordering::Greater);
    let (sign, magnitude, shift) = acc.sign_magnitude_shl();
    assert_eq!(sign, Ordering::Greater);
    assert!(
        shift < 1280,
        "the collapsing sign read must lower the returned shift"
    );
    assert_eq!(
        (&magnitude, shift),
        (&(UBig::ONE << 32usize), 1248),
        "the collapse re-deposits one digit down: shift 32 · 39, magnitude 2^32"
    );
    // The pair is one honest spelling of the unchanged value.
    assert_eq!(
        magnitude << usize::try_from(shift).expect("the shift fits the address space"),
        UBig::ONE << 1280usize,
        "the collapse is value-preserving"
    );
}

/// A carry tie that recenters to a zero remainder converts exactly:
/// the read-out's complement pass must ripple through the zero low
/// digit.
///
/// With the digit engine explicitly armed, the flush-right deposit
/// `−2^32 − 2^32` lands digit 0 on the recenter boundary (remainder 0,
/// carry −2). Pins the `rem_euclid`/complement seam of `sign_magnitude`
/// on the one shape where the complement's carry crosses a zero digit —
/// the arm the negative-conversion test's operands never exercise. The
/// mode asserts keep the witness honest: the same schedule on the quick
/// register stays register-held and never reaches the seam, so the
/// spill and the exact digit state are pinned alongside the value. The
/// generalized family is `carry_tie_streams_match_the_oracle` in the
/// differential suite; this witness is its deterministic tripwire.
#[test]
fn flush_right_carry_tie_converts_exactly() {
    let mut acc = Accumulator::new();
    acc.spill();
    acc.sub_u64(1 << 32);
    acc.sub_u64(1 << 32);
    assert!(
        acc.quick.is_none(),
        "the seam under test lives in the digit engine"
    );
    assert_eq!(
        &acc.digits[..=acc.top],
        &[0, -2],
        "the recenter tie leaves remainder 0 and carry −2"
    );
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

/// `sign_limbs` at its conversion-path corners: zero reads empty in
/// both tiers, a register value spanning two limbs splits exactly at
/// the limb seam, and negatives read the magnitude's limbs.
///
/// The register readout packs an `i128` magnitude into at most two
/// limbs and strips high zeros; the digit-engine readout pairs base-2^32
/// digits into limbs. Each corner is checked in the register and again
/// after a forced spill, so both tiers pin the same spellings — and the
/// register corner at exactly `2^64` (a low limb of zero under a high
/// limb of one) pins the strip as top-down, never a sweep of interior
/// zeros.
#[test]
fn sign_limbs_conversion_corners() {
    // Zero: empty limbs, both tiers.
    let mut zero = Accumulator::new();
    assert_eq!(zero.sign_limbs(), (Ordering::Equal, vec![]));
    zero.spill();
    assert_eq!(zero.sign_limbs(), (Ordering::Equal, vec![]));

    // (value, expected LE limbs): the u64 ceiling, the limb seam at
    // 2^64 (interior zero limb kept), and a two-limb composite.
    let corners: [(u128, Vec<u64>); 3] = [
        (u128::from(u64::MAX), vec![u64::MAX]),
        (1u128 << 64, vec![0, 1]),
        ((7u128 << 64) | 5, vec![5, 7]),
    ];
    for (value, limbs) in corners {
        for negative in [false, true] {
            for spill in [false, true] {
                // Register-preserving construction: word deposits and
                // in-register shifts only (30 + 30 + 4 covers one limb),
                // so the unspilled leg genuinely reads the register tier.
                let mut acc = Accumulator::new();
                acc.add_u64((value >> 64) as u64);
                acc.shl(30);
                acc.shl(30);
                acc.shl(4);
                acc.add_u64(value as u64);
                if negative {
                    acc.negate();
                }
                if spill {
                    acc.spill();
                } else {
                    assert!(acc.quick.is_some(), "the construction stays registered");
                }
                let sign = if negative {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
                assert_eq!(
                    acc.sign_limbs(),
                    (sign, limbs.clone()),
                    "value {value}, negative {negative}, spilled {spill}"
                );
            }
        }
    }
}

/// `reserve_digits` is value-neutral in every tier.
///
/// Reserving on the register leaves it registered, reserving less than
/// the held width changes nothing, and writes after a reservation land
/// exactly as without one.
#[test]
fn reserve_digits_is_value_neutral() {
    // On the register: the reservation warms the idle buffer without
    // arming the digit engine or touching the held value.
    let mut acc = Accumulator::new();
    acc.add_small(7);
    acc.reserve_digits(100);
    assert!(acc.quick.is_some(), "a reservation never arms the engine");
    let mut oracle = IBig::from(7);
    assert_value(&acc, &oracle);

    // In the digit engine, before and after the covered writes — and a
    // reservation smaller than the held width is a no-op.
    acc.add_wide(&(UBig::from(1u8) << 3_200usize));
    oracle += IBig::from(UBig::from(1u8) << 3_200usize);
    acc.reserve_digits(500);
    assert_value(&acc, &oracle);
    acc.reserve_digits(1);
    assert_value(&acc, &oracle);
    acc.sub_wide(&(UBig::from(1u8) << 12_800usize));
    oracle -= IBig::from(UBig::from(1u8) << 12_800usize);
    assert_value(&acc, &oracle);
}
