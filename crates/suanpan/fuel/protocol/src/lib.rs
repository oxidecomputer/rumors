//! The program format shared by the interleaving-fuel guest and its harness.
//!
//! A [`Program`] is a list of [`Step`]s over a pool of [`POOL`] accumulators,
//! run [`Program::repetitions`] times in a row. Every step is one public
//! `suanpan` operation, or one of the fixed compositions the crate's cost
//! rules themselves decompose (an owned operator on a clone, a sum, an
//! `abs_diff`). The harness builds programs, encodes them with
//! [`Program::encode`], and writes the words into the guest, which decodes
//! them with [`Program::decode`] and runs them under fuel.
//!
//! Steps carry concrete sizes. Wide limb operands travel as a [`LimbInput`],
//! a pattern and a count that both sides expand the same way, so a megabyte
//! operand crosses the boundary in two words.
//!
//! A step that returns an answer (a sign, an order, a magnitude) also yields
//! one observation word, computed on both sides by the functions below from
//! the answer alone. The harness compares the guest's observations with its
//! own model's, which establishes that the guest ran the program the harness
//! charged.

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::fmt;

/// The number of accumulators in a program's pool.
///
/// Three let one step involve a receiver, an operand, and a member it leaves
/// alone.
pub const POOL: usize = 3;

/// The most members one [`Step::Sum`] adds.
pub const MAX_SUM_TERMS: usize = 5;

/// The number of words each encoded step occupies.
const STEP_WORDS: usize = 5;

/// A pool member, an index below [`POOL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Member(u8);

impl Member {
    /// Every member, in index order.
    pub const ALL: [Member; POOL] = [Member(0), Member(1), Member(2)];

    /// Returns the member at `index`, or `None` if the pool has no such
    /// member.
    pub fn new(index: usize) -> Option<Member> {
        Member::ALL.get(index).copied()
    }

    /// Returns the member's index into the pool.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// Whether an update adds its operand or subtracts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// `+=`, `+`, or an `add_*` method.
    Add,
    /// `-=`, `-`, or a `sub_*` method.
    Sub,
}

/// A primitive integer operand of one of the widths the crate accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Primitive {
    /// A signed machine word.
    I64(i64),
    /// An unsigned machine word.
    U64(u64),
    /// A signed 128-bit value.
    I128(i128),
    /// An unsigned 128-bit value.
    U128(u128),
}

/// How the limbs of a [`LimbInput`] are filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// Every limb is `u64::MAX`: subtracting it from a power of two leaves a
    /// run of ones as long as the input, the material of borrow runs.
    Ones,
    /// Only the highest bit of the highest limb is set.
    TopBit,
    /// Every limb is `0xAAAA_AAAA_AAAA_AAAA`.
    Alternating,
    /// The lowest and highest limbs are 1 and every limb between is zero.
    Sparse,
    /// Every limb is this word.
    Word(u64),
    /// Each limb is a deterministic pseudorandom word drawn from the seed and
    /// the limb's index, except the highest, which is drawn from the seed
    /// alone.
    ///
    /// The lowest and highest limbs are therefore the same at every count,
    /// so the operand's ends, where a shift spills into another digit and a
    /// carry leaves the top, behave alike however long the operand is.
    Mixed(u64),
}

/// A limb operand: `count` little-endian `u64` limbs filled by `pattern`.
///
/// The count's type is a length type, as in [`Step`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimbInput<L = u64> {
    /// How the limbs are filled.
    pub pattern: Pattern,
    /// The number of limbs.
    pub count: L,
}

impl LimbInput {
    /// Returns limb `index` of the operand, for any `index` below
    /// [`count`](Self::count).
    pub fn limb(&self, index: u64) -> u64 {
        let top = index + 1 == self.count;
        match self.pattern {
            Pattern::Ones => u64::MAX,
            Pattern::TopBit => u64::from(top) << 63,
            Pattern::Alternating => 0xAAAA_AAAA_AAAA_AAAA,
            Pattern::Sparse => u64::from(index == 0 || top),
            Pattern::Word(word) => word,
            Pattern::Mixed(seed) if top => splitmix64(!seed),
            Pattern::Mixed(seed) => splitmix64(seed.wrapping_add(index)),
        }
    }

    /// Returns every limb of the operand, lowest first.
    pub fn expand(&self) -> Vec<u64> {
        (0..self.count).map(|index| self.limb(index)).collect()
    }
}

/// How an accumulator operand reaches its receiver in a [`Step::Operand`].
///
/// When the receiver is its own operand, [`Borrowed`](Form::Borrowed)
/// computes `&member ± &member`, the one spelling that reads one accumulator
/// as both operands; every other form lends a clone of the member, since it
/// borrows the receiver mutably or consumes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form<L = u64> {
    /// `receiver += &operand` or `receiver -= &operand`.
    Borrowed,
    /// `add_shifted` or `sub_shifted` at this shift in bits.
    Shifted(L),
    /// `receiver ± operand.clone()`, consuming the receiver.
    OwnedClone,
    /// `receiver ± take(operand)`, consuming both, so the operand's member
    /// starts again as a new accumulator.
    OwnedTake,
    /// `&operand ± receiver`, consuming the receiver as the right operand.
    Reversed,
}

/// A question about a member's sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignQuery<L = u64> {
    /// `sign`.
    Sign,
    /// `is_zero`.
    IsZero,
    /// `is_positive`.
    IsPositive,
    /// `is_negative`.
    IsNegative,
    /// `cmp_zero_stable_under` at this width in bits.
    StableUnder(L),
}

/// A read that returns, or consumes, the whole magnitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Read {
    /// `signed_magnitude`.
    Signed,
    /// `scaled_signed_magnitude`.
    Scaled,
    /// `Hash::hash` into the standard library's default hasher.
    Hash,
}

/// One instruction of a program.
///
/// `L` is the type of the step's lengths: its shifts, its stability width,
/// and its limb count. A program's steps hold `u64` bits and limbs; the
/// harness describes a step whose lengths are measured against a unit with
/// another `L`, and resolves it with [`map_lengths`](Step::map_lengths).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step<L = u64> {
    /// `member += value` or `member -= value`.
    Primitive {
        /// The member updated.
        member: Member,
        /// Whether to add or subtract.
        direction: Direction,
        /// The operand.
        value: Primitive,
    },
    /// `add_shifted_limbs` or `sub_shifted_limbs`.
    Limbs {
        /// The member updated.
        member: Member,
        /// Whether to add or subtract.
        direction: Direction,
        /// The power-of-two scale, in bits.
        shift: L,
        /// The limbs.
        input: LimbInput<L>,
    },
    /// Add or subtract one member into another, or into itself.
    Operand {
        /// The member updated.
        receiver: Member,
        /// The member supplying the operand.
        operand: Member,
        /// Whether to add or subtract.
        direction: Direction,
        /// How the operand is supplied.
        form: Form<L>,
    },
    /// Negate the owned value, or replace it by `-&member`.
    Negate {
        /// The member negated.
        member: Member,
        /// Whether to negate a borrow, which clones first.
        borrowed: bool,
    },
    /// `member <<= shift`.
    Shift {
        /// The member shifted.
        member: Member,
        /// The shift count in bits.
        shift: L,
    },
    /// `clear`.
    Clear {
        /// The member cleared.
        member: Member,
    },
    /// Ask a question about the sign.
    Sign {
        /// The member asked.
        member: Member,
        /// The question.
        query: SignQuery<L>,
    },
    /// `member.partial_cmp(&value)` against a primitive.
    ComparePrimitive {
        /// The member compared.
        member: Member,
        /// The primitive it is compared with.
        value: i128,
    },
    /// `i64::try_from(member)`, putting a value that does not fit back, and
    /// replacing one that fits by `Accumulator::from` of it.
    Convert {
        /// The member converted.
        member: Member,
    },
    /// `left.cmp(&right)`.
    Compare {
        /// The left operand.
        left: Member,
        /// The right operand.
        right: Member,
    },
    /// Read the magnitude in place.
    Read {
        /// The member read.
        member: Member,
        /// The reading operation.
        read: Read,
    },
    /// `receiver = source.clone()`.
    Clone {
        /// The member overwritten.
        receiver: Member,
        /// The member copied.
        source: Member,
    },
    /// `receiver.clone_from(&source)`, or a plain clone when they are the
    /// same member.
    CloneFrom {
        /// The member overwritten.
        receiver: Member,
        /// The member copied.
        source: Member,
    },
    /// Replace the receiver by `|receiver - operand|` with `abs_diff`.
    AbsDiff {
        /// The member replaced.
        receiver: Member,
        /// The member subtracted.
        operand: Member,
        /// Whether to take the operand, leaving its member new, rather than
        /// clone it; a receiver that is its own operand is always cloned.
        take: bool,
    },
    /// Replace the receiver by the sum of members, by `Sum`.
    Sum {
        /// The member replaced.
        receiver: Member,
        /// The members summed, in order, repeats allowed, at most
        /// [`MAX_SUM_TERMS`].
        members: Vec<Member>,
        /// Whether to sum clones rather than borrows.
        owned: bool,
    },
}

/// A program: steps, run `repetitions` times over a pool that starts at
/// zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    /// How many times the steps run, one after another.
    pub repetitions: u32,
    /// The steps of one repetition.
    pub steps: Vec<Step>,
}

/// Why a word sequence is not an encoded [`Program`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The words end before the header or a step does.
    Truncated,
    /// Words follow the last step.
    TrailingWords,
    /// A step's tag, member, or field names nothing.
    Malformed {
        /// The index of the offending step.
        step: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "the program's words end early"),
            DecodeError::TrailingWords => write!(f, "words follow the program's last step"),
            DecodeError::Malformed { step } => write!(f, "step {step} is malformed"),
        }
    }
}

/// The tag in the low byte of a step's first word.
mod tag {
    pub const PRIMITIVE: u8 = 0;
    pub const LIMBS: u8 = 1;
    pub const OPERAND: u8 = 2;
    pub const NEGATE: u8 = 3;
    pub const SHIFT: u8 = 4;
    pub const CLEAR: u8 = 5;
    pub const SIGN: u8 = 6;
    pub const COMPARE_PRIMITIVE: u8 = 7;
    pub const CONVERT: u8 = 8;
    pub const COMPARE: u8 = 9;
    pub const READ: u8 = 10;
    pub const CLONE: u8 = 11;
    pub const CLONE_FROM: u8 = 12;
    pub const ABS_DIFF: u8 = 13;
    pub const SUM: u8 = 14;
}

/// The first word of an encoded step: five byte fields, lowest first.
///
/// Byte 0 is the tag, bytes 1 and 2 are members, byte 3 is a flag or
/// direction, and byte 4 selects a variant within the step (a primitive's
/// width, a form, a pattern, a query, a read, or a sum's term count).
#[derive(Clone, Copy, Default)]
struct Head {
    /// The step's tag.
    tag: u8,
    /// The first member field.
    first: u8,
    /// The second member field.
    second: u8,
    /// A direction or flag, 0 or 1.
    flag: u8,
    /// A variant selector.
    variant: u8,
}

impl Head {
    /// Packs the fields into one word.
    fn word(self) -> u64 {
        u64::from(self.tag)
            | u64::from(self.first) << 8
            | u64::from(self.second) << 16
            | u64::from(self.flag) << 24
            | u64::from(self.variant) << 32
    }

    /// Unpacks a word written by [`word`](Self::word).
    fn from_word(word: u64) -> Head {
        let byte = |shift: u32| (word >> shift) as u8;
        Head {
            tag: byte(0),
            first: byte(8),
            second: byte(16),
            flag: byte(24),
            variant: byte(32),
        }
    }
}

/// Encodes a direction as a flag byte.
fn direction_flag(direction: Direction) -> u8 {
    match direction {
        Direction::Add => 0,
        Direction::Sub => 1,
    }
}

/// Decodes a flag byte written by [`direction_flag`].
fn flag_direction(flag: u8) -> Option<Direction> {
    match flag {
        0 => Some(Direction::Add),
        1 => Some(Direction::Sub),
        _ => None,
    }
}

/// Decodes a flag byte holding a `bool`.
fn flag_bool(flag: u8) -> Option<bool> {
    match flag {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// Splits a 128-bit value into its low and high words.
fn split(value: u128) -> [u64; 2] {
    [value as u64, (value >> 64) as u64]
}

/// Joins a low and a high word into a 128-bit value.
fn join(low: u64, high: u64) -> u128 {
    u128::from(low) | u128::from(high) << 64
}

impl<L> Step<L> {
    /// Returns the step with every length replaced: shifts and the stability
    /// width through `bits`, and the limb count through `limbs`.
    pub fn map_lengths<M>(&self, bits: impl Fn(&L) -> M, limbs: impl Fn(&L) -> M) -> Step<M> {
        match self {
            Step::Primitive {
                member,
                direction,
                value,
            } => Step::Primitive {
                member: *member,
                direction: *direction,
                value: *value,
            },
            Step::Limbs {
                member,
                direction,
                shift,
                input,
            } => Step::Limbs {
                member: *member,
                direction: *direction,
                shift: bits(shift),
                input: LimbInput {
                    pattern: input.pattern,
                    count: limbs(&input.count),
                },
            },
            Step::Operand {
                receiver,
                operand,
                direction,
                form,
            } => Step::Operand {
                receiver: *receiver,
                operand: *operand,
                direction: *direction,
                form: match form {
                    Form::Borrowed => Form::Borrowed,
                    Form::Shifted(shift) => Form::Shifted(bits(shift)),
                    Form::OwnedClone => Form::OwnedClone,
                    Form::OwnedTake => Form::OwnedTake,
                    Form::Reversed => Form::Reversed,
                },
            },
            Step::Negate { member, borrowed } => Step::Negate {
                member: *member,
                borrowed: *borrowed,
            },
            Step::Shift { member, shift } => Step::Shift {
                member: *member,
                shift: bits(shift),
            },
            Step::Clear { member } => Step::Clear { member: *member },
            Step::Sign { member, query } => Step::Sign {
                member: *member,
                query: match query {
                    SignQuery::Sign => SignQuery::Sign,
                    SignQuery::IsZero => SignQuery::IsZero,
                    SignQuery::IsPositive => SignQuery::IsPositive,
                    SignQuery::IsNegative => SignQuery::IsNegative,
                    SignQuery::StableUnder(width) => SignQuery::StableUnder(bits(width)),
                },
            },
            Step::ComparePrimitive { member, value } => Step::ComparePrimitive {
                member: *member,
                value: *value,
            },
            Step::Convert { member } => Step::Convert { member: *member },
            Step::Compare { left, right } => Step::Compare {
                left: *left,
                right: *right,
            },
            Step::Read { member, read } => Step::Read {
                member: *member,
                read: *read,
            },
            Step::Clone { receiver, source } => Step::Clone {
                receiver: *receiver,
                source: *source,
            },
            Step::CloneFrom { receiver, source } => Step::CloneFrom {
                receiver: *receiver,
                source: *source,
            },
            Step::AbsDiff {
                receiver,
                operand,
                take,
            } => Step::AbsDiff {
                receiver: *receiver,
                operand: *operand,
                take: *take,
            },
            Step::Sum {
                receiver,
                members,
                owned,
            } => Step::Sum {
                receiver: *receiver,
                members: members.clone(),
                owned: *owned,
            },
        }
    }
}

impl Step {
    /// Encodes the step as [`STEP_WORDS`] words.
    fn encode(&self) -> [u64; STEP_WORDS] {
        let member = |m: Member| m.0;
        let mut head = Head::default();
        let mut payload = [0u64; STEP_WORDS - 1];
        match self {
            Step::Primitive {
                member: m,
                direction,
                value,
            } => {
                head.tag = tag::PRIMITIVE;
                head.first = member(*m);
                head.flag = direction_flag(*direction);
                let (variant, bits) = match *value {
                    Primitive::I64(v) => (0, u128::from(v as u64)),
                    Primitive::U64(v) => (1, u128::from(v)),
                    Primitive::I128(v) => (2, v as u128),
                    Primitive::U128(v) => (3, v),
                };
                head.variant = variant;
                [payload[0], payload[1]] = split(bits);
            }
            Step::Limbs {
                member: m,
                direction,
                shift,
                input,
            } => {
                head.tag = tag::LIMBS;
                head.first = member(*m);
                head.flag = direction_flag(*direction);
                let (variant, parameter) = match input.pattern {
                    Pattern::Ones => (0, 0),
                    Pattern::TopBit => (1, 0),
                    Pattern::Alternating => (2, 0),
                    Pattern::Sparse => (3, 0),
                    Pattern::Mixed(seed) => (4, seed),
                    Pattern::Word(word) => (5, word),
                };
                head.variant = variant;
                payload[0] = *shift;
                payload[1] = input.count;
                payload[2] = parameter;
            }
            Step::Operand {
                receiver,
                operand,
                direction,
                form,
            } => {
                head.tag = tag::OPERAND;
                head.first = member(*receiver);
                head.second = member(*operand);
                head.flag = direction_flag(*direction);
                head.variant = match *form {
                    Form::Borrowed => 0,
                    Form::Shifted(shift) => {
                        payload[0] = shift;
                        1
                    }
                    Form::OwnedClone => 2,
                    Form::OwnedTake => 3,
                    Form::Reversed => 4,
                };
            }
            Step::Negate {
                member: m,
                borrowed,
            } => {
                head.tag = tag::NEGATE;
                head.first = member(*m);
                head.flag = u8::from(*borrowed);
            }
            Step::Shift { member: m, shift } => {
                head.tag = tag::SHIFT;
                head.first = member(*m);
                payload[0] = *shift;
            }
            Step::Clear { member: m } => {
                head.tag = tag::CLEAR;
                head.first = member(*m);
            }
            Step::Sign { member: m, query } => {
                head.tag = tag::SIGN;
                head.first = member(*m);
                head.variant = match *query {
                    SignQuery::Sign => 0,
                    SignQuery::IsZero => 1,
                    SignQuery::IsPositive => 2,
                    SignQuery::IsNegative => 3,
                    SignQuery::StableUnder(bits) => {
                        payload[0] = bits;
                        4
                    }
                };
            }
            Step::ComparePrimitive { member: m, value } => {
                head.tag = tag::COMPARE_PRIMITIVE;
                head.first = member(*m);
                [payload[0], payload[1]] = split(*value as u128);
            }
            Step::Convert { member: m } => {
                head.tag = tag::CONVERT;
                head.first = member(*m);
            }
            Step::Compare { left, right } => {
                head.tag = tag::COMPARE;
                head.first = member(*left);
                head.second = member(*right);
            }
            Step::Read { member: m, read } => {
                head.tag = tag::READ;
                head.first = member(*m);
                head.variant = match read {
                    Read::Signed => 0,
                    Read::Scaled => 1,
                    Read::Hash => 2,
                };
            }
            Step::Clone { receiver, source } => {
                head.tag = tag::CLONE;
                head.first = member(*receiver);
                head.second = member(*source);
            }
            Step::CloneFrom { receiver, source } => {
                head.tag = tag::CLONE_FROM;
                head.first = member(*receiver);
                head.second = member(*source);
            }
            Step::AbsDiff {
                receiver,
                operand,
                take,
            } => {
                head.tag = tag::ABS_DIFF;
                head.first = member(*receiver);
                head.second = member(*operand);
                head.flag = u8::from(*take);
            }
            Step::Sum {
                receiver,
                members,
                owned,
            } => {
                head.tag = tag::SUM;
                head.first = member(*receiver);
                head.flag = u8::from(*owned);
                head.variant = u8::try_from(members.len()).unwrap_or(u8::MAX);
                for (slot, term) in members.iter().enumerate().take(MAX_SUM_TERMS) {
                    payload[0] |= u64::from(term.0) << (8 * slot);
                }
            }
        }
        let mut words = [0u64; STEP_WORDS];
        words[0] = head.word();
        words[1..].copy_from_slice(&payload);
        words
    }

    /// Decodes one step from its [`STEP_WORDS`] words, or returns `None` if
    /// they name no step.
    fn decode(words: &[u64; STEP_WORDS]) -> Option<Step> {
        let head = Head::from_word(words[0]);
        let payload = &words[1..];
        let first = Member::new(usize::from(head.first))?;
        let second = || Member::new(usize::from(head.second));
        let direction = || flag_direction(head.flag);
        let step = match head.tag {
            tag::PRIMITIVE => {
                let bits = join(payload[0], payload[1]);
                let value = match head.variant {
                    0 => Primitive::I64(bits as u64 as i64),
                    1 => Primitive::U64(bits as u64),
                    2 => Primitive::I128(bits as i128),
                    3 => Primitive::U128(bits),
                    _ => return None,
                };
                Step::Primitive {
                    member: first,
                    direction: direction()?,
                    value,
                }
            }
            tag::LIMBS => {
                let pattern = match head.variant {
                    0 => Pattern::Ones,
                    1 => Pattern::TopBit,
                    2 => Pattern::Alternating,
                    3 => Pattern::Sparse,
                    4 => Pattern::Mixed(payload[2]),
                    5 => Pattern::Word(payload[2]),
                    _ => return None,
                };
                Step::Limbs {
                    member: first,
                    direction: direction()?,
                    shift: payload[0],
                    input: LimbInput {
                        pattern,
                        count: payload[1],
                    },
                }
            }
            tag::OPERAND => {
                let form = match head.variant {
                    0 => Form::Borrowed,
                    1 => Form::Shifted(payload[0]),
                    2 => Form::OwnedClone,
                    3 => Form::OwnedTake,
                    4 => Form::Reversed,
                    _ => return None,
                };
                Step::Operand {
                    receiver: first,
                    operand: second()?,
                    direction: direction()?,
                    form,
                }
            }
            tag::NEGATE => Step::Negate {
                member: first,
                borrowed: flag_bool(head.flag)?,
            },
            tag::SHIFT => Step::Shift {
                member: first,
                shift: payload[0],
            },
            tag::CLEAR => Step::Clear { member: first },
            tag::SIGN => {
                let query = match head.variant {
                    0 => SignQuery::Sign,
                    1 => SignQuery::IsZero,
                    2 => SignQuery::IsPositive,
                    3 => SignQuery::IsNegative,
                    4 => SignQuery::StableUnder(payload[0]),
                    _ => return None,
                };
                Step::Sign {
                    member: first,
                    query,
                }
            }
            tag::COMPARE_PRIMITIVE => Step::ComparePrimitive {
                member: first,
                value: join(payload[0], payload[1]) as i128,
            },
            tag::CONVERT => Step::Convert { member: first },
            tag::COMPARE => Step::Compare {
                left: first,
                right: second()?,
            },
            tag::READ => {
                let read = match head.variant {
                    0 => Read::Signed,
                    1 => Read::Scaled,
                    2 => Read::Hash,
                    _ => return None,
                };
                Step::Read {
                    member: first,
                    read,
                }
            }
            tag::CLONE => Step::Clone {
                receiver: first,
                source: second()?,
            },
            tag::CLONE_FROM => Step::CloneFrom {
                receiver: first,
                source: second()?,
            },
            tag::ABS_DIFF => Step::AbsDiff {
                receiver: first,
                operand: second()?,
                take: flag_bool(head.flag)?,
            },
            tag::SUM => {
                let count = usize::from(head.variant);
                if count > MAX_SUM_TERMS {
                    return None;
                }
                let members = (0..count)
                    .map(|slot| Member::new(usize::from((payload[0] >> (8 * slot)) as u8)))
                    .collect::<Option<Vec<_>>>()?;
                Step::Sum {
                    receiver: first,
                    members,
                    owned: flag_bool(head.flag)?,
                }
            }
            _ => return None,
        };
        Some(step)
    }
}

impl Program {
    /// Encodes the program as words: the repetition count, the step count,
    /// then each step.
    ///
    /// # Panics
    ///
    /// Panics if a [`Step::Sum`] names more than [`MAX_SUM_TERMS`] members,
    /// which no decoder could recover.
    pub fn encode(&self) -> Vec<u64> {
        let mut words = Vec::with_capacity(2 + STEP_WORDS * self.steps.len());
        words.push(u64::from(self.repetitions));
        words.push(self.steps.len() as u64);
        for step in &self.steps {
            if let Step::Sum { members, .. } = step {
                assert!(
                    members.len() <= MAX_SUM_TERMS,
                    "a sum names at most {MAX_SUM_TERMS} members"
                );
            }
            words.extend_from_slice(&step.encode());
        }
        words
    }

    /// Decodes words written by [`encode`](Self::encode).
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] if the words end early, run past the last
    /// step, or hold a step that names nothing.
    pub fn decode(words: &[u64]) -> Result<Program, DecodeError> {
        let [repetitions, count, rest @ ..] = words else {
            return Err(DecodeError::Truncated);
        };
        let repetitions = u32::try_from(*repetitions).map_err(|_| DecodeError::Truncated)?;
        let count = usize::try_from(*count).map_err(|_| DecodeError::Truncated)?;
        let needed = count
            .checked_mul(STEP_WORDS)
            .ok_or(DecodeError::Truncated)?;
        if rest.len() < needed {
            return Err(DecodeError::Truncated);
        }
        if rest.len() > needed {
            return Err(DecodeError::TrailingWords);
        }
        let steps = rest
            .chunks_exact(STEP_WORDS)
            .enumerate()
            .map(|(index, chunk)| {
                let chunk: &[u64; STEP_WORDS] = chunk
                    .try_into()
                    .expect("chunks_exact yields chunks of exactly STEP_WORDS words");
                Step::decode(chunk).ok_or(DecodeError::Malformed { step: index })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Program { repetitions, steps })
    }
}

/// The observation word of a step that returns nothing.
pub const NO_OBSERVATION: u64 = 0;

/// Returns the observation word of an order or a sign.
pub fn observe_ordering(order: Ordering) -> u64 {
    match order {
        Ordering::Less => 1,
        Ordering::Equal => 2,
        Ordering::Greater => 3,
    }
}

/// Returns the observation word of a yes-or-no answer.
pub fn observe_bool(answer: bool) -> u64 {
    4 + u64::from(answer)
}

/// Returns the observation word of `cmp_zero_stable_under`'s answer.
pub fn observe_stable(answer: Option<Ordering>) -> u64 {
    answer.map_or(6, |order| 6 + observe_ordering(order))
}

/// Returns the observation word of an `i64` conversion: the value if it fit,
/// or a refusal.
pub fn observe_conversion(converted: Option<i64>) -> u64 {
    match converted {
        Some(value) => splitmix64(10 ^ value as u64),
        None => 10,
    }
}

/// Returns the observation word of a magnitude read: the sign, the value's
/// bit length, and its lowest limb (0 when the read does not reveal it).
pub fn observe_magnitude(sign: Ordering, bit_length: u64, low_limb: u64) -> u64 {
    splitmix64(observe_ordering(sign) ^ bit_length.rotate_left(2) ^ splitmix64(low_limb))
}

/// One step of SplitMix64 (Steele, Lea, and Flood, OOPSLA 2014): a bijective
/// mix of a 64-bit word.
pub fn splitmix64(word: u64) -> u64 {
    let mut z = word.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
