//! Differential coverage of the complete public arithmetic surface.
//!
//! One generated program updates an [`Accumulator`] and an independent
//! [`BigInt`] in lockstep. After every instruction, the observer checks every
//! public query and primitive conversion. Instructions cover the standard
//! arithmetic traits, shifted limb and accumulator inputs, sums, reservation,
//! reset, normalization, and both owned and borrowed operands.
//!
//! The generators put explicit weight on the representation's boundaries:
//! 30-bit small-path shifts, 32-bit digits, 64-bit limbs, the `2^96` small
//! limit, cancellation across sparse high positions, and every primitive
//! integer endpoint. Random values and shifts between those points exercise
//! their neighborhoods rather than only the named examples.

use num_bigint::{BigInt, BigUint};
use proptest::prelude::*;

use super::{assert_value, from_limbs, oracle_sign, Accumulator, TestBig as _};

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

/// The four arithmetic forms supported for every primitive.
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
                }
                let value = BigInt::from(value);
                match operation {
                    PrimitiveArithmetic::AddAssign | PrimitiveArithmetic::Add => *oracle += value,
                    PrimitiveArithmetic::SubAssign | PrimitiveArithmetic::Subtract => {
                        *oracle -= value;
                    }
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

    /// Check `From` and the type-specific `Sum` implementation.
    fn check_construction_and_sum(&self) {
        macro_rules! check {
            ($value:expr) => {{
                let value = *$value;
                let oracle = BigInt::from(value);
                assert_value(&Accumulator::from(value), &oracle);
                let sum: Accumulator = [value, value].into_iter().sum();
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

/// Exercise either the assignment or consuming shift operator.
impl Shift {
    /// Multiply both values by this power of two.
    fn apply(&self, owned: bool, actual: &mut Accumulator, oracle: &mut BigInt) {
        macro_rules! shift {
            ($count:expr) => {{
                let count = *$count;
                if owned {
                    *actual = core::mem::take(actual) << count;
                } else {
                    *actual <<= count;
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
}

/// Build operand values exclusively through the public arithmetic surface.
impl Operand {
    /// Return matching accumulator and big-integer representations.
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
    /// Assignment or consuming left shift.
    Shift {
        /// Whether to use the consuming operator.
        owned: bool,
        /// Typed shift count.
        shift: Shift,
    },
    /// Unary negation.
    Negate,
    /// Reset to zero while retaining reusable storage.
    Reset,
    /// Normalize the working form without changing the value.
    Normalize,
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
}

/// A generated instruction and a width for the stability query after it.
#[derive(Clone, Debug)]
struct Step {
    /// The mutation under test.
    operation: Operation,
    /// Width supplied to `cmp_zero_stable_under` afterward.
    comparison_bits: u64,
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
                let (operand, operand_oracle) = operand.values();
                match arithmetic {
                    AccumulatorArithmetic::AddAssignBorrowed => *actual += &operand,
                    AccumulatorArithmetic::SubAssignBorrowed => *actual -= &operand,
                    AccumulatorArithmetic::AddAssignOwned => *actual += operand,
                    AccumulatorArithmetic::SubAssignOwned => *actual -= operand,
                    AccumulatorArithmetic::AddBorrowed => {
                        *actual = core::mem::take(actual) + &operand;
                    }
                    AccumulatorArithmetic::SubBorrowed => {
                        *actual = core::mem::take(actual) - &operand;
                    }
                    AccumulatorArithmetic::AddOwned => {
                        *actual = core::mem::take(actual) + operand;
                    }
                    AccumulatorArithmetic::SubOwned => {
                        *actual = core::mem::take(actual) - operand;
                    }
                }
                match arithmetic {
                    AccumulatorArithmetic::AddAssignBorrowed
                    | AccumulatorArithmetic::AddAssignOwned
                    | AccumulatorArithmetic::AddBorrowed
                    | AccumulatorArithmetic::AddOwned => *oracle += operand_oracle,
                    AccumulatorArithmetic::SubAssignBorrowed
                    | AccumulatorArithmetic::SubAssignOwned
                    | AccumulatorArithmetic::SubBorrowed
                    | AccumulatorArithmetic::SubOwned => *oracle -= operand_oracle,
                }
            }
            Operation::ShiftedAccumulator {
                subtract,
                operand,
                shift,
            } => {
                let (mut operand, operand_oracle) = operand.values();
                let scaled = &operand_oracle << *shift as usize;
                if *subtract {
                    actual.sub_shifted(*shift, &mut operand);
                    *oracle -= scaled;
                } else {
                    actual.add_shifted(*shift, &mut operand);
                    *oracle += scaled;
                }
                // The merge may compact its operand, but never changes its value.
                assert_value(&operand, &operand_oracle);
            }
            Operation::Shift { owned, shift } => shift.apply(*owned, actual, oracle),
            Operation::Negate => {
                *actual = -core::mem::take(actual);
                *oracle = -core::mem::take(oracle);
            }
            Operation::Reset => {
                actual.reset();
                *oracle = BigInt::ZERO;
            }
            Operation::Normalize => {
                let previous_digits = actual.stored_digit_count();
                actual.normalize();
                let expected_digits = usize::try_from(oracle.magnitude().bits())
                    .expect("generated values have addressable widths")
                    .div_ceil(32)
                    .max(1);
                assert_eq!(
                    actual.stored_digit_count(),
                    expected_digits,
                    "normalization leaves the exact magnitude width"
                );
                assert!(
                    actual.stored_digit_count() <= previous_digits.saturating_add(1),
                    "normalization grows by at most one digit"
                );
            }
            Operation::Reserve(bits) => actual.reserve_bits(*bits),
            Operation::Sum { borrowed, operands } => {
                let (values, oracles): (Vec<_>, Vec<_>) =
                    operands.iter().map(Operand::values).unzip();
                *oracle = oracles.into_iter().sum();
                *actual = if *borrowed {
                    values.iter().sum()
                } else {
                    values.into_iter().sum()
                };
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
            Operation::Negate => 5,
            Operation::Reset => 6,
            Operation::Normalize => 7,
            Operation::Reserve(_) => 8,
            Operation::Sum { .. } => 9,
        }
    }
}

/// Check every public observation against the exact modeled value.
fn assert_observations(
    actual: &mut Accumulator,
    oracle: &BigInt,
    comparison_bits: u64,
    trace: &[u8],
) {
    assert_value(actual, oracle);
    assert_value(&actual.clone(), oracle);

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
    let (_, magnitude) = actual.sign_biguint();
    assert!(
        magnitude.bits() <= actual.stored_bits() + 2,
        "stored width bounds the redundant value after {trace:?}"
    );

    if let Some(reported) = actual.cmp_zero_stable_under(comparison_bits) {
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

    assert_eq!(
        actual.cmp_zero(),
        oracle_sign(oracle),
        "exact order after {trace:?}"
    );
    if oracle == &BigInt::ZERO {
        assert!(
            actual.is_known_zero(),
            "an exact zero comparison compacts cancellation after {trace:?}"
        );
    }

    macro_rules! check_conversion {
        ($integer:ty) => {
            match (
                <$integer>::try_from(oracle),
                <$integer>::try_from(actual.clone()),
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

    actual
        .digits
        .assert_invariants(actual.small.is_some(), trace);
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
    let boundaries: Vec<u64> = [
        0, 1, 29, 30, 31, 32, 33, 63, 64, 65, 95, 96, 97, 127, 128, 129, 191, 192, 193, 255, 256,
        257, 511, 512, 513, 1024,
    ]
    .into_iter()
    .filter(|&value| value <= maximum)
    .collect();
    prop_oneof![3 => proptest::sample::select(boundaries), 1 => 0..=maximum].boxed()
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

/// Generate operands in both storage modes and around each transition.
fn arb_operand() -> BoxedStrategy<Operand> {
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
        2 => (arb_shift_value(1024).prop_filter("high cancellation", |&shift| shift >= 96), arb_primitive())
            .prop_map(|(shift, tail)| Operand::CancelledHigh { shift, tail }),
    ]
    .boxed()
}

/// Generate all arithmetic instructions over the public API.
fn arb_operation() -> BoxedStrategy<Operation> {
    prop_oneof![
        5 => (0u8..4, arb_primitive()).prop_map(|(operation, operand)| {
            let arithmetic = match operation {
                0 => PrimitiveArithmetic::AddAssign,
                1 => PrimitiveArithmetic::SubAssign,
                2 => PrimitiveArithmetic::Add,
                _ => PrimitiveArithmetic::Subtract,
            };
            Operation::Primitive { arithmetic, operand }
        }),
        4 => (any::<bool>(), arb_limbs(), arb_shift_value(1024)).prop_map(
            |(subtract, limbs, shift)| Operation::Limbs {
                subtract,
                limbs,
                shift,
            }
        ),
        5 => (0u8..8, arb_operand()).prop_map(|(operation, operand)| {
            let arithmetic = match operation {
                0 => AccumulatorArithmetic::AddAssignBorrowed,
                1 => AccumulatorArithmetic::SubAssignBorrowed,
                2 => AccumulatorArithmetic::AddAssignOwned,
                3 => AccumulatorArithmetic::SubAssignOwned,
                4 => AccumulatorArithmetic::AddBorrowed,
                5 => AccumulatorArithmetic::SubBorrowed,
                6 => AccumulatorArithmetic::AddOwned,
                _ => AccumulatorArithmetic::SubOwned,
            };
            Operation::Accumulator { arithmetic, operand }
        }),
        3 => (any::<bool>(), arb_operand(), arb_shift_value(1024)).prop_map(
            |(subtract, operand, shift)| Operation::ShiftedAccumulator {
                subtract,
                operand,
                shift,
            }
        ),
        2 => (any::<bool>(), arb_shift()).prop_map(|(owned, shift)| Operation::Shift {
            owned,
            shift,
        }),
        1 => Just(Operation::Negate),
        1 => Just(Operation::Reset),
        1 => Just(Operation::Normalize),
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
    ]
    .boxed()
}

/// Generate post-operation comparison widths at the same phase boundaries.
fn arb_step() -> BoxedStrategy<Step> {
    (arb_operation(), arb_shift_value(1024))
        .prop_map(|(operation, comparison_bits)| Step {
            operation,
            comparison_bits,
        })
        .boxed()
}

proptest! {
    /// Every public arithmetic operation and observation agrees with an
    /// independent arbitrary-precision integer after every generated step, and
    /// a shifted accumulator merge leaves its operand's value unchanged.
    #[test]
    fn complete_surface_matches_bigint(
        steps in proptest::collection::vec(arb_step(), 1..=80),
    ) {
        let mut actual = Accumulator::default();
        let mut oracle = BigInt::ZERO;
        let mut trace = Vec::with_capacity(steps.len());
        assert_observations(&mut actual, &oracle, 0, &trace);

        for step in steps {
            step.operation.apply(&mut actual, &mut oracle);
            trace.push(step.operation.tag());
            assert_observations(
                &mut actual,
                &oracle,
                step.comparison_bits,
                &trace,
            );
        }
    }
}
