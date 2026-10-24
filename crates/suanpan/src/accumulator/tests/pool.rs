//! Programs over a pool of accumulators, each checked against an exact oracle.
//!
//! The surface property builds every accumulator operand fresh from a short
//! recipe, so an operand carries no history beyond its construction. Here
//! every operand is a member of a small pool, so it arrives carrying whatever its own history
//! left: retained cancellation, a sparse written set, a compacted or
//! normalized top, extreme digits, or a width of thousands of digits. A
//! program updates the members, lends them to one another as operands, passes
//! one member as both operands of `&a + &a`, and reads members in place, so a
//! read changes the stored form that later steps meet, as it would in a
//! caller's program.
//!
//! Each member is paired with a `BigInt` oracle. After every step, every
//! member is observed without being changed: both limb readouts, the
//! representation invariants, known-zero soundness, and the bounds its
//! working width places on its value. A step that asks a question (a sign, a
//! comparison, a stability query, an in-place read, a conversion) checks the
//! answer where it asks. Nothing else compares a member with zero, so
//! cancellation builds up across steps until the program itself reads it.
//!
//! Each case enables a random subset of step kinds with random weights, which
//! is swarm testing (Groce et al., ISSTA 2012): some programs grow wide values
//! with no clear or compacting read among them, while others repeat one
//! interaction many times.

use core::cmp::Ordering;
use core::hash::BuildHasher;
use std::collections::hash_map::RandomState;

use num_bigint::{BigInt as IBig, BigUint as UBig};
use proptest::prelude::*;

use super::{
    assert_stored_width_bounds, assert_value, fresh, from_limbs, oracle_sign, own_digits,
    park_extreme_negative_digit, read_value, Accumulator, LARGEST_DIGIT,
};

/// The number of accumulators in the pool.
///
/// Three let one step involve a receiver, an operand, and a member the step
/// leaves alone.
const POOL: usize = 3;

/// The most steps a program holds.
const MAX_STEPS: usize = 119;

/// How a pool member supplies an accumulator operand to a receiver.
///
/// When one member is both receiver and operand, the borrowed form passes it
/// twice, as `&member ± &member`, the one spelling of addition or subtraction
/// that reads the same accumulator as both operands. The other forms take a
/// clone of it as the operand, since they borrow the receiver mutably or
/// consume it.
#[derive(Clone, Copy, Debug)]
enum OperandForm {
    /// `receiver += &operand` or `receiver -= &operand`.
    Borrowed,
    /// `add_shifted` or `sub_shifted`, reading the operand at a scale.
    Shifted,
    /// `receiver ± operand.clone()`, consuming the receiver. The clone
    /// normalizes the operand in place first.
    OwnedClone,
    /// `receiver ± take(operand)`, consuming both, so the operand's member
    /// restarts as a new accumulator. A member that is its own operand
    /// passes a clone instead.
    OwnedTake,
    /// `&operand ± receiver`, consuming the receiver as the right operand.
    Reversed,
}

/// How an in-place read normalizes a member.
#[derive(Clone, Copy, Debug)]
enum ReadForm {
    /// `signed_magnitude`.
    Signed,
    /// `scaled_signed_magnitude`.
    Scaled,
    /// `clone`, which normalizes its source in place before copying.
    Clone,
}

/// One instruction of a program over the pool.
///
/// Every `usize` field indexes a pool member. A step's receiver and the
/// member it reads may be the same member.
#[derive(Clone, Debug)]
enum Step {
    /// `+=` a signed machine word.
    Word {
        /// The member updated.
        member: usize,
        /// The word added.
        value: i64,
    },
    /// `+=` or `-=` an unsigned machine word.
    UnsignedWord {
        /// The member updated.
        member: usize,
        /// Whether to subtract rather than add.
        subtract: bool,
        /// The word applied.
        value: u64,
    },
    /// `+=` a signed 128-bit value.
    Wide {
        /// The member updated.
        member: usize,
        /// The value added.
        value: i128,
    },
    /// `+=` or `-=` an unsigned 128-bit value.
    UnsignedWide {
        /// The member updated.
        member: usize,
        /// Whether to subtract rather than add.
        subtract: bool,
        /// The value applied.
        value: u128,
    },
    /// `add_shifted_limbs` or `sub_shifted_limbs`.
    Limbs {
        /// The member updated.
        member: usize,
        /// Whether to subtract rather than add.
        subtract: bool,
        /// Little-endian magnitude limbs, possibly empty or with high zeros.
        limbs: Vec<u64>,
        /// Power-of-two scale.
        shift: u64,
    },
    /// Add or subtract a member, another or the receiver itself.
    Accumulator {
        /// The member updated.
        receiver: usize,
        /// The member supplying the operand.
        operand: usize,
        /// Whether to subtract rather than add.
        subtract: bool,
        /// The scale of the [`Shifted`](OperandForm::Shifted) form, ignored
        /// by the others.
        shift: u64,
        /// How the operand is supplied.
        form: OperandForm,
    },
    /// Subtract `(2^33 − 1) · 2^(32 · index)` as two word deposits at one
    /// digit, which leaves the extreme coefficient `−(2^33 − 1)` there when
    /// the digit was zero. Negation makes the positive extreme.
    ParkExtreme {
        /// The member updated.
        member: usize,
        /// The digit position deposited at.
        index: u64,
    },
    /// `<<=` by a count.
    Shift {
        /// The member shifted.
        member: usize,
        /// The shift count in bits.
        shift: u64,
    },
    /// Negate the owned value, or replace it by `-&member`.
    Negate {
        /// The member negated.
        member: usize,
        /// Whether to negate a borrow, which clones first.
        borrowed: bool,
    },
    /// `clear`.
    Clear {
        /// The member cleared.
        member: usize,
    },
    /// Read in place, keeping the normalized form for later steps.
    Read {
        /// The member read.
        member: usize,
        /// The reading operation.
        form: ReadForm,
    },
    /// `sign` in place.
    Sign {
        /// The member asked.
        member: usize,
    },
    /// `cmp_zero_stable_under` in place, at a generated width.
    Stable {
        /// The member asked.
        member: usize,
        /// The requested width.
        bits: u64,
    },
    /// `cmp_zero_stable_under` in place, at a member's `stored_bits`.
    StableAgainst {
        /// The member asked.
        member: usize,
        /// The member whose working width is requested.
        operand: usize,
    },
    /// Compare two members in place, by `cmp`, `partial_cmp`, and `==`.
    Compare {
        /// The left operand of `cmp` and `==`.
        left: usize,
        /// The right operand of `cmp` and `==`.
        right: usize,
    },
    /// `reserve_bits`, which may be unsatisfiable.
    Reserve {
        /// The member reserving.
        member: usize,
        /// The requested working width.
        bits: u64,
    },
    /// Convert stored copies to every primitive integer type.
    Convert {
        /// The member converted.
        member: usize,
    },
    /// `receiver.clone_from(&source)`, or `receiver = receiver.clone()`.
    CloneFrom {
        /// The member overwritten.
        receiver: usize,
        /// The member copied.
        source: usize,
    },
    /// Replace the receiver by a sum of members.
    Sum {
        /// The member replaced.
        receiver: usize,
        /// The members summed, in order, repeats allowed.
        members: Vec<usize>,
        /// Whether to sum clones rather than borrows.
        cloned: bool,
    },
    /// Replace the receiver by the magnitude of its difference from an
    /// operand, checking the reported order.
    AbsDiff {
        /// The member replaced.
        receiver: usize,
        /// The member subtracted.
        operand: usize,
        /// Whether to take the operand, leaving its member new, rather than
        /// clone it; a receiver that is its own operand is always cloned.
        take: bool,
    },
    /// A value-neutral round of single-digit writes: `1` at `low`, if any,
    /// then at `top` and `inner`, then `−1` at `top`, `inner`, and `low`.
    ///
    /// Each write lands above or beside a gap of unwritten positions, and
    /// each cancellation of the top makes it descend across one. Beneath a
    /// wider value the positions stay written, so the written set becomes
    /// sparse.
    GapRound {
        /// The member written.
        member: usize,
        /// The lowest digit position, below `inner`, if any.
        low: Option<u64>,
        /// A digit position strictly between `low` and `top`.
        inner: u64,
        /// The highest digit position.
        top: u64,
    },
}

/// Declare [`Kind`] and [`Kind::ALL`] from one list of kinds.
///
/// Generating both from the list makes `ALL` name every kind exactly once,
/// in declaration order, which is also the order of the discriminants that
/// index a case's swarm weights.
macro_rules! kinds {
    ($($(#[$doc:meta])* $kind:ident,)*) => {
        /// The kinds of [`Step`], in the order of a case's swarm weights.
        #[derive(Clone, Copy, Debug)]
        enum Kind {
            $($(#[$doc])* $kind,)*
        }

        impl Kind {
            /// Every kind, in declaration order.
            const ALL: &[Kind] = &[$(Kind::$kind,)*];
        }
    };
}

kinds! {
    /// [`Step::Word`].
    Word,
    /// [`Step::UnsignedWord`].
    UnsignedWord,
    /// [`Step::Wide`].
    Wide,
    /// [`Step::UnsignedWide`].
    UnsignedWide,
    /// [`Step::Limbs`].
    Limbs,
    /// [`Step::Accumulator`].
    Accumulator,
    /// [`Step::ParkExtreme`].
    ParkExtreme,
    /// [`Step::Shift`].
    Shift,
    /// [`Step::Negate`].
    Negate,
    /// [`Step::Clear`].
    Clear,
    /// [`Step::Read`].
    Read,
    /// [`Step::Sign`].
    Sign,
    /// [`Step::Stable`].
    Stable,
    /// [`Step::StableAgainst`].
    StableAgainst,
    /// [`Step::Compare`].
    Compare,
    /// [`Step::Reserve`].
    Reserve,
    /// [`Step::Convert`].
    Convert,
    /// [`Step::CloneFrom`].
    CloneFrom,
    /// [`Step::Sum`].
    Sum,
    /// [`Step::AbsDiff`].
    AbsDiff,
    /// [`Step::GapRound`].
    GapRound,
}

impl Kind {
    /// The kinds that can write a nonzero value into a pool of zeros.
    ///
    /// Every case enables at least one, so every case can write a nonzero
    /// value.
    const WRITERS: [Kind; 6] = [
        Kind::Word,
        Kind::UnsignedWord,
        Kind::Wide,
        Kind::UnsignedWide,
        Kind::Limbs,
        Kind::ParkExtreme,
    ];
}

impl Step {
    /// The step's kind, which also tags it in invariant failure messages.
    fn kind(&self) -> Kind {
        match self {
            Step::Word { .. } => Kind::Word,
            Step::UnsignedWord { .. } => Kind::UnsignedWord,
            Step::Wide { .. } => Kind::Wide,
            Step::UnsignedWide { .. } => Kind::UnsignedWide,
            Step::Limbs { .. } => Kind::Limbs,
            Step::Accumulator { .. } => Kind::Accumulator,
            Step::ParkExtreme { .. } => Kind::ParkExtreme,
            Step::Shift { .. } => Kind::Shift,
            Step::Negate { .. } => Kind::Negate,
            Step::Clear { .. } => Kind::Clear,
            Step::Read { .. } => Kind::Read,
            Step::Sign { .. } => Kind::Sign,
            Step::Stable { .. } => Kind::Stable,
            Step::StableAgainst { .. } => Kind::StableAgainst,
            Step::Compare { .. } => Kind::Compare,
            Step::Reserve { .. } => Kind::Reserve,
            Step::Convert { .. } => Kind::Convert,
            Step::CloneFrom { .. } => Kind::CloneFrom,
            Step::Sum { .. } => Kind::Sum,
            Step::AbsDiff { .. } => Kind::AbsDiff,
            Step::GapRound { .. } => Kind::GapRound,
        }
    }
}

/// A generated program: each member's starting representation, then steps.
#[derive(Clone, Debug)]
struct Program {
    /// Whether each member starts in the digit representation rather than
    /// the small one.
    digits: [bool; POOL],
    /// The instructions, applied in order.
    steps: Vec<Step>,
}

/// The accumulators and their oracles, kept in lockstep.
struct Pool {
    /// The accumulators under test.
    members: [Accumulator; POOL],
    /// Each member's exact value.
    oracles: [IBig; POOL],
}

/// Borrow two different members mutably.
///
/// # Panics
///
/// Panics if `first == second`.
fn two_members(
    members: &mut [Accumulator; POOL],
    first: usize,
    second: usize,
) -> (&mut Accumulator, &mut Accumulator) {
    assert_ne!(first, second, "two members must differ to borrow both");
    let (low, high) = members.split_at_mut(first.max(second));
    if first < second {
        (&mut low[first], &mut high[0])
    } else {
        (&mut high[0], &mut low[second])
    }
}

/// The digit count of every accumulator whose `stored_bits` reports `bits`,
/// if any can report it.
///
/// The working width is a whole number of 32-bit digits, at least one
/// (`Value::stored_bits`), so the widths an accumulator can report are the
/// positive multiples of 32.
fn reportable_digits(bits: u64) -> Option<u64> {
    (bits > 0 && bits.is_multiple_of(32)).then_some(bits / 32)
}

/// The largest magnitude `count` stored digits can hold: every digit at
/// [`LARGEST_DIGIT`].
fn max_magnitude(count: usize) -> UBig {
    // A base-2^32 repunit times the digit limit is
    // (2^33 − 1) · Σ_{i < count} 2^(32 i).
    UBig::new(vec![1; count]) * LARGEST_DIGIT
}

/// Check one answer of `cmp_zero_stable_under(bits)` against its contract.
///
/// `Some(order)` is the value's sign, never `Equal`, and promises that every
/// integer of magnitude below `2^bits` leaves that order unchanged. Adding
/// the value's own negation would make it zero, so the promise holds exactly
/// when the magnitude is at least `2^bits`. When `bits` is a width that some
/// accumulator's `stored_bits` reports, the promise extends to adding or
/// subtracting any such accumulator, so the magnitude must exceed the
/// largest value of that width.
///
/// `None` claims nothing about the order. When `bits` is such a width, it
/// leaves this accumulator's working width at most 64 bits wider.
fn check_stable(
    answer: Option<Ordering>,
    bits: u64,
    member: &Accumulator,
    oracle: &IBig,
    trace: &[u8],
) {
    match answer {
        Some(order) => {
            assert_eq!(
                order,
                oracle_sign(oracle),
                "a stable answer is the value's sign after {trace:?}"
            );
            assert_ne!(
                order,
                Ordering::Equal,
                "a stable answer is never Equal after {trace:?}"
            );
            assert!(
                oracle.magnitude().bits() > bits,
                "a stable answer at {bits} bits has magnitude at least 2^{bits} after {trace:?}"
            );
            if let Some(digits) = reportable_digits(bits) {
                // The magnitude exceeds 2^bits, so the digit count is below
                // the value's own width.
                let digits = usize::try_from(digits)
                    .expect("a digit count below a stored value's width is addressable");
                assert!(
                    oracle.magnitude() > &max_magnitude(digits),
                    "a stable answer at {bits} bits exceeds every accumulator of that width \
                     after {trace:?}"
                );
            }
        }
        None => {
            if reportable_digits(bits).is_some() {
                assert!(
                    u128::from(member.stored_bits()) <= u128::from(bits) + 64,
                    "a refusal at {bits} bits leaves stored_bits {} at most 64 bits wider \
                     after {trace:?}",
                    member.stored_bits()
                );
            }
        }
    }
}

/// Convert stored copies of `member` to every primitive integer type.
///
/// A conversion succeeds exactly when the value fits, with the value itself;
/// a rejection returns the same value. Copies keep the conversions' reads from
/// changing the member's stored form.
fn check_conversions(member: &Accumulator, oracle: &IBig, trace: &[u8]) {
    macro_rules! check {
        ($($integer:ty),*) => {$(
            match (
                <$integer>::try_from(oracle),
                <$integer>::try_from(member.stored_copy()),
            ) {
                (Ok(expected), Ok(found)) => assert_eq!(
                    found,
                    expected,
                    "{} conversion after {trace:?}",
                    stringify!($integer)
                ),
                (Err(_), Err(rejected)) => assert_value(&rejected, oracle),
                (expected, found) => panic!(
                    "{} conversion after {trace:?}: expected {expected:?}, found {found:?}",
                    stringify!($integer)
                ),
            }
        )*};
    }
    check!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
}

/// Apply steps to the members and their oracles in lockstep.
impl Pool {
    /// Start every member at zero, in the representation `digits` selects.
    fn new(digits: [bool; POOL]) -> Pool {
        Pool {
            members: digits.map(fresh),
            oracles: Default::default(),
        }
    }

    /// Check every member without changing its stored form.
    fn observe(&self, trace: &[u8]) {
        for (member, oracle) in self.members.iter().zip(&self.oracles) {
            assert_value(member, oracle);
            member.stored().assert_invariants(trace);
            if member.is_known_zero() {
                assert_eq!(oracle, &IBig::ZERO, "a known zero is zero after {trace:?}");
            }
            assert_stored_width_bounds(member, oracle, trace);
        }
    }

    /// Apply one step, checking any answer it asks for.
    fn apply(&mut self, step: &Step, trace: &[u8]) {
        match step {
            Step::Word { member, value } => {
                self.members[*member] += *value;
                self.oracles[*member] += *value;
            }
            Step::UnsignedWord {
                member,
                subtract,
                value,
            } => {
                if *subtract {
                    self.members[*member] -= *value;
                    self.oracles[*member] -= *value;
                } else {
                    self.members[*member] += *value;
                    self.oracles[*member] += *value;
                }
            }
            Step::Wide { member, value } => {
                self.members[*member] += *value;
                self.oracles[*member] += *value;
            }
            Step::UnsignedWide {
                member,
                subtract,
                value,
            } => {
                if *subtract {
                    self.members[*member] -= *value;
                    self.oracles[*member] -= *value;
                } else {
                    self.members[*member] += *value;
                    self.oracles[*member] += *value;
                }
            }
            Step::Limbs {
                member,
                subtract,
                limbs,
                shift,
            } => {
                let scaled = IBig::from(from_limbs(limbs)) << *shift;
                if *subtract {
                    self.members[*member].sub_shifted_limbs(*shift, limbs.iter().copied());
                    self.oracles[*member] -= scaled;
                } else {
                    self.members[*member].add_shifted_limbs(*shift, limbs.iter().copied());
                    self.oracles[*member] += scaled;
                }
            }
            Step::Accumulator {
                receiver,
                operand,
                subtract,
                shift,
                form,
            } => self.apply_accumulator(*receiver, *operand, *subtract, *shift, *form),
            Step::ParkExtreme { member, index } => {
                park_extreme_negative_digit(&mut self.members[*member], *index);
                self.oracles[*member] -= IBig::from(LARGEST_DIGIT) << (32 * *index);
            }
            Step::Shift { member, shift } => {
                self.members[*member] <<= *shift;
                self.oracles[*member] <<= *shift;
            }
            Step::Negate { member, borrowed } => {
                let value = &mut self.members[*member];
                *value = if *borrowed {
                    -&*value
                } else {
                    -core::mem::take(value)
                };
                self.oracles[*member] = -core::mem::take(&mut self.oracles[*member]);
            }
            Step::Clear { member } => {
                self.members[*member].clear();
                self.oracles[*member] = IBig::ZERO;
            }
            Step::Read { member, form } => {
                let (value, oracle) = (&mut self.members[*member], &self.oracles[*member]);
                match form {
                    ReadForm::Signed => assert_eq!(&read_value(value, false), oracle),
                    ReadForm::Scaled => assert_eq!(&read_value(value, true), oracle),
                    ReadForm::Clone => assert_value(&value.clone(), oracle),
                }
                assert_eq!(
                    value.stored_digit_count(),
                    own_digits(oracle),
                    "an in-place {form:?} read leaves the value's own working width after \
                     {trace:?}"
                );
            }
            Step::Sign { member } => {
                let sign = self.members[*member].sign();
                assert_eq!(
                    sign,
                    oracle_sign(&self.oracles[*member]),
                    "sign after {trace:?}"
                );
                if sign == Ordering::Equal {
                    assert!(
                        self.members[*member].is_known_zero(),
                        "a zero sign leaves a known zero after {trace:?}"
                    );
                }
            }
            Step::Stable { member, bits } => {
                let value = &self.members[*member];
                let answer = value.cmp_zero_stable_under(*bits);
                check_stable(answer, *bits, value, &self.oracles[*member], trace);
            }
            Step::StableAgainst { member, operand } => {
                let bits = self.members[*operand].stored_bits();
                assert!(
                    reportable_digits(bits).is_some(),
                    "a working width of {bits} bits is a whole number of digits, so the \
                     accumulator clause applies, after {trace:?}"
                );
                let value = &self.members[*member];
                let answer = value.cmp_zero_stable_under(bits);
                let oracle = &self.oracles[*member];
                check_stable(answer, bits, value, oracle, trace);
                if let Some(order) = answer {
                    let operand_oracle = &self.oracles[*operand];
                    assert_eq!(
                        oracle_sign(&(oracle + operand_oracle)),
                        order,
                        "adding the operand keeps a stable order after {trace:?}"
                    );
                    assert_eq!(
                        oracle_sign(&(oracle - operand_oracle)),
                        order,
                        "subtracting the operand keeps a stable order after {trace:?}"
                    );
                }
            }
            Step::Compare { left, right } => {
                let expected = self.oracles[*left].cmp(&self.oracles[*right]);
                let (left, right) = (&self.members[*left], &self.members[*right]);
                assert_eq!(left.cmp(right), expected, "cmp after {trace:?}");
                assert_eq!(
                    right.partial_cmp(left),
                    Some(expected.reverse()),
                    "reversed partial_cmp after {trace:?}"
                );
                assert_eq!(
                    left == right,
                    expected == Ordering::Equal,
                    "equality after {trace:?}"
                );
                if expected == Ordering::Equal {
                    let hasher = RandomState::new();
                    assert_eq!(
                        hasher.hash_one(left),
                        hasher.hash_one(right),
                        "equal values hash equally after {trace:?}"
                    );
                }
            }
            Step::Reserve { member, bits } => self.members[*member].reserve_bits(*bits),
            Step::Convert { member } => {
                check_conversions(&self.members[*member], &self.oracles[*member], trace);
            }
            Step::CloneFrom { receiver, source } => {
                if receiver == source {
                    let value = &mut self.members[*receiver];
                    *value = value.clone();
                } else {
                    let (target, source_value) = two_members(&mut self.members, *receiver, *source);
                    target.clone_from(source_value);
                    self.oracles[*receiver] = self.oracles[*source].clone();
                }
                for member in [*receiver, *source] {
                    assert_eq!(
                        self.members[member].stored_digit_count(),
                        own_digits(&self.oracles[member]),
                        "a clone and its source hold the value's own working width after \
                         {trace:?}"
                    );
                }
            }
            Step::Sum {
                receiver,
                members,
                cloned,
            } => {
                let total: IBig = members.iter().map(|&member| &self.oracles[member]).sum();
                let sum: Accumulator = if *cloned {
                    members
                        .iter()
                        .map(|&member| self.members[member].clone())
                        .sum()
                } else {
                    members.iter().map(|&member| &self.members[member]).sum()
                };
                self.members[*receiver] = sum;
                self.oracles[*receiver] = total;
            }
            Step::AbsDiff {
                receiver,
                operand,
                take,
            } => {
                let order = self.oracles[*receiver].cmp(&self.oracles[*operand]);
                let gap = IBig::from(
                    (&self.oracles[*receiver] - &self.oracles[*operand])
                        .magnitude()
                        .clone(),
                );
                let other = if *take && receiver != operand {
                    self.oracles[*operand] = IBig::ZERO;
                    core::mem::take(&mut self.members[*operand])
                } else {
                    self.members[*operand].clone()
                };
                let value = core::mem::take(&mut self.members[*receiver]);
                let (reported, difference) = value.abs_diff(other);
                assert_eq!(reported, order, "abs_diff order after {trace:?}");
                self.members[*receiver] = difference;
                self.oracles[*receiver] = gap;
            }
            Step::GapRound {
                member,
                low,
                inner,
                top,
            } => {
                let value = &mut self.members[*member];
                let lows = low.iter().copied();
                for position in lows.clone().chain([*top, *inner]) {
                    value.add_shifted_limbs(32 * position, [1]);
                }
                for position in [*top, *inner].into_iter().chain(lows) {
                    value.sub_shifted_limbs(32 * position, [1]);
                }
            }
        }
    }

    /// Add or subtract `operand` into `receiver` in one [`OperandForm`].
    fn apply_accumulator(
        &mut self,
        receiver: usize,
        operand: usize,
        subtract: bool,
        shift: u64,
        form: OperandForm,
    ) {
        let operand_oracle = self.oracles[operand].clone();
        let receiver_oracle = core::mem::take(&mut self.oracles[receiver]);
        let (left, right) = match form {
            OperandForm::Shifted => (receiver_oracle, operand_oracle << shift),
            OperandForm::Reversed => (operand_oracle, receiver_oracle),
            _ => (receiver_oracle, operand_oracle),
        };
        self.oracles[receiver] = if subtract { left - right } else { left + right };

        if receiver == operand {
            let value = &mut self.members[receiver];
            if let OperandForm::Borrowed = form {
                #[allow(
                    clippy::eq_op,
                    reason = "one accumulator as both operands is the case under test"
                )]
                let result = if subtract {
                    &*value - &*value
                } else {
                    &*value + &*value
                };
                *value = result;
                return;
            }
            // Every other form borrows the receiver mutably or consumes it, so
            // the member lends a clone of itself.
            let copy = value.clone();
            apply_borrowed(value, &copy, subtract, shift, form);
            return;
        }

        let (target, source) = two_members(&mut self.members, receiver, operand);
        if let OperandForm::OwnedTake = form {
            apply_owned(target, core::mem::take(source), subtract);
            self.oracles[operand] = IBig::ZERO;
        } else {
            apply_borrowed(target, source, subtract, shift, form);
        }
    }
}

/// Apply `operand` to `receiver` in `form`, leaving `operand` with its owner.
///
/// The owned forms consume a clone of `operand`, so for them this is
/// [`OwnedClone`](OperandForm::OwnedClone).
fn apply_borrowed(
    receiver: &mut Accumulator,
    operand: &Accumulator,
    subtract: bool,
    shift: u64,
    form: OperandForm,
) {
    match form {
        OperandForm::Borrowed => {
            if subtract {
                *receiver -= operand;
            } else {
                *receiver += operand;
            }
        }
        OperandForm::Shifted => {
            if subtract {
                receiver.sub_shifted(shift, operand);
            } else {
                receiver.add_shifted(shift, operand);
            }
        }
        OperandForm::OwnedClone | OperandForm::OwnedTake => {
            apply_owned(receiver, operand.clone(), subtract);
        }
        OperandForm::Reversed => {
            let value = core::mem::take(receiver);
            *receiver = if subtract {
                operand - value
            } else {
                operand + value
            };
        }
    }
}

/// Replace `receiver` by `receiver ± operand`, consuming both.
fn apply_owned(receiver: &mut Accumulator, operand: Accumulator, subtract: bool) {
    let value = core::mem::take(receiver);
    *receiver = if subtract {
        value - operand
    } else {
        value + operand
    };
}

/// Generate a pool index.
fn arb_member() -> impl Strategy<Value = usize> {
    0..POOL
}

/// Generate machine words at the digit, limb, and sign boundaries, small
/// counts, and arbitrary words.
fn arb_word() -> impl Strategy<Value = u64> {
    prop_oneof![
        3 => proptest::sample::select(vec![
            0u64, 1, 2, 3, 4, 5, (1 << 31) - 1, 1 << 31, (1 << 32) - 1, 1 << 32, (1 << 32) + 1,
            (1 << 33) - 1, 1 << 33, 3 << 31, (1 << 63) - 1, 1 << 63, u64::MAX - 1, u64::MAX,
        ]),
        2 => 0u64..16,
        2 => any::<u64>(),
    ]
}

/// Generate shift counts at digit and limb boundaries, small, arbitrary,
/// and far enough to make values thousands of digits wide.
fn arb_shift() -> impl Strategy<Value = u64> {
    prop_oneof![
        3 => 0u64..=4,
        3 => proptest::sample::select(vec![
            29u64, 30, 31, 32, 33, 63, 64, 65, 95, 96, 97, 127, 128, 129, 159, 160, 191, 192, 193,
            223, 224, 255, 256, 257,
        ]),
        2 => 0u64..400,
        1 => (0u64..64).prop_map(|digits| 32 * digits),
        1 => (0u64..64).prop_map(|digits| 32 * digits + 31),
        1 => 1_000u64..6_000,
        2 => proptest::sample::select(vec![3_200u64, 3_207, 32_000, 32_031, 64_000]),
    ]
}

/// Generate stability widths: digit-aligned, arbitrary, and far beyond any
/// width a test value reaches, including digit-aligned widths near
/// `u64::MAX`.
fn arb_stability_bits() -> impl Strategy<Value = u64> {
    prop_oneof![
        3 => (0u64..12).prop_map(|digits| 32 * digits),
        2 => 0u64..400,
        1 => proptest::sample::select(vec![
            u64::MAX,
            u64::MAX - 1,
            u64::MAX - 31,
            1 << 40,
            (1u64 << 63) + 7,
        ]),
    ]
}

/// Generate signed values within `2^33` of `k · 2^(32 d)`, for `k` in 1..=4
/// and `d` in 1..=2, where the small form's comparison threshold and the
/// digit boundaries lie.
fn arb_near_threshold() -> impl Strategy<Value = i128> {
    (
        1i128..=4,
        1u32..=2,
        -(1i128 << 33)..(1i128 << 33),
        any::<bool>(),
    )
        .prop_map(|(multiple, digits, offset, negative)| {
            let value = (multiple << (32 * digits)) + offset;
            if negative {
                -value
            } else {
                value
            }
        })
}

/// Generate one step of `kind`.
fn arb_step(kind: Kind) -> BoxedStrategy<Step> {
    let member = arb_member;
    match kind {
        Kind::Word => (member(), any::<i64>(), 0u32..64)
            // Shifting right by a random count spreads magnitudes over every width.
            .prop_map(|(member, value, narrowing)| Step::Word {
                member,
                value: value >> narrowing,
            })
            .boxed(),
        Kind::UnsignedWord => (member(), any::<bool>(), arb_word())
            .prop_map(|(member, subtract, value)| Step::UnsignedWord {
                member,
                subtract,
                value,
            })
            .boxed(),
        Kind::Wide => (
            member(),
            prop_oneof![
                any::<i128>(),
                Just(i128::MIN),
                Just(i128::MAX),
                Just(1i128 << 96),
                Just(-(1i128 << 96)),
                Just((1i128 << 96) + 1),
                arb_near_threshold(),
            ],
        )
            .prop_map(|(member, value)| Step::Wide { member, value })
            .boxed(),
        Kind::UnsignedWide => (
            member(),
            any::<bool>(),
            prop_oneof![
                any::<u128>(),
                Just(u128::MAX),
                Just(1u128 << 127),
                Just(1u128 << 96),
            ],
        )
            .prop_map(|(member, subtract, value)| Step::UnsignedWide {
                member,
                subtract,
                value,
            })
            .boxed(),
        Kind::Limbs => (
            member(),
            any::<bool>(),
            proptest::collection::vec(arb_word(), 0..=6),
            arb_shift(),
        )
            .prop_map(|(member, subtract, limbs, shift)| Step::Limbs {
                member,
                subtract,
                limbs,
                shift,
            })
            .boxed(),
        Kind::Accumulator => (
            member(),
            member(),
            any::<bool>(),
            arb_shift(),
            prop_oneof![
                Just(OperandForm::Borrowed),
                Just(OperandForm::Shifted),
                Just(OperandForm::OwnedClone),
                Just(OperandForm::OwnedTake),
                Just(OperandForm::Reversed),
            ],
        )
            .prop_map(
                |(receiver, operand, subtract, shift, form)| Step::Accumulator {
                    receiver,
                    operand,
                    subtract,
                    shift,
                    form,
                },
            )
            .boxed(),
        Kind::ParkExtreme => (member(), 0u64..12)
            .prop_map(|(member, index)| Step::ParkExtreme { member, index })
            .boxed(),
        Kind::Shift => (member(), arb_shift())
            .prop_map(|(member, shift)| Step::Shift { member, shift })
            .boxed(),
        Kind::Negate => (member(), any::<bool>())
            .prop_map(|(member, borrowed)| Step::Negate { member, borrowed })
            .boxed(),
        Kind::Clear => member().prop_map(|member| Step::Clear { member }).boxed(),
        Kind::Read => (
            member(),
            prop_oneof![
                Just(ReadForm::Signed),
                Just(ReadForm::Scaled),
                Just(ReadForm::Clone),
            ],
        )
            .prop_map(|(member, form)| Step::Read { member, form })
            .boxed(),
        Kind::Sign => member().prop_map(|member| Step::Sign { member }).boxed(),
        Kind::Stable => (member(), arb_stability_bits())
            .prop_map(|(member, bits)| Step::Stable { member, bits })
            .boxed(),
        Kind::StableAgainst => (member(), member())
            .prop_map(|(member, operand)| Step::StableAgainst { member, operand })
            .boxed(),
        Kind::Compare => (member(), member())
            .prop_map(|(left, right)| Step::Compare { left, right })
            .boxed(),
        Kind::Reserve => (
            member(),
            prop_oneof![
                0u64..=10_000,
                // At least 2^60 bits is at least 2^55 eight-byte digits, 2^58
                // bytes, more than any platform can address, so the hint is
                // ignored.
                (1u64 << 60)..=u64::MAX,
            ],
        )
            .prop_map(|(member, bits)| Step::Reserve { member, bits })
            .boxed(),
        Kind::Convert => member().prop_map(|member| Step::Convert { member }).boxed(),
        Kind::CloneFrom => (member(), member())
            .prop_map(|(receiver, source)| Step::CloneFrom { receiver, source })
            .boxed(),
        Kind::Sum => (
            member(),
            proptest::collection::vec(member(), 0..=5),
            any::<bool>(),
        )
            .prop_map(|(receiver, members, cloned)| Step::Sum {
                receiver,
                members,
                cloned,
            })
            .boxed(),
        Kind::AbsDiff => (member(), member(), any::<bool>())
            .prop_map(|(receiver, operand, take)| Step::AbsDiff {
                receiver,
                operand,
                take,
            })
            .boxed(),
        Kind::GapRound => (member(), 3u64..2_000, any::<bool>(), 0u64..1_000_000)
            .prop_map(|(member, top, with_low, choice)| {
                // `inner` lies in 2..top, and `low` leaves a gap below it.
                let inner = 2 + choice % (top - 2);
                let low = (with_low && inner > 3).then(|| choice / 7 % (inner - 2));
                Step::GapRound {
                    member,
                    low,
                    inner,
                    top,
                }
            })
            .boxed(),
    }
}

/// Generate a program: starting representations, a swarm of enabled kinds
/// with weights, then up to [`MAX_STEPS`] steps drawn from that swarm.
///
/// Each kind is disabled in about a third of cases and otherwise weighted
/// from 1 to 5; a case that disables every [`Kind::WRITERS`] kind enables
/// [`Kind::Limbs`].
fn arb_program() -> impl Strategy<Value = Program> {
    let weights = proptest::array::uniform::<_, { Kind::ALL.len() }>(
        prop_oneof![1 => Just(0u32), 2 => 1u32..6],
    )
    .prop_map(|mut weights| {
        if Kind::WRITERS
            .iter()
            .all(|&kind| weights[kind as usize] == 0)
        {
            weights[Kind::Limbs as usize] = 3;
        }
        weights
    });
    (any::<[bool; POOL]>(), weights).prop_flat_map(|(digits, weights)| {
        let arms: Vec<(u32, BoxedStrategy<Step>)> = Kind::ALL
            .iter()
            .copied()
            .filter(|&kind| weights[kind as usize] > 0)
            .map(|kind| (weights[kind as usize], arb_step(kind)))
            .collect();
        proptest::collection::vec(proptest::strategy::Union::new_weighted(arms), 1..=MAX_STEPS)
            .prop_map(move |steps| Program { digits, steps })
    })
}

proptest! {
    /// Every step over a pool of accumulators agrees with exact big-integer
    /// oracles, whatever histories its receiver and operands carry.
    ///
    /// After every step, every member reads out its oracle's value through
    /// both limb readouts, keeps the representation invariants, reports a
    /// known zero only for zero, and holds a value within the public
    /// `2^(stored_bits + 2)` bound and the `2.01 · 2^stored_bits` bound the
    /// comparison with zero rests on. A step that asks a question gets the
    /// oracle's answer: signs, comparisons with equal hashes for equal
    /// values, conversions, `abs_diff`'s order, and stability answers sound
    /// for every integer and every accumulator of the requested width (and
    /// so every narrower one), with a refusal at another accumulator's width
    /// leaving the working width at most 64 bits wider. In-place reads and
    /// clones leave the value's own working width.
    #[test]
    fn pool_programs_match_the_oracle(program in arb_program()) {
        let mut pool = Pool::new(program.digits);
        let mut trace = Vec::with_capacity(program.steps.len());
        pool.observe(&trace);
        for step in &program.steps {
            trace.push(step.kind() as u8);
            pool.apply(step, &trace);
            pool.observe(&trace);
        }
    }
}
