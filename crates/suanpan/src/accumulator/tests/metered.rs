//! Exact digit-touch pins for the accumulator's documented cost bounds.
//!
//! Exact counts show both that the meter observes the work and that the count
//! stays independent of the dimensions excluded by each complexity bound.
//! `no_collapse_fold_re_scans_the_prefix` supplies a deliberately inefficient
//! control: removing the sign fold's collapse makes its per-read cost grow with
//! the cancelling prefix.

use core::cmp::Ordering;

use num_bigint::{BigInt as IBig, BigUint as UBig};

use super::{
    assert_value, cancelled_top, fresh, from_limbs, read_value, Accumulator, TestBig as _,
};
use crate::accumulator::operand::Update;
use crate::touch_meter;

/// Read a stored copy's sign, magnitude, and factored scale in bits, leaving the
/// stored form unchanged.
fn scaled_reading(acc: &Accumulator) -> (Ordering, UBig, u64) {
    let (sign, limbs, shift) = acc.stored_copy().scaled_signed_magnitude();
    (sign, from_limbs(limbs.as_ref()), shift)
}

/// A scaled read and a reset skip an untouched low prefix; an unscaled read
/// visits it.
///
/// A two-limb value at bit 32,000 takes at most sixteen scaled-read touches
/// but more than a thousand unscaled-read touches. Both reconstruct the same
/// integer. The reset then also takes at most sixteen touches, and clears the
/// value and its remembered scale.
#[test]
fn scaled_read_costs_the_written_span() {
    let mut acc = Accumulator::new();
    // Park a two-limb value at digit 1000 (bit shift 32_000).
    acc.add_limb_value_shl(&from_limbs(&[7, 9]), 32_000);
    touch_meter::reset();
    let (sign, magnitude, shift) = scaled_reading(&acc);
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
    let (_, full) = acc.sign_biguint();
    assert!(
        touch_meter::touches() > 1000,
        "the full-width control read the whole span; if this dropped, the \
         separation this pin demonstrates needs re-deriving"
    );
    assert_eq!(
        IBig::from(full),
        IBig::from(from_limbs(&[7, 9])) << 32_000usize
    );
    touch_meter::reset();
    acc.reset();
    let reset = touch_meter::touches();
    assert!(
        reset <= 16,
        "the reset cleared {reset} digits: the write watermark is not \
         skipping the never-written prefix"
    );
    assert!(acc.is_known_zero());
    let (sign, magnitude, shift) = scaled_reading(&acc);
    assert_eq!(
        (sign, magnitude, shift),
        (Ordering::Equal, UBig::ZERO, 0),
        "a reset accumulator reads zero at scale zero"
    );
}

/// Alternating shifted updates cost three touches per pair regardless of the gap.
///
/// Two deposits and one skip down the unwritten gap suffice at both tested
/// shifts. The same pin holds with a nonzero digit below the gap: recording
/// only a single lowest-written position could not justify skipping that
/// interior gap.
#[test]
fn alternating_shifted_writes_cost_the_operand_not_the_gap() {
    let one = UBig::from(1u8);
    for shift in [32_000u64, 64_000] {
        let mut acc = Accumulator::new();
        acc.add_limb_value_shl(&one, shift);
        touch_meter::reset();
        for _ in 0..1_000 {
            acc.sub_limb_value_shl(&one, shift);
            acc.add_limb_value_shl(&one, shift);
        }
        assert_eq!(
            touch_meter::touches(),
            3_000,
            "1,000 alternating one-limb pairs at shift {shift}: 3 touches per \
             pair (2 deposits + 1 gap skip), whatever the shift"
        );
        // The oscillation is value-neutral: the held value is still 2^shift.
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            UBig::from(1u8) << usize::try_from(shift).unwrap()
        );
    }
    // An array-backed one-limb stream follows the same fast path as the
    // oracle's iterator, including the single skip over the unwritten gap.
    for shift in [32_000u64, 64_000] {
        let mut acc = Accumulator::new();
        acc.add_shifted_limbs(shift, [1]);
        touch_meter::reset();
        for _ in 0..1_000 {
            acc.sub_shifted_limbs(shift, [1]);
            acc.add_shifted_limbs(shift, [1]);
        }
        assert_eq!(
            touch_meter::touches(),
            3_000,
            "1,000 alternating word pairs at shift {shift}: 3 touches per \
             pair (2 deposits + 1 gap skip), whatever the shift"
        );
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            UBig::from(1u8) << usize::try_from(shift).unwrap()
        );
    }
    // An occupied low digit leaves an interior gap that must still be skipped.
    for shift in [32_000u64, 64_000] {
        let mut acc = Accumulator::new();
        acc += 5_i64;
        acc.add_limb_value_shl(&one, shift);
        touch_meter::reset();
        for _ in 0..1_000 {
            acc.sub_limb_value_shl(&one, shift);
            acc.add_limb_value_shl(&one, shift);
        }
        assert_eq!(
            touch_meter::touches(),
            3_000,
            "the occupied digit 0 changes nothing: 3 touches per pair at \
             shift {shift}"
        );
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            (UBig::from(1u8) << usize::try_from(shift).unwrap()) + 5u8
        );
    }
}

/// Lowering the highest nonzero position counts every inspected written digit.
///
/// Clearing digits six through ten costs exactly sixteen touches: six limb
/// reads, five deposits, and five steps down to the remaining digit five.
/// Omitting the last scan from the meter would incorrectly report eleven.
#[test]
fn top_settlement_steps_are_metered() {
    // Digits 5..=10 hold 1 each; digits 0..=4 are never written.
    let six_high = from_limbs(&[0, 0, 1 << 32, (1 << 32) | 1, (1 << 32) | 1, 1]);
    // Digits 6..=10 hold 1 each.
    let five_high = from_limbs(&[0, 0, 0, (1 << 32) | 1, (1 << 32) | 1, 1]);
    let mut acc = Accumulator::new();
    acc.add_limb_value(&six_high);
    touch_meter::reset();
    acc.sub_limb_value(&five_high);
    assert_eq!(
        touch_meter::touches(),
        16,
        "6 limb reads + 5 deposits + 5 settlement steps: the settlement \
         scan must count one touch per digit it steps past"
    );
    let (sign, magnitude) = acc.sign_biguint();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(magnitude, UBig::from(1u8) << 160usize);
}

/// An exactly cancelling prefix lets the sign query skip a thousand zero digits.
///
/// The query costs six touches: three reads and three clearing writes.
/// It must also leave the representation visibly zero.
#[test]
fn sign_query_skips_unwritten_zeros() {
    let mut acc = Accumulator::new();
    acc.add_limb_value_shl(&UBig::from(1u8), 32 * 1_001);
    // Deposit −2^32 in digit 1,000: the value is now zero, spelled
    // across digits 1,000 and 1,001 above the never-written gap.
    acc.sub_value_shl(&UBig::from(1u64 << 32), 32 * 1_000);
    touch_meter::reset();
    assert_eq!(acc.sign(), Ordering::Equal);
    assert_eq!(
        touch_meter::touches(),
        6,
        "3 fold reads + 3 collapse zeroes: a zero partial crosses the \
         unwritten gap in one skip, touching none of its digits"
    );
    assert!(acc.is_known_zero(), "the collapse canonicalized zero");
}

/// The accumulator-operand rows cost the operand's held digits, not
/// the receiver's width or the shift.
///
/// Every accumulator operand is first compared with zero. The two-digit
/// operand's top digit, 1, does not decide, so the comparison reads and clears
/// both digits and rewrites the value `2^32 + 1` as one digit at position 0:
/// 5 touches. The deposit then reads that digit and writes it. Added to the
/// receiver's digit `2^32 − 1`, it reaches the digit limit `2^33` and carries
/// once: 8 touches for `+=` and owned addition. Subtracted, it leaves `−2`
/// without a carry: 7, as for `add_shifted` and `sub_shifted`, whose deposits
/// land past the receiver's digits. Each total holds at either receiver width
/// and either shift. Adding the compacted operand again costs 3: one deciding
/// touch, a read, and a deposit onto the digit the carry cleared.
///
/// These are the cost table's "amortized O(operand's held digits),
/// whatever the held width / independent of the shift" rows, pinned
/// exact at their canonical shape.
#[test]
fn accumulator_operand_rows_cost_the_operand() {
    // A two-digit operand: digits 0 and 1 hold 1 each.
    let narrow = || {
        // Arm the digit representation explicitly: this row measures digit-to-digit
        // accumulator folds, not the small-value operand path.
        let mut acc = fresh(true);
        acc.value
            .get_mut()
            .digits
            .apply_limbs(core::iter::once((1 << 32) | 1), Update::Add, 0);
        acc
    };
    // Receivers of 64 and 128 held digits: 2^k − 1 fills every digit.
    for held_bits in [2_048u32, 4_096] {
        let wide_value = (UBig::from(1u8) << held_bits as usize) - 1u8;

        let operand = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_limb_value(&wide_value);
        touch_meter::reset();
        receiver += &operand;
        assert_eq!(
            touch_meter::touches(),
            8,
            "adding 2 operand digits into {} held digits: 5 compacting the operand \
             to one digit + 1 read + 1 deposit + 1 carry",
            held_bits / 32,
        );
        touch_meter::reset();
        receiver += &operand;
        assert_eq!(
            touch_meter::touches(),
            3,
            "adding the compacted operand again: 1 deciding touch + 1 read + 1 deposit"
        );

        let operand = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_limb_value(&wide_value);
        touch_meter::reset();
        receiver -= &operand;
        assert_eq!(
            touch_meter::touches(),
            7,
            "borrowed subtraction at {held_bits} bits"
        );

        let operand = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_limb_value(&wide_value);
        touch_meter::reset();
        receiver += operand;
        assert_eq!(
            touch_meter::touches(),
            8,
            "owned addition reads the narrower operand only"
        );
        let (sign, magnitude) = receiver.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            (UBig::from(1u8) << held_bits as usize) - 1u8 + UBig::from((1u64 << 32) | 1)
        );
    }
    // Each scaled merge compacts the operand the same way, for the same total
    // at both shifts and in both signs.
    for shift in [32_000u64, 64_000] {
        let operand = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_limb_value(&((UBig::from(1u8) << 2_048usize) - 1u8));
        touch_meter::reset();
        receiver.add_shifted(shift, &operand);
        assert_eq!(
            touch_meter::touches(),
            7,
            "add_shifted of 2 digits at shift {shift}: 5 compacting the operand to one \
             digit + 1 read + 1 deposit"
        );
        assert_eq!(
            operand.stored_digit_count(),
            1,
            "the zero comparison compacts"
        );
        let (sign, magnitude) = receiver.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            (UBig::from(1u8) << 2_048usize) - 1u8
                + (UBig::from((1u64 << 32) | 1) << usize::try_from(shift).unwrap())
        );

        let operand = narrow();
        let mut receiver = Accumulator::new();
        receiver.add_limb_value(&((UBig::from(1u8) << 2_048usize) - 1u8));
        touch_meter::reset();
        receiver.sub_shifted(shift, &operand);
        assert_eq!(
            touch_meter::touches(),
            7,
            "sub_shifted of 2 digits at shift {shift}: 5 compacting the operand to one \
             digit + 1 read + 1 deposit"
        );
        // The shifted operand exceeds the receiver, so the difference is
        // negative; compute its magnitude operand-first.
        let (sign, magnitude) = receiver.sign_biguint();
        assert_eq!(sign, Ordering::Less);
        assert_eq!(
            magnitude,
            (UBig::from((1u64 << 32) | 1) << usize::try_from(shift).unwrap())
                - ((UBig::from(1u8) << 2_048usize) - 1u8)
        );
    }
}

/// owned addition on a digit-count tie reads `other` — the
/// documented tie routing ("on a tie, `other` is the one read and
/// `self`'s buffer keeps the sum"), observable in the exact counts.
///
/// Two tied-width operands with different interior-zero populations:
/// the fold pays one touch for the operand's deciding top digit, one read
/// per operand digit up to its top, and one deposit per *nonzero* operand
/// digit, so which operand is read is visible whenever their nonzero counts
/// differ. Here `other` is dense and `self` carries interior zeros — 7
/// touches at the 3-digit tie (1 + 3 reads + 3 deposits) and 11 at the
/// 5-digit tie (1 + 5 + 5); a swap on the tie would read the zero-bearing
/// buffer and undercount. The sum and the destination's reset behavior are
/// asserted alongside.
#[test]
fn merge_tie_reads_the_operand() {
    // (self limbs, other limbs, tied digit count, expected touches):
    // `self` spells its value with interior zero digits, `other` is
    // dense, and both settle the same top digit index.
    let ties: [(&[u64], &[u64], usize, u64); 2] = [
        // self digits [5, 0, 7]; other digits [1, 2, 3].
        (&[5, 7], &[(2 << 32) | 1, 3], 3, 7),
        // self digits [5, 0, 0, 0, 7]; other digits [1, 2, 3, 4, 5].
        (&[5, 0, 7], &[(2 << 32) | 1, (4 << 32) | 3, 5], 5, 11),
    ];
    for (self_limbs, other_limbs, tied_count, expected) in ties {
        let mut receiver = Accumulator::new();
        receiver.add_limb_value(&from_limbs(self_limbs));
        let mut operand = Accumulator::new();
        operand.add_limb_value(&from_limbs(other_limbs));
        assert_eq!(receiver.stored_digit_count(), tied_count, "the tie premise");
        assert_eq!(operand.stored_digit_count(), tied_count, "the tie premise");
        touch_meter::reset();
        receiver += operand;
        assert_eq!(
            touch_meter::touches(),
            expected,
            "a {tied_count}-digit tie reads the dense `other`: one deciding \
             touch, one read per operand digit, and one deposit per nonzero \
             operand digit"
        );
        let (sign, magnitude) = receiver.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(magnitude, from_limbs(self_limbs) + from_limbs(other_limbs));
        receiver.reset();
        assert!(receiver.is_known_zero(), "reset clears the destination");
    }
}

/// The per-call O(held digits) rows read exact totals at 64 and 128
/// held digits, and shift assignment's total is independent of the shift.
///
/// With `d` held digits:
/// - unary `-` touches each digit once: `d`;
/// - `reset` after `<<= 32` touches each written digit once: `d`, because
///   the shift moved the span up one position and left digit zero unwritten;
/// - `<<=` reads the top digit to rule out zero, then reads and re-deposits
///   each digit: `2d + 1`. The top digit, `2^32 - 1`, settles the zero check
///   alone, and the total is the same at a thousandfold larger shift;
/// - a magnitude read decides the sign on that same top digit, then
///   normalizes the span in one pass: `d + 1`.
///
/// Exact equality across the width doubling is the linearity claim, with no
/// slack for a hidden second pass.
#[test]
fn held_width_rows_cost_the_held_digits() {
    for bits in [2_048u32, 4_096] {
        let held_digits = u64::from(bits / 32);
        let wide_value = (UBig::from(1u8) << bits as usize) - 1u8;
        let mut acc = Accumulator::new();
        acc.add_limb_value(&wide_value);

        touch_meter::reset();
        acc = -acc;
        assert_eq!(
            touch_meter::touches(),
            held_digits,
            "negation at {held_digits} held digits: one touch per digit"
        );
        acc = -acc;

        touch_meter::reset();
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!(
            touch_meter::touches(),
            held_digits + 1,
            "sign_biguint at {held_digits} held digits: one deciding touch and \
             one normalizing pass over the span"
        );
        assert_eq!((sign, magnitude), (Ordering::Greater, wide_value.clone()));

        touch_meter::reset();
        let (limb_sign, limbs) = acc.signed_magnitude();
        assert_eq!(
            touch_meter::touches(),
            held_digits + 1,
            "signed_magnitude at {held_digits} stored digits: the same deciding \
             touch and pass as sign_biguint"
        );
        assert_eq!(limb_sign, Ordering::Greater);
        assert_eq!(from_limbs(limbs.as_ref()), wide_value.clone());

        touch_meter::reset();
        acc <<= 32;
        assert_eq!(
            touch_meter::touches(),
            2 * held_digits + 1,
            "shift assignment at {held_digits} held digits: one read of the top digit \
             to rule out zero, then one read and one re-deposit per digit"
        );
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(magnitude, wide_value.clone() << 32usize);

        touch_meter::reset();
        acc.reset();
        assert_eq!(
            touch_meter::touches(),
            held_digits,
            "reset at {held_digits} held digits: one touch per written digit \
             (the shift left digit zero unwritten)"
        );
        assert!(acc.is_known_zero());
    }
    // The same in-place scale at a thousandfold larger shift costs the
    // same touches: shifting is priced by the held digits alone (memory, not
    // digit work, covers the shifted positions).
    let wide_value = (UBig::from(1u8) << 2_048usize) - 1u8;
    let mut acc = Accumulator::new();
    acc.add_limb_value(&wide_value);
    touch_meter::reset();
    acc <<= 32_000;
    assert_eq!(
        touch_meter::touches(),
        129,
        "<<= 32_000 at 64 held digits: the same top-digit read and 2 touches \
         per held digit as <<= 32"
    );
    let (sign, magnitude) = acc.sign_biguint();
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
        acc.add_limb_value(&((UBig::from(1u8) << cliff_bits as usize) - 1u8));
        touch_meter::reset();
        for _ in 0..pairs {
            acc += u64::MAX;
            assert_eq!(acc.sign(), Ordering::Greater, "above the cliff");
            acc -= u64::MAX;
            assert_eq!(acc.sign(), Ordering::Greater, "back below the cliff");
        }
        assert_eq!(
            touch_meter::touches(),
            6 * pairs + 1,
            "u64::MAX comb at k = {cliff_bits}: 2 touches per delta and 1 \
             per sign read, whatever the cliff height and stream length"
        );
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(magnitude, (UBig::from(1u8) << cliff_bits as usize) - 1u8);
    }
}

/// Limb-stream writes cost the operand's limbs, not the held width: exact
/// equal totals into receivers of 64 and 128 held digits, both signs.
///
/// A two-limb operand costs exactly 4 touches (one read plus one
/// deposit per limb — the high halves here are zero and deposit
/// nothing) whether the receiver holds 64 or 128 digits. The companion
/// shifted-write test covers shift independence.
#[test]
fn limb_writes_cost_the_operand_at_any_held_width() {
    for bits in [2_048u32, 4_096] {
        let held = (UBig::from(1u8) << bits as usize) - 1u8;
        let mut acc = Accumulator::new();
        acc.add_limb_value(&held);
        touch_meter::reset();
        acc.add_limb_value(&from_limbs(&[3, 5]));
        assert_eq!(
            touch_meter::touches(),
            4,
            "add_limb_value of 2 limbs into {} held digits: 2 limb reads + 2 deposits",
            bits / 32
        );
        touch_meter::reset();
        acc.sub_limb_value(&from_limbs(&[3, 5]));
        assert_eq!(
            touch_meter::touches(),
            4,
            "sub_limb_value twin at {} held digits",
            bits / 32
        );
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!((sign, magnitude), (Ordering::Greater, held));
    }
}

/// Limb streams choose the word cost for one limb and charge each limb once
/// when the operand is wider, independently of the shift and held width.
///
/// A one-limb stream delegates to the word operation. On this oscillating
/// schedule, 1,000 sub/add pairs cost exactly 3,000 touches: two deposits and
/// one gap skip per pair, pinned identical across a shift doubling.
/// The second clause pins the contract's
/// padding sentence exactly: a `[5, 0, 0]` stream costs 4 touches (3
/// yielded-limb reads + 1 deposit) — high zero limbs are value-neutral
/// but each yielded limb pays its touch.
#[test]
fn limb_streams_select_word_or_wide_cost() {
    for shift in [32_000u64, 64_000] {
        let mut acc = Accumulator::new();
        acc.add_shifted_limbs(shift, [1u64]);
        touch_meter::reset();
        for _ in 0..1_000 {
            acc.sub_shifted_limbs(shift, [1u64]);
            acc.add_shifted_limbs(shift, [1u64]);
        }
        assert_eq!(
            touch_meter::touches(),
            3_000,
            "1,000 alternating one-limb stream pairs at shift {shift}: \
             3 touches per pair, whatever the shift"
        );
        let (sign, magnitude) = acc.sign_biguint();
        assert_eq!(sign, Ordering::Greater);
        assert_eq!(
            magnitude,
            UBig::from(1u8) << usize::try_from(shift).unwrap()
        );
    }
    let mut acc = Accumulator::new();
    acc.add_shifted_limbs(0, [5u64, 0, 0]);
    touch_meter::reset();
    acc.add_shifted_limbs(0, [5u64, 0, 0]);
    assert_eq!(
        touch_meter::touches(),
        4,
        "a padded [5, 0, 0] stream: 3 yielded-limb reads + 1 deposit — \
         zero limbs are value-neutral but pay their touch"
    );
    let (sign, magnitude) = acc.sign_biguint();
    assert_eq!((sign, magnitude), (Ordering::Greater, UBig::from(10u8)));
}

/// Repeated stability queries do not repeat the first query's simplification.
///
/// At 2^2048, the first query costs five touches to combine two digits and
/// deposit the result. Each of the following thousand queries costs one touch
/// and still proves stability for the requested width.
#[test]
fn domination_reads_cost_one_touch_after_the_first() {
    let mut acc = Accumulator::new();
    acc.add_limb_value(&(UBig::from(1u8) << 2_048usize));
    touch_meter::reset();
    assert_eq!(
        acc.cmp_zero_stable_under(128),
        Some(Ordering::Greater),
        "2^2048 dominates any value under 2^128"
    );
    assert_eq!(
        touch_meter::touches(),
        5,
        "the first read folds two digits, collapses, and re-deposits"
    );
    touch_meter::reset();
    for _ in 0..1_000 {
        assert_eq!(acc.cmp_zero_stable_under(64), Some(Ordering::Greater));
    }
    assert_eq!(
        touch_meter::touches(),
        1_000,
        "every read after the collapse is one top-digit touch"
    );
}

/// A sufficiently large top coefficient decides stability on its first touch.
///
/// Coefficient five at position four covers 96 bits in one touch, but cannot
/// cover 128 bits. Coefficient two must read lower positions, so its refusal
/// takes more than one touch.
#[test]
fn decision_bound_top_decides_on_the_first_touch() {
    // Top digit 5 at index 4, spilled past the register's 2^96 bound.
    let mut acc = Accumulator::new();
    acc.add_limb_value(&(UBig::from(5u8) << 128usize));
    touch_meter::reset();
    assert_eq!(acc.cmp_zero_stable_under(96), Some(Ordering::Greater));
    assert_eq!(
        touch_meter::touches(),
        1,
        "the decision-bound top decides on the first digit touch"
    );
    // Adequacy leg, one digit short: the fold still stops on its first
    // touch (the partial is decided), but the bound refuses.
    touch_meter::reset();
    assert_eq!(acc.cmp_zero_stable_under(128), None);
    assert_eq!(
        touch_meter::touches(),
        1,
        "one digit short refuses without reading further"
    );
    // Adequacy leg, below-bound top: the fold must descend past the
    // top digit, so the read cannot decide on its first touch.
    let mut low_top = Accumulator::new();
    low_top.add_limb_value(&(UBig::from(2u8) << 128usize));
    touch_meter::reset();
    assert_eq!(low_top.cmp_zero_stable_under(96), None);
    assert!(
        touch_meter::touches() > 1,
        "a below-bound top cannot decide on its first touch"
    );
}

/// A scaled read skips only the low prefix, not gaps between written positions.
///
/// Three nonzero digits at positions zero, 1000, and 1002 require exactly 1004
/// touches: one to decide the sign on the top digit, and one per position in
/// the normalizing pass. Charging only for the three writes would undercount
/// the normalized read.
#[test]
fn scaled_read_costs_the_span_not_the_write_count() {
    let mut acc = Accumulator::new();
    // Limbs [7, 9] at bit 32_000: digit 1000 holds 7, digit 1002 holds 9
    // (the odd-limb high halves are zero and deposit nothing).
    acc.add_limb_value_shl(&from_limbs(&[7, 9]), 32_000);
    acc += 5_i64;
    touch_meter::reset();
    let (sign, magnitude, shift) = scaled_reading(&acc);
    assert_eq!(
        touch_meter::touches(),
        1_004,
        "the scaled read walks the whole span between the two writes: \
         its cost is the written span, not the written-digit count"
    );
    assert_eq!((sign, shift), (Ordering::Greater, 0));
    assert_eq!(
        IBig::from(magnitude),
        (IBig::from(from_limbs(&[7, 9])) << 32_000usize) + 5
    );
}

/// A fold that does not collapse re-scans the cancelling prefix on every read,
/// making the cost grow with the prefix.
///
/// A negative control applies the production decision rule without its
/// collapse. At widths 2048 and 4096, it pays `k/32 + 1` touches on every
/// read. The production fold pays `2 * (k/32) + 3` once, then exactly one
/// touch for each of a thousand subsequent reads at both widths.
#[test]
fn no_collapse_fold_re_scans_the_prefix() {
    /// The fold's decision rule without its collapse: the digit
    /// touches a non-collapsing sign read would pay on `acc` as it
    /// stands, every time.
    fn no_collapse_read_touches(acc: &Accumulator) -> u64 {
        let mut touches = 0u64;
        let value = acc.stored();
        let digits = value.digits.stored_digits();
        let mut index = digits.len() - 1;
        let mut partial: i128 = 0;
        loop {
            touches += 1;
            partial = (partial << 32) + i128::from(digits[index]);
            if partial.abs() >= 3 || index == 0 {
                return touches;
            }
            index -= 1;
        }
    }

    for prefix_bits in [2_048u32, 4_096] {
        let prefix_digits = u64::from(prefix_bits / 32);
        let mut acc = Accumulator::new();
        acc.add_limb_value(&(UBig::from(1u8) << prefix_bits as usize));
        acc.sub_limb_value(&((UBig::from(1u8) << prefix_bits as usize) - 1u8));
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

/// A magnitude read costs the result's width, not a cancelled working width,
/// and a repeated read costs only the normalized result.
///
/// The value `2^(32r) − 1`, `r` digits of `2^32 − 1`, sits under a top that
/// cancels at position `p`. The first read compares with zero: it reads and
/// clears each of the two cancelling digits (4 touches), crosses the unwritten
/// gap below them in one untouched skip, and reads the deciding digit
/// `r − 1` and rewrites it with a clear and a deposit (3). Its normalizing pass
/// then touches the `r` digits: `r + 7` in all. The read keeps the normalized
/// form, so every repeated read costs one deciding touch and the same pass:
/// `r + 1`. Both totals are exact at two heights of the cancelled top and two
/// result widths, through both readouts. A read that kept nothing would pay
/// the first read's compaction every time.
#[test]
fn magnitude_reads_cost_the_result_width() {
    for (digits, position) in [(64u64, 1_000u64), (64, 2_000), (128, 1_000), (128, 2_000)] {
        let value = (UBig::from(1u8) << usize::try_from(32 * digits).unwrap()) - 1u8;
        for scaled in [false, true] {
            let mut acc = cancelled_top(position);
            acc.add_limb_value(&value);
            for (read, expected) in [(0, digits + 7), (1, digits + 1), (2, digits + 1)] {
                touch_meter::reset();
                let read_back = read_value(&mut acc, scaled);
                assert_eq!(
                    touch_meter::touches(),
                    expected,
                    "read {read} of {digits} digits under a top cancelled at {position} \
                     (scaled: {scaled})"
                );
                assert_eq!(read_back, IBig::from(value.clone()));
                assert_eq!(
                    acc.stored_digit_count(),
                    usize::try_from(digits).unwrap(),
                    "the read keeps the value's own working width"
                );
            }
        }
    }
}

/// A primitive conversion costs what it reads, independent of the working
/// width, and a failed conversion returns the compacted value.
///
/// `2^(32p)` has top digit 1 at position `p`. Converting it to `u64` first asks
/// whether every adjustment below `2^128` leaves its sign unchanged: the top
/// digit and the zero below it decide, and compacting them into one digit
/// costs 5 touches. The answer proves the magnitude at least `2^128`, so the
/// conversion fails without reading further. The returned value's compacted
/// top decides each later conversion in 1 touch. A value of 5 under a top that
/// cancels at position `p` converts in 9: the stability query reads and clears
/// the two cancelling digits, skips the unwritten gap, and rewrites the deciding
/// digit 5 at position 0 (7), and the normalizing read then decides on that
/// digit and passes over it (2). Both totals are exact at two heights.
#[test]
fn primitive_conversions_cost_what_they_read() {
    for position in [1_000u64, 2_000] {
        let mut wide = Accumulator::new();
        wide.add_shifted_limbs(32 * position, [1]);
        for (attempt, expected) in [(0, 5), (1, 1), (2, 1)] {
            touch_meter::reset();
            wide = u64::try_from(wide).expect_err("2^(32p) exceeds every primitive");
            assert_eq!(
                touch_meter::touches(),
                expected,
                "failed conversion {attempt} of 2^(32 * {position})"
            );
        }
        assert_value(
            &wide,
            &(IBig::from(1) << usize::try_from(32 * position).unwrap()),
        );

        let mut narrow = cancelled_top(position);
        narrow += 5_u8;
        touch_meter::reset();
        assert_eq!(u64::try_from(narrow).ok(), Some(5));
        assert_eq!(
            touch_meter::touches(),
            9,
            "converting 5 under a top cancelled at {position}: 7 for the \
             stability query + 2 for the read"
        );
    }
}

/// An accumulator operand costs its value's width, not a cancelled working
/// width, and adding it again costs only its compacted form.
///
/// The operand stores 5 under a top that cancels at position `p`. The first
/// `+=` compares it with zero in 7 touches, as in
/// `primitive_conversions_cost_what_they_read`, then reads and deposits its
/// one remaining digit: 9. Adding it again costs one deciding touch, a read,
/// and a deposit: 3. Both totals are exact at two heights of the cancelled
/// top, and the operand keeps its value.
#[test]
fn accumulator_operands_cost_their_value_width() {
    for position in [1_000u64, 2_000] {
        let mut operand = cancelled_top(position);
        operand += 5_u8;
        let mut receiver = Accumulator::new();
        receiver.add_limb_value(&((UBig::from(1u8) << 2_048usize) - 1u8));
        for (addition, expected) in [(0, 9), (1, 3), (2, 3)] {
            touch_meter::reset();
            receiver += &operand;
            assert_eq!(
                touch_meter::touches(),
                expected,
                "addition {addition} of 5 under a top cancelled at {position}"
            );
        }
        assert_value(&operand, &IBig::from(5));
        assert_value(&receiver, &((IBig::from(1) << 2_048usize) - 1 + 15));
    }
}

/// Comparing with a far narrower accumulator decides in the wider one's
/// stability query, without reading the narrower one.
///
/// `5·2^(32k)` against `2^100`, which spans four digits: the top digit 5 at
/// position `k` decides at once, one touch at any `k`, in either operand
/// order. A comparison that subtracted would read `k` digits.
#[test]
fn far_wider_comparisons_read_only_the_wider_top() {
    for digits in [1_000u64, 2_000] {
        let mut wide = Accumulator::new();
        wide.add_shifted_limbs(32 * digits, [5]);
        let narrow = Accumulator::from(1u128 << 100);
        for (left, right, expected) in [
            (&wide, &narrow, Ordering::Greater),
            (&narrow, &wide, Ordering::Less),
        ] {
            touch_meter::reset();
            assert_eq!(left.cmp(right), expected);
            assert_eq!(
                touch_meter::touches(),
                1,
                "far-wider comparison at {digits} digits"
            );
        }
    }
}

/// A comparison asks the wider side's stability query from a lead of two
/// digits, and subtracts below it or when the query refuses at that lead.
///
/// The narrow operand is `n` digits of `2^32 − 1`; the wide one is a single
/// top digit `t` at the lead above it. Leading by three digits, or by two with
/// `t = 5`, the query decides on the top digit: 1 touch. Leading by one digit,
/// the comparison subtracts into a temporary: the wide sign decides (1), its
/// `n + 1` stored digits are read and the nonzero one deposited (`n + 2`), the
/// narrow sign decides (1), its digits are read and deposited (`2n`), and the
/// difference's top decides (1): `3n + 5`. Leading by two with `t = 2`, the
/// top does not decide: compacting it into the digit below writes `2·2^32`
/// back, which carries the same 2 up again, 6 touches each time. The query
/// pays that and refuses, so the comparison subtracts, and the wide sign and
/// the difference's sign each pay it again: `6 + 6 + (n + 3) + (2n + 1) + 6`,
/// which is `3n + 22`. Every total holds in either operand order at two widths.
#[test]
fn comparison_queries_from_a_two_digit_lead() {
    for digits in [4u64, 40] {
        let mut narrow = Accumulator::new();
        narrow.add_shifted_limbs(
            0,
            core::iter::repeat_n(u64::MAX, usize::try_from(digits / 2).unwrap()),
        );
        for (lead, top, expected) in [
            (3, 5, 1),
            (2, 5, 1),
            (2, 2, 3 * digits + 22),
            (1, 5, 3 * digits + 5),
        ] {
            for reversed in [false, true] {
                let mut wide = Accumulator::new();
                wide.add_shifted_limbs(32 * (digits + lead - 1), [top]);
                assert_eq!(
                    wide.stored_digit_count(),
                    narrow.stored_digit_count() + usize::try_from(lead).unwrap()
                );
                let narrow = narrow.stored_copy();
                touch_meter::reset();
                let order = if reversed {
                    narrow.cmp(&wide).reverse()
                } else {
                    wide.cmp(&narrow)
                };
                assert_eq!(order, Ordering::Greater);
                assert_eq!(
                    touch_meter::touches(),
                    expected,
                    "a lead of {lead} digits with top {top} over {digits} digits, \
                     reversed {reversed}"
                );
            }
        }
    }
}

/// `abs_diff` of a far-wider value and a narrow one reads only the wider
/// value's top and the narrower operand, at any distance between them.
///
/// The wide value is `5·2^(32k)`; the narrow one is `n` digits of `2^32 − 1`.
/// The wider operand's storage receives the subtraction: the narrow sign
/// decides on its top digit (1), and its `n` digits are read and deposited
/// below the wide value's untouched gap (`2n`). The result's sign then decides
/// on the wide top digit (1): `2n + 2` at two distances `k`, with either
/// operand as the receiver.
#[test]
fn far_abs_diff_reads_the_wider_top_and_the_narrower_operand() {
    for k in [1_000u64, 2_000] {
        for narrow_digits in [4u64, 40] {
            for reversed in [false, true] {
                let mut wide = Accumulator::new();
                wide.add_shifted_limbs(32 * k, [5]);
                let mut narrow = Accumulator::new();
                narrow.add_shifted_limbs(
                    0,
                    core::iter::repeat_n(u64::MAX, usize::try_from(narrow_digits / 2).unwrap()),
                );
                touch_meter::reset();
                let (order, gap) = if reversed {
                    let (order, gap) = narrow.abs_diff(wide);
                    (order.reverse(), gap)
                } else {
                    wide.abs_diff(narrow)
                };
                assert_eq!(order, Ordering::Greater);
                assert_eq!(
                    touch_meter::touches(),
                    2 * narrow_digits + 2,
                    "abs_diff of 5·2^(32·{k}) and {narrow_digits} digits, reversed {reversed}"
                );
                let narrow_value =
                    (IBig::from(1) << usize::try_from(32 * narrow_digits).unwrap()) - 1;
                assert_value(
                    &gap,
                    &((IBig::from(5) << usize::try_from(32 * k).unwrap()) - narrow_value),
                );
            }
        }
    }
}

/// `abs_diff` negates its result within the result's own width: O(`n`) for
/// nonnegative operands whatever the receiver's former width, and the whole
/// width when the wider operand is negative.
///
/// The narrow operand is `n` digits of `2^32 − 1`. A receiver holding 5 under
/// a top cancelled at position `p` is wider but smaller: the subtraction
/// decides the narrow sign and reads and deposits its digits (`1 + 2n`), the
/// sign read clears the cancelling pair, skips the gap, and rewrites the
/// deciding digit `n − 1` (7), and negating the `n`-digit result costs `n`:
/// `3n + 8` at both `p`. A receiver of `−2^(32p)` is wider and smaller too, but
/// its difference keeps its width: the subtraction costs `1 + 2n`, the sign
/// read folds the top digit into the one below (5), and negation walks the
/// `p` remaining digits: `p + 2n + 6`.
#[test]
fn abs_diff_negates_within_the_result_width() {
    for position in [1_000u64, 2_000] {
        for narrow_digits in [4u64, 40] {
            let mut narrow = Accumulator::new();
            narrow.add_shifted_limbs(
                0,
                core::iter::repeat_n(u64::MAX, usize::try_from(narrow_digits / 2).unwrap()),
            );
            let narrow_value = (IBig::from(1) << usize::try_from(32 * narrow_digits).unwrap()) - 1;

            let mut small = cancelled_top(position);
            small += 5;
            touch_meter::reset();
            let (order, gap) = small.abs_diff(narrow.stored_copy());
            assert_eq!(order, Ordering::Less);
            assert_eq!(
                touch_meter::touches(),
                3 * narrow_digits + 8,
                "5 under a top cancelled at {position} against {narrow_digits} digits"
            );
            assert_value(&gap, &(&narrow_value - 5));

            let mut negative = Accumulator::new();
            negative.sub_shifted_limbs(32 * position, [1]);
            touch_meter::reset();
            let (order, gap) = negative.abs_diff(narrow.stored_copy());
            assert_eq!(order, Ordering::Less);
            assert_eq!(
                touch_meter::touches(),
                position + 2 * narrow_digits + 6,
                "-2^(32·{position}) against {narrow_digits} digits"
            );
            assert_value(
                &gap,
                &((IBig::from(1) << usize::try_from(32 * position).unwrap()) + &narrow_value),
            );
        }
    }
}
