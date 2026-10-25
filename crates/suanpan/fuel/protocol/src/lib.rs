//! The program format shared by the interleaving-fuel guest and its harness.
//!
//! A [`Program`] is a list of [`Step`]s over a pool of [`POOL`] accumulators,
//! run [`Program::repetitions`] times in a row. Every step is one public
//! `suanpan` operation, or one of the fixed compositions the crate's cost
//! rules themselves decompose (an owned operator on a clone, a sum, an
//! `abs_diff`). The harness builds programs, encodes them with
//! [`Program::encode`], and writes the bytes into the guest, which decodes
//! them with [`Program::decode`] and runs them under fuel. Decoding happens
//! before the guest's measured window opens, so the format costs the
//! readings nothing, and it is `postcard`'s encoding of the derived `serde`
//! form.
//!
//! Steps carry concrete sizes. Wide limb operands travel as a [`LimbInput`],
//! a pattern and a count that both sides expand the same way, so a megabyte
//! operand crosses the boundary in a few bytes.
//!
//! A step that returns an answer (a sign, an order, a magnitude) also yields
//! one observation word, computed on both sides by the functions below from
//! the answer alone. The harness compares the guest's observations with its
//! own model's, which establishes that the guest's answers agree with the
//! model's values at every step.

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};

/// The number of accumulators in a program's pool.
///
/// Three let one step involve a receiver, an operand, and a member it leaves
/// alone.
pub const POOL: usize = 3;

/// A pool member, an index below [`POOL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
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

/// A member index that names no member of the pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoSuchMember(pub u8);

impl fmt::Display for NoSuchMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the pool has no member {}", self.0)
    }
}

impl TryFrom<u8> for Member {
    type Error = NoSuchMember;

    fn try_from(index: u8) -> Result<Member, NoSuchMember> {
        Member::new(usize::from(index)).ok_or(NoSuchMember(index))
    }
}

impl From<Member> for u8 {
    fn from(member: Member) -> u8 {
        member.0
    }
}

/// Whether an update adds its operand or subtracts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// `+=`, `+`, or an `add_*` method.
    Add,
    /// `-=`, `-`, or a `sub_*` method.
    Sub,
}

/// A primitive integer operand of one of the widths the crate accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

/// A primitive integer type that [`Step::Convert`] converts to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Integer {
    /// `i64`.
    I64,
    /// `u64`.
    U64,
    /// `i128`.
    I128,
    /// `u128`.
    U128,
}

/// How the limbs of a [`LimbInput`] are filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// `value + member` or `value - member`, with the primitive on the
    /// left, replacing the member by the result.
    ReversedPrimitive {
        /// The member updated.
        member: Member,
        /// Whether to add the member or subtract it.
        direction: Direction,
        /// The primitive on the left.
        value: Primitive,
        /// Whether to read `&member`, which clones it first, rather than
        /// consume it.
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
    /// `TryFrom<Accumulator>` into the target type, putting a value that
    /// does not fit back, and replacing one that fits by `Accumulator::from`
    /// of it.
    Convert {
        /// The member converted.
        member: Member,
        /// The type converted to.
        target: Integer,
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
        /// The members summed, in order, repeats allowed.
        members: Vec<Member>,
        /// Whether to sum clones rather than borrows.
        owned: bool,
    },
}

/// A program: steps, run `repetitions` times over a pool that starts at
/// zero.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    /// How many times the steps run, one after another.
    pub repetitions: u32,
    /// The steps of one repetition.
    pub steps: Vec<Step>,
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
            Step::ReversedPrimitive {
                member,
                direction,
                value,
                borrowed,
            } => Step::ReversedPrimitive {
                member: *member,
                direction: *direction,
                value: *value,
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
            Step::Convert { member, target } => Step::Convert {
                member: *member,
                target: *target,
            },
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

/// Why a byte sequence is not an encoded [`Program`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The bytes end before the program does.
    Truncated,
    /// Bytes follow the end of the program.
    TrailingBytes,
    /// The bytes name a step, field, or member that does not exist.
    Malformed,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "the program's bytes end early"),
            DecodeError::TrailingBytes => write!(f, "bytes follow the program's end"),
            DecodeError::Malformed => write!(f, "the program's bytes name nothing"),
        }
    }
}

impl Program {
    /// Encodes the program as bytes.
    pub fn encode(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("encoding a program into a vector cannot fail")
    }

    /// Decodes bytes written by [`encode`](Self::encode).
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] if the bytes end early, run past the end of
    /// the program, or name a step, field, or member that does not exist.
    pub fn decode(bytes: &[u8]) -> Result<Program, DecodeError> {
        match postcard::take_from_bytes::<Program>(bytes) {
            Ok((program, [])) => Ok(program),
            Ok(_) => Err(DecodeError::TrailingBytes),
            Err(postcard::Error::DeserializeUnexpectedEnd) => Err(DecodeError::Truncated),
            Err(_) => Err(DecodeError::Malformed),
        }
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

/// Returns the observation word of a conversion: the converted value's sign
/// and magnitude if it fit, or a refusal.
pub fn observe_conversion(converted: Option<(Ordering, u128)>) -> u64 {
    match converted {
        Some((sign, magnitude)) => splitmix64(
            10 ^ observe_ordering(sign).rotate_left(4)
                ^ splitmix64(magnitude as u64)
                ^ splitmix64((magnitude >> 64) as u64).rotate_left(1),
        ),
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
