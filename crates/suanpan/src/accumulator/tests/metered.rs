//! The touch-metered cost pins: exact digit-touch totals for the
//! claims roster's cost rows, at their canonical shapes.
//!
//! Every pin asserts an exact count, not a ceiling: exactness is the
//! liveness floor (a counter that silently stops counting cannot
//! satisfy an exact total) and the flatness witness at once — each pin
//! repeats its schedule across a doubling of the axis its row claims
//! independence from. The adequacy tripwire
//! (`no_collapse_fold_re_scans_the_prefix`) commits the known-bad
//! mechanism — the sign fold without its collapse — and demonstrates
//! the flat-per-read criterion reads red on it.

use core::cmp::Ordering;

use dashu_int::{IBig, UBig};

use super::{from_limbs, Accumulator};
use crate::touch_meter;

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
#[test]
fn scaled_read_costs_the_written_span() {
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
#[test]
fn alternating_shifted_writes_cost_the_operand_not_the_gap() {
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
#[test]
fn top_settlement_steps_are_metered() {
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
#[test]
fn sign_fold_skips_certified_runs() {
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
#[test]
fn accumulator_operand_rows_cost_the_operand() {
    // A two-digit operand: digits 0 and 1 hold 1 each.
    let narrow = || {
        let mut acc = Accumulator::new();
        acc.add_wide(&from_limbs(&[(1 << 32) | 1]));
        acc
    };
    // Receivers of 64 and 128 held digits: 2^k − 1 fills every digit.
    for held_bits in [2_048u32, 4_096] {
        let wide_value = (UBig::from(1u8) << held_bits as usize) - 1u8;
        let operand = narrow();

        let mut receiver = Accumulator::new();
        receiver.add_wide(&wide_value);
        touch_meter::reset();
        receiver.add_accum(&operand);
        assert_eq!(
            touch_meter::touches(),
            4,
            "add_accum of 2 digits into {} held digits: 2 reads + 2 deposits",
            held_bits / 32,
        );

        let mut receiver = Accumulator::new();
        receiver.add_wide(&wide_value);
        touch_meter::reset();
        receiver.sub_accum(&operand);
        assert_eq!(
            touch_meter::touches(),
            4,
            "sub_accum twin at {held_bits} bits"
        );

        let mut receiver = Accumulator::new();
        receiver.add_wide(&wide_value);
        let mut spare = operand;
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
            (UBig::from(1u8) << held_bits as usize) - 1u8 + UBig::from((1u64 << 32) | 1)
        );
    }
    // The scaled merges are shift-independent at the same exact total,
    // in both signs.
    for shift in [32_000u64, 64_000] {
        let operand = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_wide(&((UBig::from(1u8) << 2_048usize) - 1u8));
        touch_meter::reset();
        receiver.add_accum_shl(&operand, shift);
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

        let operand = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_wide(&((UBig::from(1u8) << 2_048usize) - 1u8));
        touch_meter::reset();
        receiver.sub_accum_shl(&operand, shift);
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
#[test]
fn held_width_rows_cost_the_held_digits() {
    for bits in [2_048u32, 4_096] {
        let held_digits = u64::from(bits / 32);
        let wide_value = (UBig::from(1u8) << bits as usize) - 1u8;
        let mut acc = Accumulator::new();
        acc.add_wide(&wide_value);

        touch_meter::reset();
        acc.negate();
        assert_eq!(
            touch_meter::touches(),
            held_digits,
            "negate at {held_digits} held digits: one touch per digit"
        );
        acc.negate();

        touch_meter::reset();
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(
            touch_meter::touches(),
            held_digits,
            "sign_magnitude at {held_digits} held digits: one carry pass \
             over the span"
        );
        assert_eq!((sign, magnitude), (Ordering::Greater, wide_value.clone()));

        touch_meter::reset();
        acc.shl(32);
        assert_eq!(
            touch_meter::touches(),
            2 * held_digits,
            "shl at {held_digits} held digits: one read and one re-deposit \
             per digit"
        );
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(magnitude, wide_value.clone() << 32usize);

        touch_meter::reset();
        acc.reset();
        assert_eq!(
            touch_meter::touches(),
            held_digits + 1,
            "reset at {held_digits} held digits: one touch per digit (the \
             shift grew the span by one)"
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
#[test]
fn u64_comb_touches_are_flat_and_exact() {
    for (cliff_bits, pairs) in [(4_096u32, 50_000u64), (8_192, 100_000)] {
        let mut acc = Accumulator::new();
        acc.add_wide(&((UBig::from(1u8) << cliff_bits as usize) - 1u8));
        touch_meter::reset();
        for _ in 0..pairs {
            acc.add_u64(u64::MAX);
            assert_eq!(acc.sign(), Ordering::Greater, "above the cliff");
            acc.sub_u64(u64::MAX);
            assert_eq!(acc.sign(), Ordering::Greater, "back below the cliff");
        }
        assert_eq!(
            touch_meter::touches(),
            6 * pairs + 1,
            "u64::MAX comb at k = {cliff_bits}: 2 touches per delta and 1 \
             per sign read, whatever the cliff height and stream length"
        );
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(magnitude, (UBig::from(1u8) << cliff_bits as usize) - 1u8);
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
#[test]
fn wide_writes_cost_the_operand_at_any_held_width() {
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
#[test]
fn magnitude_dispatch_costs_its_width_path() {
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

/// Domination certificates are amortized O(1): the first read may
/// collapse, every later read is a single touch, and the certificate
/// stays decided.
///
/// A value parked at digit 64 answers `sign_dominates_at(3)` in 5
/// touches (a two-step fold, its collapse, and the re-deposit), then
/// exactly one touch per read for a thousand reads — a wide running
/// total is never re-folded across its width by cheap comparisons.
#[test]
fn domination_reads_cost_one_touch_after_the_first() {
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
#[test]
fn scaled_read_costs_the_span_not_the_write_count() {
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
#[test]
fn no_collapse_fold_re_scans_the_prefix() {
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

    for prefix_bits in [2_048u32, 4_096] {
        let prefix_digits = u64::from(prefix_bits / 32);
        let mut acc = Accumulator::new();
        acc.add_wide(&(UBig::from(1u8) << prefix_bits as usize));
        acc.sub_wide(&((UBig::from(1u8) << prefix_bits as usize) - 1u8));
        assert_eq!(
            no_collapse_read_touches(&acc),
            prefix_digits + 1,
            "the no-collapse fold walks the whole prefix, and would again \
             on every read"
        );
        touch_meter::reset();
        assert_eq!(acc.sign(), Ordering::Greater);
        assert_eq!(
            touch_meter::touches(),
            2 * prefix_digits + 3,
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
