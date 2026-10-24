//! Arithmetic preserves the stored-form invariants, a shift leaves its digits
//! sign-led, and shifting zero takes constant space whatever its stored form.
//!
//! At every step, compare the value with a big-integer oracle and inspect the
//! representation itself. A nonzero digit missing from the written set would
//! be skipped by a later query even while the current value still reads
//! correctly.
//!
//! A shift streams the value's magnitude into new digits, normalizing it as
//! it goes, so that its digits start the crate's cost argument for updates of
//! one sign in the state that argument needs. Normalization in place, the
//! streamed magnitude, and the shift each get a property of their own, and
//! another follows the operations of a sum that rescales its total: updates
//! of one sign interleaved with shifts, from creation.
//!
//! Zero can be stored as nonzero digits that cancel. Shifting such a zero, or
//! adding it at a shift, must take constant space, as a known zero does.

use core::cmp::Ordering;

use num_bigint::{BigInt as IBig, BigUint as UBig};
use proptest::prelude::*;

use super::{assert_value, fresh, from_limbs, oracle_sign, Accumulator, TestBig as _};

/// Fixed operands that create carries, cancellations, and separated writes.
struct Operands {
    /// `2^32`, used to deposit a whole signed coefficient in one digit.
    ///
    /// The word path does not split the coefficient across two digits. That
    /// distinction constructs an adjacent representation such as `(+1, −2^32)`,
    /// which can drive a sign fold's partial to zero above a gap of unwritten
    /// zeros.
    word32: UBig,
    /// A unit contribution at digit three.
    p96: IBig,
    /// A unit contribution at digit seven.
    p224: IBig,
    /// A word-sized contribution large enough to carry across digits.
    max64: IBig,
}

/// Number of update, negation, and sign-query operations explored at each
/// step.
const REPRESENTATION_OPS: u8 = 12;

/// Apply the finite operation alphabet using its precomputed operands.
impl Operands {
    /// Apply one operation to the accumulator and oracle in lockstep.
    ///
    /// Unit and maximum-word updates exercise ordinary deposits and carries.
    /// Jumps to digits three and seven create, clear, and split zero runs.
    /// Depositing a whole coefficient of ±2^32 at digit six makes the sign
    /// fold reach a run with either a small nonzero or exactly zero partial.
    /// Negation sets or clears the digits' flag, so every later update and
    /// comparison also runs against flagged digits.
    fn apply(&self, acc: &mut Accumulator, oracle: &mut IBig, op: u8) {
        match op {
            0 => {
                *acc += 1_i64;
                *oracle += 1;
            }
            1 => {
                *acc -= 1_i64;
                *oracle -= 1;
            }
            2 => {
                *acc += u64::MAX;
                *oracle += &self.max64;
            }
            3 => {
                *acc -= u64::MAX;
                *oracle -= &self.max64;
            }
            4 => {
                acc.add_limb_value_shl(&UBig::ONE, 96);
                *oracle += &self.p96;
            }
            5 => {
                acc.sub_limb_value_shl(&UBig::ONE, 96);
                *oracle -= &self.p96;
            }
            6 => {
                acc.add_limb_value_shl(&UBig::ONE, 224);
                *oracle += &self.p224;
            }
            7 => {
                acc.sub_limb_value_shl(&UBig::ONE, 224);
                *oracle -= &self.p224;
            }
            8 => {
                acc.sub_value_shl(&self.word32, 192);
                *oracle -= &self.p224;
            }
            9 => {
                acc.add_value_shl(&self.word32, 192);
                *oracle += &self.p224;
            }
            10 => {
                *acc = -core::mem::take(acc);
                *oracle = -core::mem::take(oracle);
            }
            _ => {
                assert_eq!(acc.sign(), oracle_sign(oracle), "comparison with zero");
            }
        }
    }
}

/// Depth of the exhaustive representation sweep.
///
/// Every schedule of at most this many alphabet ops runs, with every
/// invariant checked at every step of every schedule (the search is a
/// prefix tree, so each state is reached and checked exactly once).
const REPRESENTATION_DEPTH: usize = 6;

/// Check every child schedule, then recurse while retaining its operation trace.
///
/// Recurses `depth` levels, at most [`REPRESENTATION_DEPTH`].
fn check_schedules(
    operands: &Operands,
    acc: &Accumulator,
    oracle: &IBig,
    schedule: &mut Vec<u8>,
    depth: usize,
) {
    for op in 0..REPRESENTATION_OPS {
        schedule.push(op);
        let mut next_acc = acc.stored_copy();
        let mut next_oracle = oracle.clone();
        operands.apply(&mut next_acc, &mut next_oracle, op);
        next_acc.stored().assert_invariants(schedule);
        assert_value(&next_acc, &next_oracle);
        if depth > 1 {
            check_schedules(operands, &next_acc, &next_oracle, schedule, depth - 1);
        }
        schedule.pop();
    }
}

/// Every schedule of up to six operations preserves the digit and written-set invariants.
///
/// The operation alphabet exercises gap creation, splitting, and consumption,
/// including carries and sign collapse inside a gap of unwritten zeros. Every
/// state must match the oracle, write every nonzero digit, write nothing
/// above the top, and write nothing below the lowest written position but
/// position zero.
#[test]
fn representation_invariants_hold_exhaustively() {
    let operands = Operands {
        word32: UBig::from(1u64 << 32),
        p96: IBig::from(UBig::ONE << 96usize),
        p224: IBig::from(UBig::ONE << 224usize),
        max64: IBig::from(u64::MAX),
    };
    let acc = fresh(true);
    let oracle = IBig::from(0);
    let mut schedule = Vec::with_capacity(REPRESENTATION_DEPTH);
    check_schedules(
        &operands,
        &acc,
        &oracle,
        &mut schedule,
        REPRESENTATION_DEPTH,
    );
}

/// Draw a run-forming stream: 1 to 149 steps, each an arm, one or two limbs,
/// and a shift below 4,096 bits.
///
/// [`apply_stream_step`] gives the arms their meaning: limb updates of either
/// sign, a whole-word subtraction, a primitive update, an accumulator operand
/// whose stored digits have mixed signs, negated or not, added or subtracted
/// at the step's shift, a negation, and a comparison with zero. Together they
/// leave runs, gaps, cancelled tops, digits of both signs, and either state
/// of the digits' flag, the varied stored forms that the normalization and
/// shift properties below start from.
pub(super) fn run_forming_stream() -> impl Strategy<Value = Vec<(u8, Vec<u64>, u64)>> {
    proptest::collection::vec(
        (
            0u8..7,
            proptest::collection::vec(any::<u64>(), 1..=2),
            0u64..4_096,
        ),
        1..150,
    )
}

/// Apply one step of a [`run_forming_stream`] to the accumulator and the
/// oracle in lockstep; the comparison arm checks the sign against the oracle.
pub(super) fn apply_stream_step(
    acc: &mut Accumulator,
    oracle: &mut IBig,
    (arm, limbs, shift): &(u8, Vec<u64>, u64),
    step: usize,
) -> Result<(), TestCaseError> {
    let value = from_limbs(limbs);
    let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();
    match arm {
        0 => {
            acc.add_limb_value_shl(&value, *shift);
            *oracle += scaled;
        }
        1 => {
            acc.sub_limb_value_shl(&value, *shift);
            *oracle -= scaled;
        }
        2 => {
            acc.sub_value_shl(&value, *shift);
            *oracle -= scaled;
        }
        3 => {
            let delta = limbs[0] as i64;
            *acc += delta;
            *oracle += delta;
        }
        4 => {
            // Positive digits from position 2 up over negative ones below:
            // the operand's stored digits have mixed signs, and most shifts
            // land them at a nonzero bit offset.
            // Negating it on some steps sets its flag, so the deposit reads
            // digits whose sum is the operand's negation.
            let mut operand = Accumulator::new();
            operand.add_shifted_limbs(64, limbs.iter().copied());
            operand.sub_shifted_limbs(0, [limbs[0]]);
            let mut operand_value = (IBig::from(value.clone()) << 64_usize) - limbs[0];
            if limbs[0] % 4 >= 2 {
                operand = -operand;
                operand_value = -operand_value;
            }
            let scaled_operand = operand_value << usize::try_from(*shift).unwrap();
            if limbs[0] % 2 == 0 {
                acc.add_shifted(*shift, &operand);
                *oracle += scaled_operand;
            } else {
                acc.sub_shifted(*shift, &operand);
                *oracle -= scaled_operand;
            }
        }
        5 => {
            *acc = -core::mem::take(acc);
            *oracle = -core::mem::take(oracle);
        }
        _ => {
            prop_assert_eq!(acc.sign(), oracle_sign(oracle), "sign at step {}", step);
        }
    }
    Ok(())
}

/// A spelling of a left shift by a count.
#[derive(Clone, Copy, Debug)]
enum ShiftSpelling {
    /// `acc <<= count`.
    Assign,
    /// `acc <<= &count`.
    AssignRef,
    /// `acc << count`, consuming `acc`.
    Owned,
    /// `&acc << count`, leaving `acc` with its owner.
    Borrowed,
}

/// Every spelling of a left shift, in the order [`shift_spelling`] draws them.
const SHIFT_SPELLINGS: [ShiftSpelling; 4] = [
    ShiftSpelling::Assign,
    ShiftSpelling::AssignRef,
    ShiftSpelling::Owned,
    ShiftSpelling::Borrowed,
];

/// Draw any spelling of a left shift.
fn shift_spelling() -> impl Strategy<Value = ShiftSpelling> {
    proptest::sample::select(&SHIFT_SPELLINGS[..])
}

/// Shift `acc` by `count` in `spelling`, returning the result and, for the
/// borrowed spelling, the accumulator it read, which still holds the
/// unshifted value.
fn shift_in(
    acc: Accumulator,
    count: u64,
    spelling: ShiftSpelling,
) -> (Accumulator, Option<Accumulator>) {
    match spelling {
        ShiftSpelling::Assign => {
            let mut acc = acc;
            acc <<= count;
            (acc, None)
        }
        ShiftSpelling::AssignRef => {
            let mut acc = acc;
            acc <<= &count;
            (acc, None)
        }
        ShiftSpelling::Owned => (acc << count, None),
        ShiftSpelling::Borrowed => (&acc << count, Some(acc)),
    }
}

/// Half the digit base, `B / 2`: the farthest a sign-led digit may lie on the
/// side of zero opposite the value's sign.
const HALF_BASE: i64 = 1 << 31;

/// The digit base, `B = 2^32`, which bounds every normalized digit's magnitude.
const BASE: i64 = 1 << 32;

/// The factor that makes a digit of the value's sign nonnegative: -1 for a
/// negative value, 1 otherwise.
fn polarity(sign: Ordering) -> i64 {
    if sign == Ordering::Less {
        -1
    } else {
        1
    }
}

/// Assert that a value of sign `sign` held in digits is normalized: every
/// stored digit has the stored sum's sign or is zero, and has magnitude below
/// the base.
///
/// The stored sum is the value, or its negation when the digits' flag is set,
/// which normalization neither reads nor changes.
fn assert_normalized(acc: &Accumulator, sign: Ordering, context: &dyn core::fmt::Debug) {
    let stored = acc.stored();
    if stored.small.is_some() {
        return;
    }
    let sign = stored.digits.stored_sign(sign);
    let polarity = polarity(sign);
    for (position, &digit) in stored.digits.stored_digits().iter().enumerate() {
        assert!(
            polarity * digit >= 0 && digit.abs() < BASE,
            "normalized digit {position} = {digit} lacks the stored sum's sign {sign:?} or \
             reaches the base after {context:?}"
        );
    }
}

/// Assert that the stored digits are *sign-led* for a nonzero value of
/// `sign`: the top digit has the stored sum's sign, and no digit has the
/// other sign with magnitude above `B / 2`.
///
/// The stored sum is the value, or its negation when the digits' flag is set.
///
/// The crate's cost argument for updates of one sign starts from this state
/// and keeps it. In it, the partial a comparison with zero folds reaches at
/// least `B / 2` by the position below the top, so the comparison's scan
/// decides within two positions and never crosses a gap. When it stops one
/// position below the top, its truncation can still cross one gap, once:
/// the top digit was small and the position below it was never written.
/// A value held in the small representation, or zero, has no digits to
/// check.
fn assert_sign_led(acc: &Accumulator, sign: Ordering, context: &dyn core::fmt::Debug) {
    let stored = acc.stored();
    if stored.small.is_some() || sign == Ordering::Equal {
        return;
    }
    let sign = stored.digits.stored_sign(sign);
    let polarity = polarity(sign);
    let digits = stored.digits.stored_digits();
    let top = *digits.last().expect("the stored prefix holds digit zero");
    assert!(
        polarity * top > 0,
        "top digit {top} lacks the stored sum's sign {sign:?} after {context:?}"
    );
    for (position, &digit) in digits.iter().enumerate() {
        assert!(
            polarity * digit >= -HALF_BASE,
            "digit {position} = {digit} opposes the stored sum's sign {sign:?} by more than \
             half the base after {context:?}"
        );
    }
}

proptest! {
    /// Longer streams with wider shifts preserve the value, signed-digit bounds,
    /// stored width, and the written-set invariant after every step.
    ///
    /// The exhaustive sweep's long-schedule, deep-shift complement:
    /// shifts to 4,096 bits, schedules to 150 ops, comparisons with zero
    /// interleaved throughout.
    #[test]
    fn representation_invariants_hold_on_run_forming_streams(
        ops in run_forming_stream(),
        digits_first: bool,
    ) {
        let mut acc = fresh(digits_first);
        let mut oracle = IBig::from(0);
        let mut schedule: Vec<u8> = Vec::new();
        for (step, op) in ops.iter().enumerate() {
            schedule.push(op.0);
            apply_stream_step(&mut acc, &mut oracle, op, step)?;
            acc.stored().assert_invariants(&schedule);
            if step % 32 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }

    /// Clearing after any stream zeroes every retained digit, clears the
    /// digits' flag, keeps the buffer, and leaves the written set holding
    /// position zero alone.
    ///
    /// A second stream after the clear stays exact, with every invariant
    /// after every step.
    ///
    /// The clear zeroes only the positions the written set holds, which
    /// suffices because the set holds every nonzero digit. The first stream
    /// leaves runs, gaps, cancelled tops, and bitsets for it to clear.
    #[test]
    fn clearing_any_stream_zeroes_every_digit_and_keeps_the_buffer(
        first in run_forming_stream(),
        second in run_forming_stream(),
    ) {
        let mut acc = fresh(true);
        let mut oracle = IBig::from(0);
        for (step, op) in first.iter().enumerate() {
            apply_stream_step(&mut acc, &mut oracle, op, step)?;
        }
        let retained = acc.stored().digits.retained_len();
        acc.clear();
        // The cleared value is small, so this checks that every retained
        // digit is zero, that the flag is clear, and that the written set
        // holds nothing but zero.
        acc.stored().assert_invariants(&[]);
        prop_assert!(acc.is_known_zero(), "a clear leaves zero");
        prop_assert_eq!(
            acc.stored().digits.retained_len(),
            retained,
            "a clear keeps the buffer"
        );
        let mut oracle = IBig::from(0);
        let mut schedule: Vec<u8> = Vec::new();
        for (step, op) in second.iter().enumerate() {
            schedule.push(op.0);
            apply_stream_step(&mut acc, &mut oracle, op, step)?;
            acc.stored().assert_invariants(&schedule);
        }
        assert_value(&acc, &oracle);
    }

    /// Normalizing digits in place keeps the value and gives every stored
    /// digit the stored sum's sign, with magnitude below the base, from any
    /// stored form a stream reaches, flagged or not.
    ///
    /// A magnitude read normalizes the digits it reads this way, and keeps
    /// the normalized form. The normalization changes the representation
    /// only: the value, the returned sign, and every digit invariant must hold
    /// after it.
    #[test]
    fn normalizing_keeps_the_value_and_gives_every_digit_its_sign(
        ops in run_forming_stream(),
    ) {
        let mut acc = fresh(true);
        let mut oracle = IBig::from(0);
        for (step, op) in ops.iter().enumerate() {
            apply_stream_step(&mut acc, &mut oracle, op, step)?;
        }
        let sign = acc.value.get_mut().digits.normalize();
        prop_assert_eq!(sign, oracle_sign(&oracle), "normalize returns the value's sign");
        acc.stored().assert_invariants(&[]);
        assert_normalized(&acc, sign, &"normalize");
        assert_value(&acc, &oracle);
    }

    /// Streaming a nonzero value's stored digits through the Euclidean carry
    /// yields its magnitude's base-`2^32` digits, from the lowest, and leaves
    /// the stored digits as they are.
    ///
    /// Only zeros follow the magnitude's top, from any stored form a stream
    /// reaches after a comparison with zero.
    ///
    /// An accumulator operand at a shift that is not a multiple of 32 bits,
    /// and a shifted value at any shift, is compared with zero and then
    /// streamed this way, so that every contribution it deposits has the
    /// value's sign.
    #[test]
    fn streaming_stored_digits_yields_the_magnitude_digits(
        ops in run_forming_stream(),
    ) {
        let mut acc = fresh(true);
        let mut oracle = IBig::from(0);
        for (step, op) in ops.iter().enumerate() {
            apply_stream_step(&mut acc, &mut oracle, op, step)?;
        }
        let sign = acc.value.get_mut().digits.cmp_zero();
        prop_assert_eq!(sign, oracle_sign(&oracle));
        if sign != Ordering::Equal {
            let stored = acc.stored().digits.stored_digits().to_vec();
            let streamed = acc.stored().digits.streamed_magnitude_digits(sign);
            let magnitude = oracle.magnitude().to_u32_digits();
            prop_assert!(streamed.len() >= magnitude.len());
            prop_assert_eq!(&streamed[..magnitude.len()], &magnitude[..]);
            prop_assert!(
                streamed[magnitude.len()..].iter().all(|&digit| digit == 0),
                "only zeros follow the magnitude's top: {:?}",
                streamed
            );
            let after = acc.stored();
            prop_assert_eq!(after.digits.stored_digits(), &stored[..]);
        }
    }

    /// Adding or subtracting any accumulator at a shift that is not a
    /// multiple of 32 bits into a new accumulator keeps both values and
    /// leaves the receiver's digits sign-led.
    ///
    /// The operand's magnitude digits enter one at a time with the deposit's
    /// sign, into zeroed storage, so recentering is the only source of a
    /// digit of the other sign, and it leaves that digit within `B / 2` of
    /// zero. The operand's stored digits come from a stream, so they mix
    /// signs.
    #[test]
    fn an_operand_at_a_bit_offset_deposits_one_sign(
        ops in run_forming_stream(),
        digit_shift in 0u64..=128,
        bit_offset in 1u64..32,
        subtract: bool,
    ) {
        let mut operand = fresh(true);
        let mut oracle = IBig::from(0);
        for (step, op) in ops.iter().enumerate() {
            apply_stream_step(&mut operand, &mut oracle, op, step)?;
        }
        let shift = 32 * digit_shift + bit_offset;
        let mut receiver = fresh(true);
        let mut expected = &oracle << usize::try_from(shift).unwrap();
        if subtract {
            receiver.sub_shifted(shift, &operand);
            expected = -expected;
        } else {
            receiver.add_shifted(shift, &operand);
        }
        receiver.stored().assert_invariants(&[]);
        assert_sign_led(&receiver, oracle_sign(&expected), &(shift, subtract));
        assert_value(&receiver, &expected);
        assert_value(&operand, &oracle);
    }

    /// Every spelling of a nonzero left shift keeps the value and leaves the
    /// shifted digits sign-led, from any stored form a stream reaches; a
    /// borrowed `<<` writes nothing to its source beyond a comparison with
    /// zero.
    ///
    /// Every spelling streams the value's magnitude, one piece of the value's
    /// sign at a time, into new digits, so recentering is the only source of
    /// a digit of the other sign, and it leaves that digit within `B / 2` of
    /// zero. Depositing the stored signed digits as they stand would break
    /// this at every bit offset. The borrowed spelling compares its source
    /// with zero, as any read does, and must leave exactly the stored digits
    /// that comparison alone leaves.
    #[test]
    fn shifting_leaves_the_digits_sign_led(
        ops in run_forming_stream(),
        shift in 1u64..=4_096,
        spelling in shift_spelling(),
    ) {
        let mut acc = fresh(true);
        let mut oracle = IBig::from(0);
        for (step, op) in ops.iter().enumerate() {
            apply_stream_step(&mut acc, &mut oracle, op, step)?;
        }
        let compared = acc.stored_copy();
        compared.sign();
        let (shifted, source) = shift_in(acc, shift, spelling);
        let sign = oracle_sign(&oracle);
        if let Some(source) = source {
            source.stored().assert_invariants(&[]);
            let (left, expected) = (source.stored(), compared.stored());
            prop_assert_eq!(
                left.digits.stored_digits(),
                expected.digits.stored_digits(),
                "a borrowed shift by {} leaves its source as a comparison with zero does",
                shift
            );
            assert_value(&source, &oracle);
        }
        let shifted_oracle = oracle << usize::try_from(shift).unwrap();
        shifted.stored().assert_invariants(&[]);
        assert_sign_led(&shifted, sign, &(spelling, shift));
        assert_value(&shifted, &shifted_oracle);
    }

    /// A nonzero shift of a zero whose digits cancel keeps its storage and
    /// its set flag, and updates of one sign from there keep the digits
    /// sign-led in the stored sum's sign, exact, and no narrower.
    ///
    /// The comparison with zero that the shift begins with compacts the
    /// cancelling digits to zero, so the storage begins with zero digits and
    /// the flag set. Every contribution the stretch stores then has the sign
    /// opposite to the update's, which the crate's cost argument for updates
    /// of one sign allows: the flag cannot change within a stretch.
    #[test]
    fn a_flagged_storage_after_a_shift_stays_sign_led(
        shift in 1u32..=4_096,
        steps in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3), 0u64..2_048),
            1..40,
        ),
        negative: bool,
    ) {
        // `2^32` at digit one, negated, then `2^32` added at digit zero: the
        // stored digits `[−2^32, 1]` cancel, under a set flag.
        let mut acc = Accumulator::new();
        acc.add_shifted_limbs(32, [1]);
        acc = -acc;
        acc += 1_u64 << 32;
        prop_assert!(!acc.is_known_zero(), "the digits cancel");
        acc <<= shift;
        prop_assert!(acc.stored().digits.is_negated(), "the shift keeps the flag");
        prop_assert!(acc.is_known_zero(), "the shift's comparison zeroed the digits");
        let mut oracle = IBig::from(0);
        let mut schedule: Vec<u8> = Vec::new();
        for (limb_update, limbs, at) in &steps {
            schedule.push(u8::from(*limb_update));
            let width_before = acc.stored_digit_count();
            if *limb_update {
                let scaled = IBig::from(from_limbs(limbs)) << usize::try_from(*at).unwrap();
                if negative {
                    acc.sub_shifted_limbs(*at, limbs.iter().copied());
                    oracle -= scaled;
                } else {
                    acc.add_shifted_limbs(*at, limbs.iter().copied());
                    oracle += scaled;
                }
            } else if negative {
                acc -= limbs[0];
                oracle -= limbs[0];
            } else {
                acc += limbs[0];
                oracle += limbs[0];
            }
            acc.stored().assert_invariants(&schedule);
            assert_sign_led(&acc, oracle_sign(&oracle), &schedule);
            prop_assert!(
                acc.stored_digit_count() >= width_before,
                "the working width fell after {:?}",
                schedule
            );
        }
        prop_assert!(acc.stored().digits.is_negated(), "the stretch keeps the flag");
        assert_value(&acc, &oracle);
    }

    /// From creation, updates of one sign interleaved with left shifts in
    /// every spelling keep the digits sign-led after every step, and no step
    /// lowers the working width.
    ///
    /// A sum of nonnegative terms that shifts its total whenever a term needs
    /// a finer scale, as `before`'s `Rank` sum does, makes exactly these
    /// operations. The crate's cost argument for updates of one sign rests on
    /// this property: an
    /// update of a sign-led value's own sign never lowers its top, and each
    /// shift starts the next stretch sign-led. Each step is a limb update, a
    /// primitive update, or a shift in a spelling its first limb selects, and
    /// every update has the one sign the case draws.
    #[test]
    fn one_sign_updates_through_shifts_stay_sign_led(
        steps in proptest::collection::vec(
            (0u8..3, proptest::collection::vec(any::<u64>(), 1..=3), 0u64..2_048),
            1..60,
        ),
        negative: bool,
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        let mut schedule: Vec<u8> = Vec::new();
        for (arm, limbs, shift) in &steps {
            schedule.push(*arm);
            let width_before = acc.stored_digit_count();
            match arm {
                0 => {
                    let scaled =
                        IBig::from(from_limbs(limbs)) << usize::try_from(*shift).unwrap();
                    if negative {
                        acc.sub_shifted_limbs(*shift, limbs.iter().copied());
                        oracle -= scaled;
                    } else {
                        acc.add_shifted_limbs(*shift, limbs.iter().copied());
                        oracle += scaled;
                    }
                }
                1 => {
                    if negative {
                        acc -= limbs[0];
                        oracle -= limbs[0];
                    } else {
                        acc += limbs[0];
                        oracle += limbs[0];
                    }
                }
                _ => {
                    let spelling = SHIFT_SPELLINGS[(limbs[0] % 4) as usize];
                    acc = shift_in(acc, *shift, spelling).0;
                    oracle <<= usize::try_from(*shift).unwrap();
                }
            }
            acc.stored().assert_invariants(&schedule);
            assert_sign_led(&acc, oracle_sign(&oracle), &schedule);
            prop_assert!(
                acc.stored_digit_count() >= width_before,
                "the working width fell from {} to {} digits after {:?}",
                width_before,
                acc.stored_digit_count(),
                schedule
            );
        }
        assert_value(&acc, &oracle);
    }
}

/// The widest shift drawn, in bits: `2^20` digit positions.
///
/// A zero whose space grew with the shift would retain about a million
/// positions here, and a passing case still runs in milliseconds.
const WIDEST_ZERO_SHIFT: u64 = 32 << 20;

/// A cancellation applied after the base construction, varying the stored form.
#[derive(Clone, Debug)]
enum Cancellation {
    /// Keep the base construction's two cancelling digits alone.
    None,
    /// Add a word as a two-limb stream, then subtract it as a primitive.
    ///
    /// The stream deposits two 32-bit halves while the primitive deposits one
    /// whole contribution, so their carries can leave further cancelling digits.
    Word(u64),
    /// Add limbs at a shift, then subtract them as an accumulator operand.
    ///
    /// A one-limb operand is held in the small representation and deposits
    /// 32-bit pieces, while the one-limb stream deposits a whole word. Wider
    /// operands extend the buffer and its written set.
    Operand { limbs: Vec<u64>, shift: u64 },
}

/// Draw a cancellation: none, a word, or up to three limbs at a shift.
fn cancellation() -> impl Strategy<Value = Cancellation> {
    prop_oneof![
        Just(Cancellation::None),
        any::<u64>().prop_map(Cancellation::Word),
        (proptest::collection::vec(any::<u64>(), 1..=3), 0u64..=256)
            .prop_map(|(limbs, shift)| Cancellation::Operand { limbs, shift }),
    ]
}

/// Build a zero stored as the digits `1` at position `k` and `-2^32` at
/// position `k - 1`, then apply `cancellation`.
///
/// Neither deposit carries, so the result is never a known zero. The
/// cancellation changes the stored form, not the value.
fn redundant_zero(k: u64, cancellation: &Cancellation) -> Accumulator {
    let mut zero = Accumulator::new();
    // Two limbs select the digit representation even though the high limb is zero.
    zero.add_shifted_limbs(32 * k, [1, 0]);
    zero.sub_shifted_limbs(32 * (k - 1), [1 << 32]);
    assert!(
        !zero.is_known_zero(),
        "the cancelling digits stay stored until a comparison compacts them"
    );
    match cancellation {
        Cancellation::None => {}
        Cancellation::Word(word) => {
            zero.add_shifted_limbs(0, [*word, 0]);
            zero -= *word;
        }
        Cancellation::Operand { limbs, shift } => {
            let mut operand = Accumulator::new();
            operand.add_shifted_limbs(0, limbs.iter().copied());
            zero.add_shifted_limbs(*shift, limbs.iter().copied());
            zero.sub_shifted(*shift, &operand);
        }
    }
    assert_value(&zero, &IBig::ZERO);
    zero
}

/// Draw a fixed boundary shift, or any shift up to the widest.
///
/// Proptest shrinks a union toward its earlier alternatives, and the random
/// alternative shrinks toward the widest shift, so a failure reports the
/// widest failing shift, where growth with the shift is easiest to see.
fn zero_shift() -> impl Strategy<Value = u64> {
    prop_oneof![
        1 => Just(WIDEST_ZERO_SHIFT),
        1 => Just(32 * 1_000),
        1 => Just(64),
        1 => Just(33),
        1 => Just(32),
        1 => Just(31),
        1 => Just(1),
        1 => Just(0),
        8 => (0..=WIDEST_ZERO_SHIFT).prop_map(|below| WIDEST_ZERO_SHIFT - below),
    ]
}

/// A receiver for a zero operand, in each representation it can start from.
#[derive(Clone, Debug)]
enum Receiver {
    /// A new accumulator, which holds no digit storage yet.
    Fresh,
    /// A value held in the small representation.
    Small(i64),
    /// A value of several limbs, held in digits.
    Wide(Vec<u64>),
    /// A value of several limbs, then clear: zero, with its buffer kept.
    ///
    /// A zero operand can land inside this buffer, which is wider than the
    /// receiver's working width.
    Clear(Vec<u64>),
    /// A value of several limbs with the digit `2^33 - 2` one position above them.
    ///
    /// Two deposits of `2^32 - 1` there leave a digit just inside the
    /// redundant range without carrying, so a zero deposited beside it can
    /// carry one position past the buffer.
    NearLimit(Vec<u64>),
}

/// Construct each receiver alongside its oracle value.
impl Receiver {
    /// Build the receiver and the value it must keep.
    fn build(&self) -> (Accumulator, IBig) {
        let mut acc = Accumulator::new();
        let value = match self {
            Receiver::Fresh => IBig::ZERO,
            Receiver::Small(value) => {
                acc += *value;
                IBig::from(*value)
            }
            Receiver::Wide(limbs) => {
                acc.add_shifted_limbs(0, limbs.iter().copied());
                IBig::from(from_limbs(limbs))
            }
            Receiver::Clear(limbs) => {
                acc.add_shifted_limbs(0, limbs.iter().copied());
                acc.clear();
                IBig::ZERO
            }
            Receiver::NearLimit(limbs) => {
                acc.add_shifted_limbs(0, limbs.iter().copied());
                let top = 64 * limbs.len() as u64;
                for _ in 0..2 {
                    acc.add_shifted_limbs(top, [u64::from(u32::MAX)]);
                }
                IBig::from(from_limbs(limbs))
                    + (IBig::from(2 * u64::from(u32::MAX)) << usize::try_from(top).unwrap())
            }
        };
        (acc, value)
    }
}

/// Draw every kind of receiver, with three to six limbs where it holds limbs.
fn receiver() -> impl Strategy<Value = Receiver> {
    prop_oneof![
        Just(Receiver::Fresh),
        any::<i64>().prop_map(Receiver::Small),
        proptest::collection::vec(any::<u64>(), 3..=6).prop_map(Receiver::Wide),
        proptest::collection::vec(any::<u64>(), 3..=6).prop_map(Receiver::Clear),
        proptest::collection::vec(any::<u64>(), 3..=6).prop_map(Receiver::NearLimit),
    ]
}

/// Assert that a shifted zero is still zero, takes no more space than the
/// digit-stored zero it came from, and keeps that zero's representation and
/// buffer.
///
/// The stored digits may not grow, and both bounds depend only on the
/// unshifted zero, so they hold at every shift. The crate promises constant
/// space; the representation's one-way rule adds that a digit-stored value
/// stays in digits, so a shift neither returns it to the small form nor
/// drops its buffer.
fn assert_shifted_zero(shifted: Accumulator, shift: u64, unshifted: &Accumulator) {
    // Read the space before comparing: the comparison compacts a cancelled
    // stored form, which would hide what the shift left behind.
    let stored = shifted.stored_digit_count();
    let retained = shifted.stored().digits.retained_len();
    assert!(
        shifted.stored().small.is_none(),
        "a digit-stored zero shifted by {shift} bits stays in digits"
    );
    assert_eq!(
        shifted.sign(),
        Ordering::Equal,
        "shifting zero by {shift} bits yields zero"
    );
    let stored_before = unshifted.stored_digit_count();
    assert!(
        stored <= stored_before,
        "a zero shifted by {shift} bits takes constant space: \
         {stored} stored digits, up from {stored_before}"
    );
    let retained_before = unshifted.stored().digits.retained_len();
    assert_eq!(
        retained, retained_before,
        "a zero shifted by {shift} bits keeps its buffer"
    );
}

/// Assert that applying a zero operand at a shift keeps both values and leaves
/// the receiver's space unchanged.
///
/// The operand is a fresh stored copy of `zero`, so the call sees its cancelled form.
/// Comparing the operand with zero skips it, so neither the receiver's stored
/// digit count nor its buffer may change, whether the zero's digits would land
/// inside the buffer or past it.
fn assert_zero_operand(
    receiver: &Receiver,
    name: &str,
    apply: fn(&mut Accumulator, u64, &Accumulator),
    shift: u64,
    zero: &Accumulator,
) {
    let (mut acc, value) = receiver.build();
    let stored_before = acc.stored_digit_count();
    let retained_before = acc.stored().digits.retained_len();
    let operand = zero.stored_copy();
    apply(&mut acc, shift, &operand);
    assert_value(&acc, &value);
    assert_value(&operand, &IBig::ZERO);
    assert_eq!(
        acc.stored_digit_count(),
        stored_before,
        "{name} of a zero at {shift} bits leaves the receiver's stored digits"
    );
    assert_eq!(
        acc.stored().digits.retained_len(),
        retained_before,
        "{name} of a zero at {shift} bits leaves the receiver's buffer"
    );
}

proptest! {
    /// Shifting a zero, or adding or subtracting it at a shift, takes constant
    /// space whatever digits store it.
    ///
    /// Each zero is built by [`redundant_zero`], so it is never a known zero.
    /// A shifted zero may not gain stored digits, and keeps its digit
    /// representation and buffer. Adding or subtracting it
    /// leaves every kind of receiver's value and space unchanged, as
    /// [`assert_zero_operand`] checks. No bound depends on the shift. Depositing the
    /// cancelling digits at the shifted position instead retains space that
    /// grows with the shift, and on a 32-bit target panics once that space
    /// reaches an unaddressable position.
    #[test]
    fn shifting_zero_takes_constant_space_whatever_its_stored_form(
        k in 1u64..=40,
        cancellation in cancellation(),
        shift in zero_shift(),
        receiver in receiver(),
    ) {
        let zero = redundant_zero(k, &cancellation);

        let mut assigned = zero.stored_copy();
        assigned <<= shift;
        assert_shifted_zero(assigned, shift, &zero);
        assert_shifted_zero(zero.stored_copy() << shift, shift, &zero);

        assert_zero_operand(&receiver, "add_shifted", Accumulator::add_shifted, shift, &zero);
        assert_zero_operand(&receiver, "sub_shifted", Accumulator::sub_shifted, shift, &zero);
    }
}

proptest! {
    /// Repeatedly adding zero at the top of a receiver's buffer leaves the
    /// buffer's length unchanged.
    ///
    /// Each step adds or subtracts a fresh stored copy of the same zero, so every
    /// step sees its cancelled form. Its top digit lands on the buffer's top
    /// position or up to three positions past it, at a varying bit offset.
    /// Checking each operation alone would miss growth that accumulates: a
    /// zero check that let through zeros landing just past the buffer would
    /// grow it by one position per step.
    #[test]
    fn zero_operands_at_the_buffer_top_never_accumulate_space(
        k in 1u64..=40,
        cancellation in cancellation(),
        receiver in receiver(),
        landings in proptest::collection::vec((0u64..=3, 0u64..32), 1..=64),
    ) {
        let zero = redundant_zero(k, &cancellation);
        let top_offset = zero.stored_digit_count() as u64 - 1;
        let (mut acc, value) = receiver.build();
        let retained_before = acc.stored().digits.retained_len();
        for (step, &(past_top, bit)) in landings.iter().enumerate() {
            let top = (acc.stored().digits.retained_len() as u64).saturating_sub(1) + past_top;
            let shift = 32 * top.saturating_sub(top_offset) + bit;
            if step % 2 == 0 {
                acc.add_shifted(shift, &zero.stored_copy());
            } else {
                acc.sub_shifted(shift, &zero.stored_copy());
            }
        }
        assert_value(&acc, &value);
        assert_eq!(
            acc.stored().digits.retained_len(),
            retained_before,
            "{} zero operands at the buffer's top leave the buffer's length",
            landings.len()
        );
    }
}

/// The widths at which [`sign_queries_add_written_set_space_independent_of_width`]
/// reads each value shape, in digit positions.
///
/// They span three orders of magnitude, so storage that grew with the width
/// would differ between them by thousands of bitset words.
const SIGN_QUERY_WIDTHS: [u64; 4] = [100, 1_000, 10_000, 100_000];

/// A sign query adds the same written-set storage at every width, as the
/// crate's O(1) additional-space bound for sign queries requires.
///
/// Each shape is a value whose top digit `s` lies far above its other
/// digits: `2^(32s)` alone, whose set is a prefix and a run; with digits at
/// 0 and `s/2`, two gaps, which the set holds in a bitset; and with digits
/// at 10 and 20 as well, a bitset holding more gaps. The query reads the top
/// digit, finds the unwritten position below it, and writes the compacted
/// top back there. For every shape, the bitset words the query adds must
/// equal those it adds at the narrowest width.
#[test]
fn sign_queries_add_written_set_space_independent_of_width() {
    #[allow(clippy::type_complexity)]
    let shapes: [(&str, fn(u64) -> Vec<u64>); 3] = [
        ("a lone top digit", |_| vec![]),
        ("two gaps", |s| vec![0, s / 2]),
        ("several gaps", |s| vec![0, 10, 20, s / 2]),
    ];
    for (name, lower) in shapes {
        let added_at = |s: u64| {
            let mut acc = Accumulator::new();
            let mut oracle = IBig::ZERO;
            for position in lower(s).into_iter().chain([s]) {
                acc.add_shifted_limbs(32 * position, [1]);
                oracle += IBig::from(1) << usize::try_from(32 * position).unwrap();
            }
            let before = acc.stored().digits.written_storage_words();
            assert_eq!(acc.sign(), Ordering::Greater, "{name} at {s}");
            acc.stored().assert_invariants(&[]);
            assert_value(&acc, &oracle);
            let after = acc.stored().digits.written_storage_words();
            after as i128 - before as i128
        };
        let narrowest = added_at(SIGN_QUERY_WIDTHS[0]);
        for s in SIGN_QUERY_WIDTHS {
            assert_eq!(
                added_at(s),
                narrowest,
                "a sign query on {name} at width {s} adds bitset words that grow with the width"
            );
        }
    }
}
