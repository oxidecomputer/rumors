//! Differential coverage of the complete public arithmetic surface.
//!
//! One generated program updates an [`Accumulator`] and an independent
//! [`BigInt`] in lockstep. After every instruction, the observer checks every
//! public query and primitive conversion, reading stored copies so that
//! observing never changes the tested history; cancellation therefore builds up
//! across steps until an instruction reads in place. Instructions cover the
//! standard arithmetic traits with owned and borrowed operands on either side,
//! shifted limb and accumulator inputs, sums, `abs_diff`, reservation, clear,
//! and in-place reads.
//!
//! The generators put explicit weight on the representation's boundaries:
//! 30-bit small-path shifts, 32-bit digits, 64-bit limbs, the `2^96` small
//! limit, cancellation across sparse high positions, and every primitive
//! integer endpoint. Random values and shifts between those points exercise
//! their neighborhoods rather than only the named examples.

use core::cmp::Ordering;
use core::hash::BuildHasher;

use num_bigint::{BigInt, BigUint};
use proptest::prelude::*;

use super::{
    assert_stored_width_bounds, assert_value, cancelled_top, fresh, from_limbs, oracle_sign,
    own_digits, read_value, Accumulator,
};

/// One primitive integer, retaining its Rust type for trait dispatch.
#[derive(Clone, Debug)]
enum Primitive {
    /// An `i8` operand.
    I8(i8),
    /// An `i16` operand.
    I16(i16),
    /// An `i32` operand.
    I32(i32),
    /// An `i64` operand.
    I64(i64),
    /// An `i128` operand.
    I128(i128),
    /// An `isize` operand.
    Isize(isize),
    /// A `u8` operand.
    U8(u8),
    /// A `u16` operand.
    U16(u16),
    /// A `u32` operand.
    U32(u32),
    /// A `u64` operand.
    U64(u64),
    /// A `u128` operand.
    U128(u128),
    /// A `usize` operand.
    Usize(usize),
}

/// The arithmetic forms supported for every primitive.
#[derive(Clone, Copy, Debug)]
enum PrimitiveArithmetic {
    /// `accumulator += value`.
    AddAssign,
    /// `accumulator -= value`.
    SubAssign,
    /// `accumulator + value`.
    Add,
    /// `accumulator - value`.
    Subtract,
    /// `&accumulator + value`.
    BorrowedAdd,
    /// `&accumulator - value`.
    BorrowedSubtract,
    /// `accumulator += &value`.
    AddAssignRef,
    /// `accumulator -= &value`.
    SubAssignRef,
    /// `accumulator + &value`.
    AddRef,
    /// `accumulator - &value`.
    SubtractRef,
    /// `&accumulator + &value`.
    BorrowedAddRef,
    /// `&accumulator - &value`.
    BorrowedSubtractRef,
    /// `value + accumulator`.
    LeftAdd,
    /// `value - accumulator`.
    LeftSubtract,
    /// `value + &accumulator`.
    LeftAddBorrowed,
    /// `value - &accumulator`.
    LeftSubtractBorrowed,
    /// `&value + accumulator`.
    RefLeftAdd,
    /// `&value - accumulator`.
    RefLeftSubtract,
    /// `&value + &accumulator`.
    RefLeftAddBorrowed,
    /// `&value - &accumulator`.
    RefLeftSubtractBorrowed,
}

/// How a form combines the accumulator `a` with the primitive `p`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Combination {
    /// `a + p`.
    Sum,
    /// `a - p`.
    Difference,
    /// `p - a`.
    ReversedDifference,
}

impl PrimitiveArithmetic {
    /// Every form, for generation.
    const ALL: [Self; 20] = [
        Self::AddAssign,
        Self::SubAssign,
        Self::Add,
        Self::Subtract,
        Self::BorrowedAdd,
        Self::BorrowedSubtract,
        Self::AddAssignRef,
        Self::SubAssignRef,
        Self::AddRef,
        Self::SubtractRef,
        Self::BorrowedAddRef,
        Self::BorrowedSubtractRef,
        Self::LeftAdd,
        Self::LeftSubtract,
        Self::LeftAddBorrowed,
        Self::LeftSubtractBorrowed,
        Self::RefLeftAdd,
        Self::RefLeftSubtract,
        Self::RefLeftAddBorrowed,
        Self::RefLeftSubtractBorrowed,
    ];

    /// The value the form computes.
    fn combination(self) -> Combination {
        match self {
            Self::SubAssign
            | Self::Subtract
            | Self::BorrowedSubtract
            | Self::SubAssignRef
            | Self::SubtractRef
            | Self::BorrowedSubtractRef => Combination::Difference,
            Self::LeftSubtract
            | Self::LeftSubtractBorrowed
            | Self::RefLeftSubtract
            | Self::RefLeftSubtractBorrowed => Combination::ReversedDifference,
            _ => Combination::Sum,
        }
    }
}

/// Replace `actual` by the result of `operation` on a borrow of it, checking
/// that the borrowed operand keeps its value, `oracle`.
fn replace_borrowed(
    actual: &mut Accumulator,
    oracle: &BigInt,
    operation: impl FnOnce(&Accumulator) -> Accumulator,
) {
    let result = operation(actual);
    assert_value(actual, oracle);
    *actual = result;
}

/// Dispatch primitive traits without erasing the concrete integer type.
impl Primitive {
    /// Apply one arithmetic form to the accumulator and oracle.
    fn apply(&self, operation: PrimitiveArithmetic, actual: &mut Accumulator, oracle: &mut BigInt) {
        macro_rules! apply {
            ($value:expr) => {{
                let value = *$value;
                match operation {
                    PrimitiveArithmetic::AddAssign => *actual += value,
                    PrimitiveArithmetic::SubAssign => *actual -= value,
                    PrimitiveArithmetic::Add => {
                        *actual = core::mem::take(actual) + value;
                    }
                    PrimitiveArithmetic::Subtract => {
                        *actual = core::mem::take(actual) - value;
                    }
                    PrimitiveArithmetic::BorrowedAdd => {
                        replace_borrowed(actual, oracle, |left| left + value);
                    }
                    PrimitiveArithmetic::BorrowedSubtract => {
                        replace_borrowed(actual, oracle, |left| left - value);
                    }
                    PrimitiveArithmetic::AddAssignRef => *actual += &value,
                    PrimitiveArithmetic::SubAssignRef => *actual -= &value,
                    PrimitiveArithmetic::AddRef => {
                        *actual = core::mem::take(actual) + &value;
                    }
                    PrimitiveArithmetic::SubtractRef => {
                        *actual = core::mem::take(actual) - &value;
                    }
                    PrimitiveArithmetic::BorrowedAddRef => {
                        replace_borrowed(actual, oracle, |left| left + &value);
                    }
                    PrimitiveArithmetic::BorrowedSubtractRef => {
                        replace_borrowed(actual, oracle, |left| left - &value);
                    }
                    PrimitiveArithmetic::LeftAdd => {
                        *actual = value + core::mem::take(actual);
                    }
                    PrimitiveArithmetic::LeftSubtract => {
                        *actual = value - core::mem::take(actual);
                    }
                    PrimitiveArithmetic::LeftAddBorrowed => {
                        replace_borrowed(actual, oracle, |right| value + right);
                    }
                    PrimitiveArithmetic::LeftSubtractBorrowed => {
                        replace_borrowed(actual, oracle, |right| value - right);
                    }
                    PrimitiveArithmetic::RefLeftAdd => {
                        *actual = &value + core::mem::take(actual);
                    }
                    PrimitiveArithmetic::RefLeftSubtract => {
                        *actual = &value - core::mem::take(actual);
                    }
                    PrimitiveArithmetic::RefLeftAddBorrowed => {
                        replace_borrowed(actual, oracle, |right| &value + right);
                    }
                    PrimitiveArithmetic::RefLeftSubtractBorrowed => {
                        replace_borrowed(actual, oracle, |right| &value - right);
                    }
                }
                let value = BigInt::from(value);
                match operation.combination() {
                    Combination::Sum => *oracle += value,
                    Combination::Difference => *oracle -= value,
                    Combination::ReversedDifference => *oracle = value - &*oracle,
                }
            }};
        }

        match self {
            Primitive::I8(value) => apply!(value),
            Primitive::I16(value) => apply!(value),
            Primitive::I32(value) => apply!(value),
            Primitive::I64(value) => apply!(value),
            Primitive::I128(value) => apply!(value),
            Primitive::Isize(value) => apply!(value),
            Primitive::U8(value) => apply!(value),
            Primitive::U16(value) => apply!(value),
            Primitive::U32(value) => apply!(value),
            Primitive::U64(value) => apply!(value),
            Primitive::U128(value) => apply!(value),
            Primitive::Usize(value) => apply!(value),
        }
    }

    /// Construct the corresponding accumulator and oracle values.
    fn values(&self) -> (Accumulator, BigInt) {
        macro_rules! values {
            ($value:expr) => {{
                let value = *$value;
                (Accumulator::from(value), BigInt::from(value))
            }};
        }

        match self {
            Primitive::I8(value) => values!(value),
            Primitive::I16(value) => values!(value),
            Primitive::I32(value) => values!(value),
            Primitive::I64(value) => values!(value),
            Primitive::I128(value) => values!(value),
            Primitive::Isize(value) => values!(value),
            Primitive::U8(value) => values!(value),
            Primitive::U16(value) => values!(value),
            Primitive::U32(value) => values!(value),
            Primitive::U64(value) => values!(value),
            Primitive::U128(value) => values!(value),
            Primitive::Usize(value) => values!(value),
        }
    }

    /// Check comparison and equality with this primitive against the oracle.
    fn assert_comparisons(&self, actual: &Accumulator, oracle: &BigInt, trace: &[u8]) {
        macro_rules! check {
            ($value:expr) => {{
                let value = *$value;
                let expected = oracle.cmp(&BigInt::from(value));
                assert_eq!(
                    actual.stored_copy().partial_cmp(&value),
                    Some(expected),
                    "ordering with {value} after {trace:?}"
                );
                assert_eq!(
                    actual.stored_copy() == value,
                    expected == Ordering::Equal,
                    "equality with {value} after {trace:?}"
                );
            }};
        }

        match self {
            Primitive::I8(value) => check!(value),
            Primitive::I16(value) => check!(value),
            Primitive::I32(value) => check!(value),
            Primitive::I64(value) => check!(value),
            Primitive::I128(value) => check!(value),
            Primitive::Isize(value) => check!(value),
            Primitive::U8(value) => check!(value),
            Primitive::U16(value) => check!(value),
            Primitive::U32(value) => check!(value),
            Primitive::U64(value) => check!(value),
            Primitive::U128(value) => check!(value),
            Primitive::Usize(value) => check!(value),
        }
    }

    /// Check `From` and the type-specific `Sum` implementations, over values
    /// and over borrows.
    fn check_construction_and_sum(&self) {
        macro_rules! check {
            ($value:expr) => {{
                let value = *$value;
                let oracle = BigInt::from(value);
                assert_value(&Accumulator::from(value), &oracle);
                let sum: Accumulator = [value, value].into_iter().sum();
                assert_value(&sum, &(&oracle * 2));
                let sum: Accumulator = [value, value].iter().sum();
                assert_value(&sum, &(oracle * 2));
            }};
        }

        match self {
            Primitive::I8(value) => check!(value),
            Primitive::I16(value) => check!(value),
            Primitive::I32(value) => check!(value),
            Primitive::I64(value) => check!(value),
            Primitive::I128(value) => check!(value),
            Primitive::Isize(value) => check!(value),
            Primitive::U8(value) => check!(value),
            Primitive::U16(value) => check!(value),
            Primitive::U32(value) => check!(value),
            Primitive::U64(value) => check!(value),
            Primitive::U128(value) => check!(value),
            Primitive::Usize(value) => check!(value),
        }
    }
}

/// A shift count retaining the primitive type used by the operator.
#[derive(Clone, Debug)]
enum Shift {
    /// An `i8` count.
    I8(i8),
    /// An `i16` count.
    I16(i16),
    /// An `i32` count.
    I32(i32),
    /// An `i64` count.
    I64(i64),
    /// An `i128` count.
    I128(i128),
    /// An `isize` count.
    Isize(isize),
    /// A `u8` count.
    U8(u8),
    /// A `u16` count.
    U16(u16),
    /// A `u32` count.
    U32(u32),
    /// A `u64` count.
    U64(u64),
    /// A `u128` count.
    U128(u128),
    /// A `usize` count.
    Usize(usize),
}

/// The shift operator forms.
#[derive(Clone, Copy, Debug)]
enum ShiftForm {
    /// `accumulator <<= count`.
    Assign,
    /// `accumulator << count`.
    Owned,
    /// `&accumulator << count`.
    Borrowed,
    /// `accumulator <<= &count`.
    AssignRef,
    /// `accumulator << &count`.
    OwnedRef,
    /// `&accumulator << &count`.
    BorrowedRef,
}

impl ShiftForm {
    /// Every form, for generation.
    const ALL: [Self; 6] = [
        Self::Assign,
        Self::Owned,
        Self::Borrowed,
        Self::AssignRef,
        Self::OwnedRef,
        Self::BorrowedRef,
    ];
}

/// Exercise every shift operator form.
impl Shift {
    /// Multiply both values by this power of two.
    fn apply(&self, form: ShiftForm, actual: &mut Accumulator, oracle: &mut BigInt) {
        macro_rules! shift {
            ($count:expr) => {{
                let count = *$count;
                match form {
                    ShiftForm::Assign => *actual <<= count,
                    ShiftForm::Owned => *actual = core::mem::take(actual) << count,
                    ShiftForm::Borrowed => replace_borrowed(actual, oracle, |left| left << count),
                    ShiftForm::AssignRef => *actual <<= &count,
                    ShiftForm::OwnedRef => *actual = core::mem::take(actual) << &count,
                    ShiftForm::BorrowedRef => {
                        replace_borrowed(actual, oracle, |left| left << &count);
                    }
                }
                *oracle <<= usize::try_from(count).expect("generated shifts are nonnegative");
            }};
        }

        match self {
            Shift::I8(count) => shift!(count),
            Shift::I16(count) => shift!(count),
            Shift::I32(count) => shift!(count),
            Shift::I64(count) => shift!(count),
            Shift::I128(count) => shift!(count),
            Shift::Isize(count) => shift!(count),
            Shift::U8(count) => shift!(count),
            Shift::U16(count) => shift!(count),
            Shift::U32(count) => shift!(count),
            Shift::U64(count) => shift!(count),
            Shift::U128(count) => shift!(count),
            Shift::Usize(count) => shift!(count),
        }
    }
}

/// A value used as an accumulator operand, including difficult representations.
#[derive(Clone, Debug)]
enum Operand {
    /// A primitive-constructed value, usually retained in the small form.
    Primitive(Primitive),
    /// A signed limb magnitude at an arbitrary bit offset.
    ShiftedLimbs {
        /// Whether the magnitude is subtracted from zero.
        negative: bool,
        /// Little-endian magnitude limbs, including permitted high zeros.
        limbs: Vec<u64>,
        /// Power-of-two scale.
        shift: u64,
    },
    /// A value immediately around the small representation's `2^96` limit.
    SmallBoundary {
        /// Signed displacement from the positive limit.
        displacement: i8,
        /// Whether to negate the completed value.
        negative: bool,
    },
    /// A small tail stored after a high contribution cancels exactly.
    CancelledHigh {
        /// The cancelled contribution's scale.
        shift: u64,
        /// The surviving value.
        tail: Primitive,
    },
    /// A primitive tail under a stored top that cancels across two digits.
    ///
    /// The digits `1` at `position` and `-2^32` at `position - 1` sum to
    /// zero but stay stored, unlike [`CancelledHigh`](Self::CancelledHigh)'s,
    /// which cancel within one digit. The stored width therefore exceeds the
    /// value's until a read compacts it.
    CancelledTop {
        /// The position of the cancelled top digit, at least 1.
        position: u64,
        /// The surviving value.
        tail: Primitive,
    },
    /// Another operand's value, negated: held in digits whose flag is set
    /// when that value is held in digits, including a zero whose digits
    /// cancel.
    Negated(Box<Operand>),
}

/// Build operand values exclusively through the public arithmetic surface.
impl Operand {
    /// Return matching accumulator and big-integer representations.
    ///
    /// Recurses only through [`Negated`](Self::Negated), which
    /// [`arb_operand`] nests at most once, since it negates only an
    /// unnegated operand.
    fn values(&self) -> (Accumulator, BigInt) {
        match self {
            Operand::Primitive(value) => value.values(),
            Operand::ShiftedLimbs {
                negative,
                limbs,
                shift,
            } => {
                let magnitude = from_limbs(limbs);
                let mut actual = Accumulator::new();
                if *negative {
                    actual.sub_shifted_limbs(*shift, limbs.iter().copied());
                } else {
                    actual.add_shifted_limbs(*shift, limbs.iter().copied());
                }
                let mut oracle = BigInt::from(magnitude) << *shift as usize;
                if *negative {
                    oracle = -oracle;
                }
                (actual, oracle)
            }
            Operand::SmallBoundary {
                displacement,
                negative,
            } => {
                let limit = 1u128 << 96;
                let mut actual = Accumulator::from(limit);
                actual += *displacement;
                let mut oracle = BigInt::from(limit) + i64::from(*displacement);
                if *negative {
                    actual = -actual;
                    oracle = -oracle;
                }
                (actual, oracle)
            }
            Operand::CancelledHigh { shift, tail } => {
                let mut actual = Accumulator::new();
                actual.add_shifted_limbs(*shift, [1]);
                actual.sub_shifted_limbs(*shift, [1]);
                let (tail, oracle) = tail.values();
                actual += &tail;
                (actual, oracle)
            }
            Operand::CancelledTop { position, tail } => {
                let mut actual = cancelled_top(*position);
                let mut oracle = BigInt::ZERO;
                tail.apply(PrimitiveArithmetic::AddAssign, &mut actual, &mut oracle);
                (actual, oracle)
            }
            Operand::Negated(operand) => {
                let (actual, oracle) = operand.values();
                (-actual, -oracle)
            }
        }
    }
}

/// Arithmetic forms for accumulator operands.
#[derive(Clone, Copy, Debug)]
enum AccumulatorArithmetic {
    /// `accumulator += &operand`.
    AddAssignBorrowed,
    /// `accumulator -= &operand`.
    SubAssignBorrowed,
    /// `accumulator += operand`.
    AddAssignOwned,
    /// `accumulator -= operand`.
    SubAssignOwned,
    /// `accumulator + &operand`.
    AddBorrowed,
    /// `accumulator - &operand`.
    SubBorrowed,
    /// `accumulator + operand`.
    AddOwned,
    /// `accumulator - operand`.
    SubOwned,
    /// `&accumulator + &operand`.
    BorrowedAddBorrowed,
    /// `&accumulator - &operand`.
    BorrowedSubBorrowed,
    /// `&accumulator + operand`.
    BorrowedAddOwned,
    /// `&accumulator - operand`.
    BorrowedSubOwned,
}

/// Distinguish the forms that leave the operand with the caller.
impl AccumulatorArithmetic {
    /// Every form, for generation.
    const ALL: [Self; 12] = [
        Self::AddAssignBorrowed,
        Self::SubAssignBorrowed,
        Self::AddAssignOwned,
        Self::SubAssignOwned,
        Self::AddBorrowed,
        Self::SubBorrowed,
        Self::AddOwned,
        Self::SubOwned,
        Self::BorrowedAddBorrowed,
        Self::BorrowedSubBorrowed,
        Self::BorrowedAddOwned,
        Self::BorrowedSubOwned,
    ];

    /// Whether the operand is borrowed rather than consumed.
    fn borrows(self) -> bool {
        matches!(
            self,
            Self::AddAssignBorrowed
                | Self::SubAssignBorrowed
                | Self::AddBorrowed
                | Self::SubBorrowed
                | Self::BorrowedAddBorrowed
                | Self::BorrowedSubBorrowed
        )
    }

    /// Whether the form subtracts the operand from the accumulator.
    fn subtracts(self) -> bool {
        matches!(
            self,
            Self::SubAssignBorrowed
                | Self::SubAssignOwned
                | Self::SubBorrowed
                | Self::SubOwned
                | Self::BorrowedSubBorrowed
                | Self::BorrowedSubOwned
        )
    }
}

/// One public-surface instruction in the differential program.
#[derive(Clone, Debug)]
enum Operation {
    /// Primitive arithmetic, retaining the primitive type.
    Primitive {
        /// Assignment or consuming operator.
        arithmetic: PrimitiveArithmetic,
        /// Typed integer operand.
        operand: Primitive,
    },
    /// A shifted limb stream.
    Limbs {
        /// Whether to subtract rather than add.
        subtract: bool,
        /// Little-endian magnitude limbs.
        limbs: Vec<u64>,
        /// Power-of-two scale.
        shift: u64,
    },
    /// Owned or borrowed accumulator arithmetic.
    Accumulator {
        /// Trait form to exercise.
        arithmetic: AccumulatorArithmetic,
        /// Operand value and representation.
        operand: Operand,
    },
    /// A specialized shifted-accumulator update.
    ShiftedAccumulator {
        /// Whether to subtract rather than add.
        subtract: bool,
        /// Operand value and representation.
        operand: Operand,
        /// Power-of-two scale.
        shift: u64,
    },
    /// A left shift in one operator form.
    Shift {
        /// The operator form.
        form: ShiftForm,
        /// Typed shift count.
        shift: Shift,
    },
    /// Unary negation of the owned or a borrowed accumulator.
    Negate {
        /// Whether to negate a borrow, `-&accumulator`.
        borrowed: bool,
    },
    /// Clear to zero while retaining reusable storage.
    Clear,
    /// Reserve storage for a working width in bits, which may be
    /// unsatisfiable, without changing the value.
    Reserve(u64),
    /// Replace the value by a sum of owned or borrowed accumulators.
    Sum {
        /// Whether the iterator yields references.
        borrowed: bool,
        /// Values and representations to sum.
        operands: Vec<Operand>,
    },
    /// Compare with zero in place, keeping the compacted top for later steps.
    Sign,
    /// Replace the value by the magnitude of its difference from an operand,
    /// with either one as the receiver, checking the reported order.
    AbsDiff {
        /// Operand value and representation.
        operand: Operand,
        /// Whether the operand is the receiver, `operand.abs_diff(value)`.
        reversed: bool,
    },
    /// Read the magnitude in place, keeping the normalized form for later
    /// steps.
    ///
    /// The read must leave the value's own working width, at most one digit
    /// wider than before.
    Read {
        /// Whether to use the scaled readout.
        scaled: bool,
    },
}

/// A generated instruction and a width for the stability query after it.
#[derive(Clone, Debug)]
struct Step {
    /// The mutation under test.
    operation: Operation,
    /// Width supplied to `cmp_zero_stable_under` afterward.
    comparison_bits: u64,
    /// An accumulator the result is compared with afterward.
    comparand: Operand,
    /// A primitive the result is compared with afterward.
    primitive: Primitive,
}

/// Apply one instruction to the implementation and oracle.
impl Operation {
    /// Mutate the modeled pair in lockstep.
    fn apply(&self, actual: &mut Accumulator, oracle: &mut BigInt) {
        match self {
            Operation::Primitive {
                arithmetic,
                operand,
            } => {
                operand.check_construction_and_sum();
                operand.apply(*arithmetic, actual, oracle);
            }
            Operation::Limbs {
                subtract,
                limbs,
                shift,
            } => {
                let operand = BigInt::from(from_limbs(limbs)) << *shift as usize;
                if *subtract {
                    actual.sub_shifted_limbs(*shift, limbs.iter().copied());
                    *oracle -= operand;
                } else {
                    actual.add_shifted_limbs(*shift, limbs.iter().copied());
                    *oracle += operand;
                }
            }
            Operation::Accumulator {
                arithmetic,
                operand,
            } => {
                let (mut operand, operand_oracle) = operand.values();
                match arithmetic {
                    AccumulatorArithmetic::AddAssignBorrowed => *actual += &operand,
                    AccumulatorArithmetic::SubAssignBorrowed => *actual -= &operand,
                    AccumulatorArithmetic::AddAssignOwned => {
                        *actual += core::mem::take(&mut operand)
                    }
                    AccumulatorArithmetic::SubAssignOwned => {
                        *actual -= core::mem::take(&mut operand)
                    }
                    AccumulatorArithmetic::AddBorrowed => {
                        *actual = core::mem::take(actual) + &operand;
                    }
                    AccumulatorArithmetic::SubBorrowed => {
                        *actual = core::mem::take(actual) - &operand;
                    }
                    AccumulatorArithmetic::AddOwned => {
                        *actual = core::mem::take(actual) + core::mem::take(&mut operand);
                    }
                    AccumulatorArithmetic::SubOwned => {
                        *actual = core::mem::take(actual) - core::mem::take(&mut operand);
                    }
                    AccumulatorArithmetic::BorrowedAddBorrowed => {
                        replace_borrowed(actual, oracle, |left| left + &operand);
                    }
                    AccumulatorArithmetic::BorrowedSubBorrowed => {
                        replace_borrowed(actual, oracle, |left| left - &operand);
                    }
                    AccumulatorArithmetic::BorrowedAddOwned => {
                        let operand = core::mem::take(&mut operand);
                        replace_borrowed(actual, oracle, |left| left + operand);
                    }
                    AccumulatorArithmetic::BorrowedSubOwned => {
                        let operand = core::mem::take(&mut operand);
                        replace_borrowed(actual, oracle, |left| left - operand);
                    }
                }
                if arithmetic.borrows() {
                    // The operation may compact its operand, but never changes
                    // its value.
                    assert_value(&operand, &operand_oracle);
                }
                if arithmetic.subtracts() {
                    *oracle -= operand_oracle;
                } else {
                    *oracle += operand_oracle;
                }
            }
            Operation::ShiftedAccumulator {
                subtract,
                operand,
                shift,
            } => {
                let (operand, operand_oracle) = operand.values();
                let scaled = &operand_oracle << *shift as usize;
                if *subtract {
                    actual.sub_shifted(*shift, &operand);
                    *oracle -= scaled;
                } else {
                    actual.add_shifted(*shift, &operand);
                    *oracle += scaled;
                }
                // The merge may compact its operand, but never changes its value.
                assert_value(&operand, &operand_oracle);
            }
            Operation::Shift { form, shift } => shift.apply(*form, actual, oracle),
            Operation::Negate { borrowed } => {
                if *borrowed {
                    replace_borrowed(actual, oracle, |value| -value);
                } else {
                    *actual = -core::mem::take(actual);
                }
                *oracle = -core::mem::take(oracle);
            }
            Operation::Clear => {
                actual.clear();
                *oracle = BigInt::ZERO;
            }
            Operation::Reserve(bits) => actual.reserve_bits(*bits),
            Operation::Sum { borrowed, operands } => {
                let (values, oracles): (Vec<_>, Vec<_>) =
                    operands.iter().map(Operand::values).unzip();
                *oracle = oracles.iter().sum();
                if *borrowed {
                    *actual = values.iter().sum();
                    // Summing may compact each operand, but never changes its
                    // value.
                    for (value, value_oracle) in values.iter().zip(&oracles) {
                        assert_value(value, value_oracle);
                    }
                } else {
                    *actual = values.into_iter().sum();
                }
            }
            Operation::Sign => {
                assert_eq!(actual.sign(), oracle_sign(oracle), "in-place sign");
            }
            Operation::AbsDiff { operand, reversed } => {
                let (operand, operand_oracle) = operand.values();
                let value = core::mem::take(actual);
                let (order, gap) = if *reversed {
                    let (order, gap) = operand.abs_diff(value);
                    (order.reverse(), gap)
                } else {
                    value.abs_diff(operand)
                };
                assert_eq!(
                    order,
                    BigInt::cmp(oracle, &operand_oracle),
                    "abs_diff order"
                );
                *actual = gap;
                *oracle = BigInt::from((&*oracle - &operand_oracle).magnitude().clone());
            }
            Operation::Read { scaled } => {
                let previous_digits = actual.stored_digit_count();
                assert_eq!(&read_value(actual, *scaled), oracle, "in-place read");
                assert_eq!(
                    actual.stored_digit_count(),
                    own_digits(oracle),
                    "a read leaves the value's own working width"
                );
                assert!(
                    actual.stored_digit_count() <= previous_digits.saturating_add(1),
                    "a read grows the working width by at most one digit"
                );
            }
        }
    }

    /// A compact trace tag for invariant failure messages.
    fn tag(&self) -> u8 {
        match self {
            Operation::Primitive { .. } => 0,
            Operation::Limbs { .. } => 1,
            Operation::Accumulator { .. } => 2,
            Operation::ShiftedAccumulator { .. } => 3,
            Operation::Shift { .. } => 4,
            Operation::Negate { .. } => 5,
            Operation::Clear => 6,
            Operation::Reserve(_) => 7,
            Operation::Sum { .. } => 8,
            Operation::Read { .. } => 9,
            Operation::Sign => 10,
            Operation::AbsDiff { .. } => 11,
        }
    }
}

/// Check the standard traits and sign queries against the oracle, on stored copies so
/// that observing never changes the tested history.
fn assert_standard_traits(actual: &Accumulator, oracle: &BigInt, step: &Step, trace: &[u8]) {
    let sign = oracle_sign(oracle);
    assert_eq!(
        actual.stored_copy().is_zero(),
        sign == Ordering::Equal,
        "is_zero after {trace:?}"
    );
    assert_eq!(
        actual.stored_copy().is_positive(),
        sign == Ordering::Greater,
        "is_positive after {trace:?}"
    );
    assert_eq!(
        actual.stored_copy().is_negative(),
        sign == Ordering::Less,
        "is_negative after {trace:?}"
    );

    // `num-bigint` pads integers through the same `Formatter` helper.
    for (actual_text, oracle_text) in [
        (actual.stored_copy().to_string(), oracle.to_string()),
        (format!("{:+}", actual.stored_copy()), format!("{oracle:+}")),
        (
            format!("{:>30}", actual.stored_copy()),
            format!("{oracle:>30}"),
        ),
        (
            format!("{:+030}", actual.stored_copy()),
            format!("{oracle:+030}"),
        ),
        (
            format!("{:*^31}", actual.stored_copy()),
            format!("{oracle:*^31}"),
        ),
    ] {
        assert_eq!(actual_text, oracle_text, "formatting after {trace:?}");
    }

    let (comparand, comparand_oracle) = step.comparand.values();
    let expected = oracle.cmp(&comparand_oracle);
    assert_eq!(
        actual.stored_copy().cmp(&comparand),
        expected,
        "ordering after {trace:?}"
    );
    assert_eq!(
        comparand.cmp(&actual.stored_copy()),
        expected.reverse(),
        "reversed ordering after {trace:?}"
    );
    assert_eq!(
        actual.stored_copy() == comparand,
        expected == Ordering::Equal,
        "equality after {trace:?}"
    );
    assert_value(&comparand, &comparand_oracle);

    // The same value in its plain stored form and in flagged digits: all
    // equal, and hashed equally, whatever the flag, zero included.
    let plain = plain_form(oracle);
    let flagged = flagged_form(oracle);
    let hasher = std::collections::hash_map::RandomState::new();
    for (form, name) in [(&plain, "plain"), (&flagged, "flagged")] {
        assert!(
            actual.stored_copy() == *form,
            "equality with the {name} form after {trace:?}"
        );
        assert_eq!(
            hasher.hash_one(actual.stored_copy()),
            hasher.hash_one(form),
            "equal values hash equally, against the {name} form, after {trace:?}"
        );
    }
    assert!(
        plain == flagged,
        "the plain and flagged forms are equal after {trace:?}"
    );

    step.primitive.assert_comparisons(actual, oracle, trace);
}

/// Build the oracle's value in digits whose flag is set, zero included, by
/// depositing its negation into digits and negating the result.
fn flagged_form(oracle: &BigInt) -> Accumulator {
    let mut negation = fresh(true);
    if oracle.sign() == num_bigint::Sign::Minus {
        negation.add_shifted_limbs(0, oracle.magnitude().iter_u64_digits());
    } else {
        negation.sub_shifted_limbs(0, oracle.magnitude().iter_u64_digits());
    }
    let flagged = -negation;
    assert!(
        flagged.stored().digits.is_negated(),
        "the flagged form holds its value in flagged digits"
    );
    flagged
}

/// Build the oracle's value from its limbs, a stored form without cancellation.
fn plain_form(oracle: &BigInt) -> Accumulator {
    let mut plain = Accumulator::new();
    if oracle.sign() == num_bigint::Sign::Minus {
        plain.sub_shifted_limbs(0, oracle.magnitude().iter_u64_digits());
    } else {
        plain.add_shifted_limbs(0, oracle.magnitude().iter_u64_digits());
    }
    plain
}

/// Check every public observation against the exact modeled value, on
/// stored copies so that observing never changes the tested history.
fn assert_observations(actual: &Accumulator, oracle: &BigInt, comparison_bits: u64, trace: &[u8]) {
    assert_value(actual, oracle);
    assert_value(&actual.stored_copy(), oracle);

    if actual.is_known_zero() {
        assert_eq!(oracle, &BigInt::ZERO, "known zero after {trace:?}");
    }

    let stored_digits = actual.stored_digit_count();
    assert!(stored_digits >= 1, "stored width after {trace:?}");
    assert_eq!(
        actual.stored_bits(),
        u64::try_from(stored_digits).unwrap() * 32,
        "stored bit width after {trace:?}"
    );
    assert_stored_width_bounds(actual, oracle, trace);

    if let Some(reported) = actual.stored_copy().cmp_zero_stable_under(comparison_bits) {
        assert_eq!(
            reported,
            oracle_sign(oracle),
            "stable order after {trace:?}"
        );
        let largest_adjustment = if comparison_bits == 0 {
            BigUint::ZERO
        } else {
            (BigUint::from(1u8) << comparison_bits as usize) - 1u8
        };
        assert!(
            oracle.magnitude() > &largest_adjustment,
            "stable comparison dominates every requested adjustment after {trace:?}"
        );
    }

    let compared = actual.stored_copy();
    assert_eq!(
        compared.sign(),
        oracle_sign(oracle),
        "exact order after {trace:?}"
    );
    if oracle == &BigInt::ZERO {
        assert!(
            compared.is_known_zero(),
            "an exact zero comparison compacts cancellation after {trace:?}"
        );
    }

    macro_rules! check_conversion {
        ($integer:ty) => {
            match (
                <$integer>::try_from(oracle),
                <$integer>::try_from(actual.stored_copy()),
            ) {
                (Ok(expected), Ok(found)) => assert_eq!(found, expected, "conversion after {trace:?}"),
                (Err(_), Err(rejected)) => assert_value(&rejected, oracle),
                (expected, found) => panic!(
                    "conversion disagreement after {trace:?}: expected {expected:?}, got {found:?}"
                ),
            }
        };
    }
    check_conversion!(i8);
    check_conversion!(i16);
    check_conversion!(i32);
    check_conversion!(i64);
    check_conversion!(i128);
    check_conversion!(isize);
    check_conversion!(u8);
    check_conversion!(u16);
    check_conversion!(u32);
    check_conversion!(u64);
    check_conversion!(u128);
    check_conversion!(usize);

    actual.stored().assert_invariants(trace);
}

/// Bias one primitive type toward its endpoints while retaining arbitrary values.
macro_rules! primitive_strategy {
    ($integer:ty, $variant:path) => {
        prop_oneof![
            Just(<$integer>::MIN),
            Just(<$integer>::MIN.saturating_add(1)),
            Just(0 as $integer),
            Just(1 as $integer),
            Just(<$integer>::MAX.saturating_sub(1)),
            Just(<$integer>::MAX),
            any::<$integer>(),
        ]
        .prop_map($variant)
    };
}

/// Generate every primitive operand type around its exact range boundaries.
fn arb_primitive() -> BoxedStrategy<Primitive> {
    prop_oneof![
        primitive_strategy!(i8, Primitive::I8),
        primitive_strategy!(i16, Primitive::I16),
        primitive_strategy!(i32, Primitive::I32),
        primitive_strategy!(i64, Primitive::I64),
        primitive_strategy!(i128, Primitive::I128),
        primitive_strategy!(isize, Primitive::Isize),
        primitive_strategy!(u8, Primitive::U8),
        primitive_strategy!(u16, Primitive::U16),
        primitive_strategy!(u32, Primitive::U32),
        primitive_strategy!(u64, Primitive::U64),
        primitive_strategy!(u128, Primitive::U128),
        primitive_strategy!(usize, Primitive::Usize),
    ]
    .boxed()
}

/// Generate shifts at and between every representation boundary.
fn arb_shift_value(maximum: u64) -> BoxedStrategy<u64> {
    arb_shift_value_in(0, maximum)
}

/// Generate shifts in `minimum..=maximum`, at and between every
/// representation boundary in that range.
///
/// Bounding the range here, rather than filtering [`arb_shift_value`],
/// rejects no draws, so no case count exhausts proptest's reject limit.
fn arb_shift_value_in(minimum: u64, maximum: u64) -> BoxedStrategy<u64> {
    let boundaries: Vec<u64> = [
        0, 1, 29, 30, 31, 32, 33, 63, 64, 65, 95, 96, 97, 127, 128, 129, 191, 192, 193, 255, 256,
        257, 511, 512, 513, 1024,
    ]
    .into_iter()
    .filter(|value| (minimum..=maximum).contains(value))
    .collect();
    prop_oneof![3 => proptest::sample::select(boundaries), 1 => minimum..=maximum].boxed()
}

/// Generate every primitive shift-count type with valid nonnegative values.
fn arb_shift() -> BoxedStrategy<Shift> {
    prop_oneof![
        arb_shift_value(i8::MAX as u64).prop_map(|value| Shift::I8(value as i8)),
        arb_shift_value(1024).prop_map(|value| Shift::I16(value as i16)),
        arb_shift_value(1024).prop_map(|value| Shift::I32(value as i32)),
        arb_shift_value(1024).prop_map(|value| Shift::I64(value as i64)),
        arb_shift_value(1024).prop_map(|value| Shift::I128(value as i128)),
        arb_shift_value(1024).prop_map(|value| Shift::Isize(value as isize)),
        arb_shift_value(u8::MAX as u64).prop_map(|value| Shift::U8(value as u8)),
        arb_shift_value(1024).prop_map(|value| Shift::U16(value as u16)),
        arb_shift_value(1024).prop_map(|value| Shift::U32(value as u32)),
        arb_shift_value(1024).prop_map(Shift::U64),
        arb_shift_value(1024).prop_map(|value| Shift::U128(u128::from(value))),
        arb_shift_value(1024).prop_map(|value| Shift::Usize(value as usize)),
    ]
    .boxed()
}

/// Generate limb streams with empty, padded, boundary, and arbitrary forms.
fn arb_limbs() -> BoxedStrategy<Vec<u64>> {
    prop_oneof![
        Just(Vec::new()),
        Just(vec![0]),
        Just(vec![1]),
        Just(vec![(1 << 31) - 1]),
        Just(vec![1 << 31]),
        Just(vec![u64::from(u32::MAX)]),
        Just(vec![1u64 << 32]),
        Just(vec![u64::MAX]),
        Just(vec![u64::MAX, u64::MAX]),
        (proptest::collection::vec(any::<u64>(), 0..=8), 0usize..=2).prop_map(
            |(mut limbs, high_zeros)| {
                limbs.extend(core::iter::repeat_n(0, high_zeros));
                limbs
            }
        ),
    ]
    .boxed()
}

/// Generate operands in both storage modes and around each transition, each
/// one negated or not.
fn arb_operand() -> BoxedStrategy<Operand> {
    prop_oneof![
        3 => arb_unnegated_operand(),
        1 => arb_unnegated_operand().prop_map(|operand| Operand::Negated(Box::new(operand))),
    ]
    .boxed()
}

/// Generate operands in both storage modes and around each transition,
/// without a negation.
fn arb_unnegated_operand() -> BoxedStrategy<Operand> {
    prop_oneof![
        3 => arb_primitive().prop_map(Operand::Primitive),
        3 => (any::<bool>(), arb_limbs(), arb_shift_value(1024)).prop_map(
            |(negative, limbs, shift)| Operand::ShiftedLimbs {
                negative,
                limbs,
                shift,
            }
        ),
        2 => (-2i8..=2, any::<bool>()).prop_map(|(displacement, negative)| {
            Operand::SmallBoundary {
                displacement,
                negative,
            }
        }),
        2 => (arb_shift_value_in(96, 1024), arb_primitive())
            .prop_map(|(shift, tail)| Operand::CancelledHigh { shift, tail }),
        2 => (1u64..=40, arb_primitive())
            .prop_map(|(position, tail)| Operand::CancelledTop { position, tail }),
    ]
    .boxed()
}

/// Generate all arithmetic instructions over the public API.
fn arb_operation() -> BoxedStrategy<Operation> {
    prop_oneof![
        5 => (proptest::sample::select(&PrimitiveArithmetic::ALL[..]), arb_primitive())
            .prop_map(|(arithmetic, operand)| Operation::Primitive { arithmetic, operand }),
        4 => (any::<bool>(), arb_limbs(), arb_shift_value(1024)).prop_map(
            |(subtract, limbs, shift)| Operation::Limbs {
                subtract,
                limbs,
                shift,
            }
        ),
        5 => (proptest::sample::select(&AccumulatorArithmetic::ALL[..]), arb_operand())
            .prop_map(|(arithmetic, operand)| Operation::Accumulator { arithmetic, operand }),
        3 => (any::<bool>(), arb_operand(), arb_shift_value(1024)).prop_map(
            |(subtract, operand, shift)| Operation::ShiftedAccumulator {
                subtract,
                operand,
                shift,
            }
        ),
        2 => (proptest::sample::select(&ShiftForm::ALL[..]), arb_shift())
            .prop_map(|(form, shift)| Operation::Shift { form, shift }),
        1 => any::<bool>().prop_map(|borrowed| Operation::Negate { borrowed }),
        1 => Just(Operation::Clear),
        1 => prop_oneof![
            0u64..=4_096,
            // At least 2^60 bits needs at least 2^58 bytes of storage, more
            // than any 64-bit platform can address, so the hint is ignored.
            (1u64 << 60)..=u64::MAX,
        ]
        .prop_map(Operation::Reserve),
        2 => (any::<bool>(), proptest::collection::vec(arb_operand(), 0..=6)).prop_map(
            |(borrowed, operands)| Operation::Sum { borrowed, operands }
        ),
        2 => any::<bool>().prop_map(|scaled| Operation::Read { scaled }),
        1 => Just(Operation::Sign),
        2 => (arb_operand(), any::<bool>())
            .prop_map(|(operand, reversed)| Operation::AbsDiff { operand, reversed }),
    ]
    .boxed()
}

/// Generate post-operation comparison widths at the same phase boundaries.
fn arb_step() -> BoxedStrategy<Step> {
    (
        arb_operation(),
        arb_shift_value(1024),
        arb_operand(),
        arb_primitive(),
    )
        .prop_map(|(operation, comparison_bits, comparand, primitive)| Step {
            operation,
            comparison_bits,
            comparand,
            primitive,
        })
        .boxed()
}

proptest! {
    /// Every public arithmetic operation and observation agrees with an
    /// independent arbitrary-precision integer after every generated step.
    ///
    /// Every operation that borrows an accumulator operand leaves its value
    /// unchanged. The observations include formatting, the sign queries,
    /// comparison and equality with another accumulator and with a primitive,
    /// and equal hashes for equal values in different stored forms, with the
    /// digits' flag set or clear.
    #[test]
    fn complete_surface_matches_bigint(
        steps in proptest::collection::vec(arb_step(), 1..=80),
    ) {
        let mut actual = Accumulator::default();
        let mut oracle = BigInt::ZERO;
        let mut trace = Vec::with_capacity(steps.len());
        assert_observations(&actual, &oracle, 0, &trace);

        for step in steps {
            step.operation.apply(&mut actual, &mut oracle);
            trace.push(step.operation.tag());
            assert_standard_traits(&actual, &oracle, &step, &trace);
            assert_observations(&actual, &oracle, step.comparison_bits, &trace);
        }
    }
}
