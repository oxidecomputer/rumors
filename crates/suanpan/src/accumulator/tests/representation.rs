//! Arithmetic preserves the stored-form invariants, and shifting zero takes
//! constant space whatever its stored form.
//!
//! At every step, compare the value with a big-integer oracle and inspect the
//! representation itself. A stale range could skip a nonzero digit on a later
//! query even while the current value still reads correctly.
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
    /// which can drive a sign fold's partial to zero above a recorded zero range.
    word32: UBig,
    /// A unit contribution at digit three.
    p96: IBig,
    /// A unit contribution at digit seven.
    p224: IBig,
    /// A word-sized contribution large enough to carry across digits.
    max64: IBig,
}

/// Number of update and sign-query operations explored at each step.
const REPRESENTATION_OPS: u8 = 11;

/// Apply the finite operation alphabet using its precomputed operands.
impl Operands {
    /// Apply one operation to the accumulator and oracle in lockstep.
    ///
    /// Unit and maximum-word updates exercise ordinary deposits and carries.
    /// Jumps to digits three and seven create, clear, and split zero runs.
    /// Depositing a whole coefficient of ±2^32 at digit six makes the sign
    /// fold reach a run with either a small nonzero or exactly zero partial.
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
            _ => {
                assert_eq!(acc.cmp_zero(), oracle_sign(oracle), "comparison with zero");
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
fn check_schedules(
    operands: &Operands,
    acc: &Accumulator,
    oracle: &IBig,
    schedule: &mut Vec<u8>,
    depth: usize,
) {
    for op in 0..REPRESENTATION_OPS {
        schedule.push(op);
        let mut next_acc = acc.clone();
        let mut next_oracle = oracle.clone();
        operands.apply(&mut next_acc, &mut next_oracle, op);
        next_acc
            .digits
            .assert_invariants(next_acc.small.is_some(), schedule);
        assert_value(&next_acc, &next_oracle);
        if depth > 1 {
            check_schedules(operands, &next_acc, &next_oracle, schedule, depth - 1);
        }
        schedule.pop();
    }
}

/// Every schedule of up to six operations preserves the digit and range invariants.
///
/// The eleven-operation alphabet exercises gap creation, splitting, and
/// consumption, including carries and sign collapse inside a recorded zero range.
/// Every state must match the oracle and retain disjoint zero ranges wholly
/// within the stored width.
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

proptest! {
    /// Longer streams with wider shifts preserve the value, signed-digit bounds,
    /// stored width, and disjoint zero ranges after every step.
    ///
    /// The exhaustive sweep's long-schedule, deep-shift complement:
    /// shifts to 4,096 bits, schedules to 150 ops, comparisons with zero
    /// interleaved throughout.
    #[test]
    fn representation_invariants_hold_on_run_forming_streams(
        ops in proptest::collection::vec(
            (0u8..5, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),
            1..150,
        ),
        digits_first: bool,
    ) {
        let mut acc = fresh(digits_first);
        let mut oracle = IBig::from(0);
        let mut schedule: Vec<u8> = Vec::new();
        for (step, (arm, limbs, shift)) in ops.iter().enumerate() {
            schedule.push(*arm);
            let value = from_limbs(limbs);
            let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();
            match arm {
                0 => {
                    acc.add_limb_value_shl(&value, *shift);
                    oracle += scaled;
                }
                1 => {
                    acc.sub_limb_value_shl(&value, *shift);
                    oracle -= scaled;
                }
                2 => {
                    acc.sub_value_shl(&value, *shift);
                    oracle -= scaled;
                }
                3 => {
                    let delta = limbs[0] as i64;
                    acc += delta;
                    oracle += delta;
                }
                _ => {
                    prop_assert_eq!(
                        acc.cmp_zero(),
                        oracle_sign(&oracle),
                        "sign at step {}",
                        step
                    );
                }
            }
            acc.digits.assert_invariants(acc.small.is_some(), &schedule);
            if step % 32 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }
}

/// The widest shift drawn, in bits: `2^20` digit positions.
///
/// A zero whose space grew with the shift would retain about a million
/// positions here, and a passing case still runs in milliseconds.
const WIDEST_ZERO_SHIFT: u64 = 32 << 20;

/// Digit positions that one operation on zero may add to the buffer, at any
/// shift.
///
/// A zero deposited inside the buffer leaves the value unchanged, but when the
/// receiver's top digit is near the digit limit, the deposit can carry one
/// position past it. No carry goes further, because every position above is
/// zero. Carries cannot build up at the new position either: read from the
/// top, a zero's partial sums stay below 3, so the digit there stays within a
/// few units of zero.
const ZERO_RETAINED_GROWTH: usize = 1;

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
    /// operands extend the buffer and its zero ranges.
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
    /// A value of several limbs, then reset: zero, with its buffer kept.
    ///
    /// A zero operand can land inside this buffer, which is wider than the
    /// receiver's working width.
    Reset(Vec<u64>),
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
            Receiver::Reset(limbs) => {
                acc.add_shifted_limbs(0, limbs.iter().copied());
                acc.reset();
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
        proptest::collection::vec(any::<u64>(), 3..=6).prop_map(Receiver::Reset),
        proptest::collection::vec(any::<u64>(), 3..=6).prop_map(Receiver::NearLimit),
    ]
}

/// Assert that a shifted zero is still zero and takes no more space than the
/// zero it came from.
///
/// Stored digits may not grow, and the buffer may grow by at most
/// [`ZERO_RETAINED_GROWTH`] positions. Both bounds depend only on the unshifted
/// zero, so they hold at every shift. The crate promises constant space, not a
/// particular stored form.
fn assert_shifted_zero(mut shifted: Accumulator, shift: u64, unshifted: &Accumulator) {
    // Read the space before comparing: the comparison compacts a cancelled
    // stored form, which would hide what the shift left behind.
    let stored = shifted.stored_digit_count();
    let retained = shifted.digits.retained_len();
    assert_eq!(
        shifted.cmp_zero(),
        Ordering::Equal,
        "shifting zero by {shift} bits yields zero"
    );
    let stored_before = unshifted.stored_digit_count();
    assert!(
        stored <= stored_before,
        "a zero shifted by {shift} bits takes constant space: \
         {stored} stored digits, up from {stored_before}"
    );
    let retained_before = unshifted.digits.retained_len();
    assert!(
        retained <= retained_before + ZERO_RETAINED_GROWTH,
        "a zero shifted by {shift} bits takes constant space: \
         {retained} retained digit positions, up from {retained_before}"
    );
}

/// Assert that applying a zero operand at a shift keeps the receiver's value
/// and bounds its space.
///
/// The buffer may grow by at most [`ZERO_RETAINED_GROWTH`] positions. The
/// stored digit count may rise to the larger of its former value and the
/// buffer's length, plus the same allowance, because a zero deposited inside
/// the buffer leaves its cancelling digits there.
fn assert_zero_operand(
    receiver: &Receiver,
    name: &str,
    apply: fn(&mut Accumulator, u64, &Accumulator),
    shift: u64,
    zero: &Accumulator,
) {
    let (mut acc, value) = receiver.build();
    let stored_before = acc.stored_digit_count();
    let retained_before = acc.digits.retained_len();
    apply(&mut acc, shift, zero);
    assert_value(&acc, &value);
    let stored = acc.stored_digit_count();
    let stored_limit = stored_before.max(retained_before) + ZERO_RETAINED_GROWTH;
    assert!(
        stored <= stored_limit,
        "{name} of a zero at {shift} bits takes constant space: \
         {stored} stored digits, beyond the {stored_limit} the receiver's \
         held space allows"
    );
    let retained = acc.digits.retained_len();
    assert!(
        retained <= retained_before + ZERO_RETAINED_GROWTH,
        "{name} of a zero at {shift} bits takes constant space: \
         {retained} retained digit positions, up from {retained_before}"
    );
}

proptest! {
    /// Shifting a zero, or adding or subtracting it at a shift, takes constant
    /// space whatever digits store it.
    ///
    /// Each zero is built by [`redundant_zero`], so it is never a known zero.
    /// A shifted zero may not gain stored digits. Adding or subtracting it
    /// leaves every kind of receiver's value unchanged, with the bounds of
    /// [`assert_zero_operand`]. No bound depends on the shift. Depositing the
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

        let mut assigned = zero.clone();
        assigned <<= shift;
        assert_shifted_zero(assigned, shift, &zero);
        assert_shifted_zero(zero.clone() << shift, shift, &zero);

        assert_zero_operand(&receiver, "add_shifted", Accumulator::add_shifted, shift, &zero);
        assert_zero_operand(&receiver, "sub_shifted", Accumulator::sub_shifted, shift, &zero);
    }
}

proptest! {
    /// Repeatedly adding zero at the top of a receiver's buffer keeps the
    /// buffer within the larger of its former length and the value's width
    /// plus [`ZERO_RETAINED_GROWTH`].
    ///
    /// Each step adds or subtracts the same zero, with its top digit on the
    /// buffer's top position or up to three positions past it, at a varying
    /// bit offset. A zero deposited inside the buffer carries past it only
    /// where the receiver's own value fills the top digit. Checking each
    /// operation alone would miss growth that accumulates: a zero check that
    /// let through zeros landing just past the buffer would grow it by one
    /// position per step, while every step stayed within a constant.
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
        let value_digits = usize::try_from(value.bits().div_ceil(32)).unwrap().max(1);
        let limit = acc
            .digits
            .retained_len()
            .max(value_digits + ZERO_RETAINED_GROWTH);
        for (step, &(past_top, bit)) in landings.iter().enumerate() {
            let top = (acc.digits.retained_len() as u64).saturating_sub(1) + past_top;
            let shift = 32 * top.saturating_sub(top_offset) + bit;
            if step % 2 == 0 {
                acc.add_shifted(shift, &zero);
            } else {
                acc.sub_shifted(shift, &zero);
            }
        }
        assert_value(&acc, &value);
        let retained = acc.digits.retained_len();
        assert!(
            retained <= limit,
            "{} zero operands at the buffer's top leave {retained} retained digit \
             positions, beyond {limit}",
            landings.len()
        );
    }
}
