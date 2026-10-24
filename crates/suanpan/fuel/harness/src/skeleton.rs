//! Program skeletons: steps whose lengths are measured against a unit `u`,
//! so one skeleton yields the same program at every scale.
//!
//! A [`Skeleton`] holds [`Step<Extent>`]s. An [`Extent`] is `units · u +
//! offset` bits, so resolving a skeleton at unit `k · u`
//! ([`Skeleton::program`]) multiplies every size-carrying length by `k` and
//! keeps every offset: the digit and limb misalignments, the primitive
//! operands, and the fixed positions stay exactly as they are. A shape that
//! cancels, borrows across a run, or leaves a sparse high digit at one unit
//! does so at every unit, over a span `k` times as long.
//!
//! The step kinds, operand forms, and swarm weighting follow the pool
//! property in `suanpan`'s own tests (`accumulator::tests::pool`), which is
//! internal to that crate and draws absolute sizes: each case enables a
//! random subset of step kinds with random weights (swarm testing, Groce et
//! al., ISSTA 2012), and its value-neutral gap rounds and extreme-digit
//! deposits appear here as fixed sequences of limb updates. One regime in
//! four restricts a case's context to the kinds in [`Kind::FIXED_CHARGE`],
//! whose charge has no size-carrying term once the storage is grown; only
//! such a skeleton's repetitions isolate the cost of returning to storage
//! already grown (see the crate docs).
//!
//! The generator draws conversions to `i64` alone, and never a
//! [`Step::ReversedPrimitive`]; the focused claims in `tests/claims.rs` run
//! the other conversion targets and the primitive on the left. Drawing them
//! here would change the programs the committed seeds replay.
//!
//! Every skeleton also carries a block: a value-neutral sequence of
//! fixed-charge steps that runs repeatedly after the context, over whatever
//! widths the context left. The judgment reads the block's cost per call
//! from those repetitions.
//!
//! # The budgets
//!
//! [`Skeleton::new`] holds every case to two budgets, rewriting a step that
//! would exceed either as a clear of the member it writes, decided once at
//! the base unit and so identical at every scale:
//!
//! - every value stays below [`HEIGHT_BUDGET_UNITS`] units wide, so values
//!   reach a few megabytes at the top scale and shifted self-additions
//!   cannot compound;
//! - every position a value's structure can hold stays within
//!   [`OFFSET_BUDGET_BITS`] of a multiple of the unit (see
//!   [`Skeleton::offset_bound`]), which bounds how much faster than the unit
//!   any length the implementation traverses can grow.

use proptest::prelude::*;
use suanpan_fuel_protocol::{
    Direction, Form, Integer, LimbInput, Member, Pattern, Primitive, Program, Read, SignQuery,
    Step, POOL,
};

use crate::model::{fits, width, written_member, Model};

/// The base unit `u`, in bits: 40 limbs, 80 digits.
///
/// At the top scale, `4096 · u` is about 1.3 megabytes, and the
/// [`HEIGHT_BUDGET_UNITS`] budget lets a value reach a few such units. The
/// unit is wide enough that every position that scales lies, already at the
/// base unit, past the absolute thresholds where the implementation's
/// constants change (see [`judge::growth_bound`](crate::judge::growth_bound)):
/// a unit less [`OFFSET_BUDGET_BITS`] is 72 digits, beyond the 64 digits the
/// written-position bitset's first word covers, and a unit's digits fill
/// 640 bytes, beyond the allocator's small-block bins.
pub const UNIT_BITS: u64 = 2560;

/// The widest a value may grow, in units, at the base unit.
pub const HEIGHT_BUDGET_UNITS: u64 = 3;

/// The most members one generated [`Step::Sum`] adds.
pub const MAX_SUM_TERMS: usize = 5;

/// The farthest a position in a value's structure may lie from a multiple of
/// the unit, in bits.
///
/// Fixed rather than proportional to the unit, so it bounds how close to
/// the absolute thresholds a scaled position can come (see [`UNIT_BITS`]),
/// and every length that spans at least one unit at the base unit spans
/// at least four fifths of one.
pub const OFFSET_BUDGET_BITS: u64 = 256;

/// How many times a skeleton's block runs between its context and its
/// epilogue.
///
/// The judgment reads the block's cost per call over the last
/// [`BLOCK_WINDOW`] calls, after the earlier calls have spent whatever
/// deferred work the context left.
pub const BLOCK_REPETITIONS: usize = 12;

/// The block calls, at the end of its repetitions, whose costs must agree for
/// the block's state to count as periodic.
pub const BLOCK_WINDOW: usize = 4;

/// The most steps a generated skeleton holds before its gap rounds and
/// extreme deposits expand and its epilogue is appended.
pub const MAX_STEPS: usize = 24;

/// A length of `units · u + offset` bits, or of `units · (u / 64) + offset`
/// limbs for a limb count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extent {
    /// The multiple of the unit.
    pub units: u64,
    /// The fixed part, in bits or limbs; never negative when `units` is zero.
    pub offset: i64,
}

impl Extent {
    /// Returns an extent that does not scale.
    pub const fn fixed(offset: u64) -> Extent {
        Extent {
            units: 0,
            offset: offset as i64,
        }
    }

    /// Returns the length in bits at unit `unit`.
    ///
    /// # Panics
    ///
    /// Panics if the length is negative, which the generators never produce:
    /// a negative offset comes only with a positive multiple of a unit far
    /// wider than any offset.
    pub fn bits(self, unit: u64) -> u64 {
        (self.units * unit)
            .checked_add_signed(self.offset)
            .expect("an extent's offset never exceeds its scaled part")
    }

    /// Returns the length in limbs at unit `unit`.
    ///
    /// # Panics
    ///
    /// As for [`bits`](Self::bits).
    pub fn limbs(self, unit: u64) -> u64 {
        (self.units * (unit / 64))
            .checked_add_signed(self.offset)
            .expect("a limb count's offset is never negative")
    }

    /// Returns the magnitude of the fixed part, in bits, for a bit length.
    fn offset_bits(self) -> u64 {
        self.offset.unsigned_abs()
    }
}

/// A program skeleton, normalized to the budgets: a context, a value-neutral
/// block repeated [`BLOCK_REPETITIONS`] times, and an epilogue that clears
/// every member.
#[derive(Clone, Debug)]
pub struct Skeleton {
    /// The steps: context, block repetitions, epilogue.
    steps: Vec<Step<Extent>>,
    /// The number of context steps.
    context: usize,
    /// The number of steps in one call of the block, zero for none.
    block: usize,
    /// The largest structural offset any value reaches, in bits.
    offset_bound: u64,
}

impl Skeleton {
    /// Normalizes `context` and `block` to the budgets, and assembles the
    /// skeleton.
    ///
    /// Each context step is replayed in a [`Model`] at the base unit. A step
    /// that would leave a value wider than [`HEIGHT_BUDGET_UNITS`] units, or a
    /// structural offset beyond [`OFFSET_BUDGET_BITS`], is replaced by a clear
    /// of the member it writes. The block, which must leave every value as
    /// it found it, is kept whole or dropped whole, since rewriting one of its
    /// steps would leave it no longer value-neutral; it is dropped too if a
    /// conversion in it would succeed, which would start a new storage. Its
    /// offsets count once, because every call starts from the values the
    /// first call starts from.
    ///
    /// # Panics
    ///
    /// Panics if the block changes a value.
    pub fn new(context: Vec<Step<Extent>>, block: Vec<Step<Extent>>) -> Skeleton {
        let mut model = Model::new(UNIT_BITS, false);
        let mut offsets = Offsets::default();
        let mut steps = Vec::new();
        for step in context {
            let step = if Skeleton::trial(&model, offsets, std::slice::from_ref(&step)).is_some() {
                step
            } else {
                let member = written_member(&step).expect("only a step that writes can grow");
                Step::Clear { member }
            };
            model.step(&resolve(&step, UNIT_BITS));
            offsets = offsets.after(&step);
            steps.push(step);
        }
        let context_len = steps.len();
        let (block, offsets) = match Skeleton::trial(&model, offsets, &block) {
            Some((after, block_offsets)) if Skeleton::conversions_refuse(&model, &block) => {
                assert!(
                    after.values() == model.values(),
                    "a skeleton's block leaves every value as it found it"
                );
                (block, block_offsets)
            }
            _ => (Vec::new(), offsets),
        };
        for _ in 0..BLOCK_REPETITIONS {
            steps.extend(block.iter().cloned());
        }
        steps.extend(Member::ALL.map(|member| Step::Clear { member }));
        Skeleton {
            steps,
            context: context_len,
            block: block.len(),
            offset_bound: offsets.bound,
        }
    }

    /// Replays `steps` from `model` and `offsets`, returning the model and
    /// offsets after them if every value and offset stays within the budgets,
    /// and nothing otherwise.
    fn trial(model: &Model, offsets: Offsets, steps: &[Step<Extent>]) -> Option<(Model, Offsets)> {
        let mut model = model.clone();
        let mut offsets = offsets;
        for step in steps {
            model.step(&resolve(step, UNIT_BITS));
            offsets = offsets.after(step);
            let within = model
                .values()
                .iter()
                .all(|value| width(value) <= HEIGHT_BUDGET_UNITS * UNIT_BITS)
                && offsets.max() <= OFFSET_BUDGET_BITS;
            if !within {
                return None;
            }
        }
        Some((model, offsets))
    }

    /// Returns whether every conversion in `block`, replayed from `model`,
    /// refuses because its value does not fit an `i64`.
    ///
    /// A refusal returns the accumulator as it was, keeping its storage, so a
    /// block of refusing conversions stays value-neutral and fixed-charge. It
    /// refuses at every unit when it refuses at the base unit: a value that
    /// scales is at least a unit less twice the offset budget wide at every
    /// unit, and one that does not scale is the same at every unit.
    fn conversions_refuse(model: &Model, block: &[Step<Extent>]) -> bool {
        let mut model = model.clone();
        block.iter().all(|step| {
            let refuses = match step {
                Step::Convert { member, target } => !fits(&model.values()[member.index()], *target),
                _ => true,
            };
            model.step(&resolve(step, UNIT_BITS));
            refuses
        })
    }

    /// Returns the number of context steps, which the block follows.
    pub fn context_len(&self) -> usize {
        self.context
    }

    /// Returns the number of steps in one call of the block, zero if the
    /// skeleton has none.
    pub fn block_len(&self) -> usize {
        self.block
    }

    /// Returns the steps, epilogue included.
    pub fn steps(&self) -> &[Step<Extent>] {
        &self.steps
    }

    /// Returns the largest offset, in bits, that any position in any value's
    /// structure has from a multiple of the unit.
    ///
    /// A position is where a value's bits change: a limb operand's ends, a
    /// shifted operand's structure moved by its shift, a carry out of a top.
    /// Every length the implementation traverses (a deposit span, a borrow
    /// run, a gap between written positions) runs between two such positions,
    /// so it is `d · u + δ` bits with `|δ|` at most twice this bound, which
    /// is what [`growth_bound`](crate::judge::growth_bound) rests on.
    pub fn offset_bound(&self) -> u64 {
        self.offset_bound
    }

    /// Returns the steps resolved at unit `unit`.
    pub fn steps_at(&self, unit: u64) -> Vec<Step> {
        self.steps.iter().map(|step| resolve(step, unit)).collect()
    }

    /// Returns the program that runs the steps, resolved at unit `unit`,
    /// `repetitions` times.
    pub fn program(&self, unit: u64, repetitions: u32) -> Program {
        Program {
            repetitions,
            steps: self.steps_at(unit),
        }
    }
}

/// Resolves a skeleton step at unit `unit`.
fn resolve(step: &Step<Extent>, unit: u64) -> Step {
    step.map_lengths(|extent| extent.bits(unit), |extent| extent.limbs(unit))
}

/// The structural offset bound of each member's value, and the largest any
/// value has reached.
#[derive(Clone, Copy, Debug, Default)]
struct Offsets {
    /// Each member's bound, in bits.
    members: [u64; POOL],
    /// The largest bound reached so far, in bits.
    bound: u64,
}

/// The bits a primitive operand's structure can occupy: all of a `u128`.
const PRIMITIVE_BITS: u64 = 128;

/// The offset a sum's carries can add: `⌈log₂ MAX_SUM_TERMS⌉` bits.
const SUM_CARRY_BITS: u64 = 3;

impl Offsets {
    /// Returns the largest member bound.
    fn max(&self) -> u64 {
        self.members.into_iter().max().unwrap_or(0).max(self.bound)
    }

    /// Returns the bounds after `step`.
    ///
    /// An addition's carry can move a value's top, and with it the end of a
    /// run, by one bit; a sum of up to [`MAX_SUM_TERMS`] terms by
    /// [`SUM_CARRY_BITS`]. A shift moves every position by the shift's
    /// offset. A limb operand's positions lie at its shift's offset and at
    /// that plus its count's fixed limbs.
    fn after(mut self, step: &Step<Extent>) -> Offsets {
        let o = &mut self.members;
        match step {
            Step::Primitive { member, .. } | Step::ReversedPrimitive { member, .. } => {
                let m = member.index();
                o[m] = o[m].max(PRIMITIVE_BITS) + 1;
            }
            Step::Limbs {
                member,
                shift,
                input,
                ..
            } => {
                let m = member.index();
                let operand = shift.offset_bits() + 64 * input.count.offset_bits();
                o[m] = o[m].max(operand) + 1;
            }
            Step::Operand {
                receiver,
                operand,
                form,
                ..
            } => {
                let (r, s) = (receiver.index(), operand.index());
                let moved = match form {
                    Form::Shifted(shift) => shift.offset_bits(),
                    _ => 0,
                };
                o[r] = o[r].max(o[s] + moved) + 1;
                if *form == Form::OwnedTake && r != s {
                    o[s] = 0;
                }
            }
            Step::Shift { member, shift } => {
                let m = member.index();
                o[m] += shift.offset_bits();
            }
            Step::Clear { member } => o[member.index()] = 0,
            Step::Convert { member, .. } => {
                let m = member.index();
                o[m] = o[m].max(PRIMITIVE_BITS);
            }
            Step::Clone { receiver, source } | Step::CloneFrom { receiver, source } => {
                o[receiver.index()] = o[source.index()];
            }
            Step::AbsDiff {
                receiver,
                operand,
                take,
            } => {
                let (r, s) = (receiver.index(), operand.index());
                o[r] = o[r].max(o[s]) + 1;
                if *take && r != s {
                    o[s] = 0;
                }
            }
            Step::Sum {
                receiver, members, ..
            } => {
                let terms = members.iter().map(|member| o[member.index()]).max();
                o[receiver.index()] = terms.map_or(0, |bound| bound + SUM_CARRY_BITS);
            }
            Step::Negate { .. }
            | Step::Sign { .. }
            | Step::ComparePrimitive { .. }
            | Step::Compare { .. }
            | Step::Read { .. } => {}
        }
        self.bound = self.bound.max(self.members.into_iter().max().unwrap_or(0));
        self
    }
}

/// Declares [`Kind`] and [`Kind::ALL`] from one list of kinds.
///
/// Generating both from the list makes `ALL` name every kind exactly once,
/// in declaration order, which is also the order of the discriminants that
/// index a case's swarm weights.
macro_rules! kinds {
    ($($(#[$doc:meta])* $kind:ident,)*) => {
        /// The kinds of generated step, in the order of a case's swarm
        /// weights.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Kind {
            $($(#[$doc])* $kind,)*
        }

        impl Kind {
            /// Every kind, in declaration order.
            pub const ALL: &[Kind] = &[$(Kind::$kind,)*];
        }
    };
}

kinds! {
    /// [`Step::Primitive`].
    Primitive,
    /// [`Step::Limbs`].
    Limbs,
    /// A value-neutral round of single-limb writes: `1` at `low`, if any,
    /// then at `top` and `inner`, then `−1` at `top`, `inner`, and `low`.
    /// Each cancellation of the top descends across a gap of unwritten
    /// positions.
    GapRound,
    /// Two deposits at one digit that leave the extreme coefficient
    /// `−(2^33 − 1)` there when the digit was zero.
    ParkExtreme,
    /// [`Step::Operand`].
    Operand,
    /// [`Step::Negate`].
    Negate,
    /// [`Step::Shift`].
    Shift,
    /// [`Step::Clear`].
    Clear,
    /// [`Step::Sign`].
    Sign,
    /// [`Step::ComparePrimitive`].
    ComparePrimitive,
    /// [`Step::Convert`].
    Convert,
    /// [`Step::Compare`].
    Compare,
    /// [`Step::Read`].
    Read,
    /// [`Step::Clone`].
    Clone,
    /// [`Step::CloneFrom`].
    CloneFrom,
    /// [`Step::AbsDiff`].
    AbsDiff,
    /// [`Step::Sum`].
    Sum,
}

impl Kind {
    /// The kinds that can write a nonzero value into a pool of zeros. Every
    /// case enables at least one.
    pub const WRITERS: [Kind; 4] = [
        Kind::Primitive,
        Kind::Limbs,
        Kind::GapRound,
        Kind::ParkExtreme,
    ];

    /// The kinds the fixed-charge regime draws from: none reads, copies, or
    /// produces a whole value, so once the storage has grown, each step's
    /// charge is the same at every unit apart from its logarithm.
    ///
    /// A conversion that succeeds replaces its member by a new accumulator,
    /// which starts a new storage; a later write above it grows storage
    /// again in every repetition, and the model charges that growth, so such
    /// a skeleton's steady repetition does not count as fixed-charge.
    pub const FIXED_CHARGE: [Kind; 9] = [
        Kind::Primitive,
        Kind::Limbs,
        Kind::GapRound,
        Kind::ParkExtreme,
        Kind::Negate,
        Kind::Clear,
        Kind::Sign,
        Kind::ComparePrimitive,
        Kind::Convert,
    ];
}

/// Generates a member.
fn arb_member() -> impl Strategy<Value = Member> {
    (0..POOL).prop_map(|index| Member::new(index).expect("an index below POOL names a member"))
}

/// Generates either direction.
fn arb_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![Just(Direction::Add), Just(Direction::Sub)]
}

/// Machine words at the digit, limb, and sign boundaries, the pool
/// property's list.
const BOUNDARY_WORDS: [u64; 18] = [
    0,
    1,
    2,
    3,
    4,
    5,
    (1 << 31) - 1,
    1 << 31,
    (1 << 32) - 1,
    1 << 32,
    (1 << 32) + 1,
    (1 << 33) - 1,
    1 << 33,
    3 << 31,
    (1 << 63) - 1,
    1 << 63,
    u64::MAX - 1,
    u64::MAX,
];

/// Generates machine words at the boundaries, small counts, and arbitrary
/// words.
fn arb_word() -> impl Strategy<Value = u64> {
    prop_oneof![
        3 => proptest::sample::select(BOUNDARY_WORDS.to_vec()),
        2 => 0u64..16,
        2 => any::<u64>(),
    ]
}

/// Generates signed values within `2^33` of `k · 2^(32 d)`, for `k` in 1..=4
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

/// Generates a primitive operand of any width.
fn arb_primitive() -> impl Strategy<Value = Primitive> {
    prop_oneof![
        (any::<i64>(), 0u32..64).prop_map(|(value, narrowing)| Primitive::I64(value >> narrowing)),
        arb_word().prop_map(Primitive::U64),
        prop_oneof![
            any::<i128>(),
            Just(i128::MIN),
            Just(i128::MAX),
            Just(1i128 << 96),
            arb_near_threshold(),
        ]
        .prop_map(Primitive::I128),
        prop_oneof![any::<u128>(), Just(u128::MAX), Just(1u128 << 127)].prop_map(Primitive::U128),
    ]
}

/// Offsets from a multiple of the unit at the digit and limb boundaries.
const ALIGNMENTS: [i64; 6] = [0, 1, 31, 32, 33, 63];

/// Generates a position of one to `max_units` units, misaligned by a
/// boundary offset either way, or, one time in four, a fixed position of at
/// most 128 bits.
fn arb_position(max_units: u64) -> BoxedStrategy<Extent> {
    let scaled = (
        1..=max_units,
        proptest::sample::select(ALIGNMENTS.to_vec()),
        any::<bool>(),
    )
        .prop_map(|(units, alignment, below)| Extent {
            units,
            offset: if below { -alignment } else { alignment },
        });
    prop_oneof![3 => scaled, 1 => (0u64..=128).prop_map(Extent::fixed)].boxed()
}

/// Generates a digit-aligned position of exactly `units` units, offset by
/// up to two digits either way (never below zero).
fn arb_digit(units: u64) -> impl Strategy<Value = Extent> {
    (-2i64..=2).prop_map(move |digits| Extent {
        units,
        offset: if units == 0 {
            32 * digits.abs()
        } else {
            32 * digits
        },
    })
}

/// Generates a limb count: a unit's worth of limbs or none, plus zero or one;
/// fixed when `scaled` is false.
fn arb_count(scaled: bool) -> impl Strategy<Value = Extent> {
    let units = if scaled { 0u64..=1 } else { 0u64..=0 };
    (units, 0i64..=1).prop_map(|(units, offset)| Extent { units, offset })
}

/// Generates a limb pattern.
fn arb_pattern() -> impl Strategy<Value = Pattern> {
    prop_oneof![
        Just(Pattern::Ones),
        Just(Pattern::TopBit),
        Just(Pattern::Alternating),
        Just(Pattern::Sparse),
        arb_word().prop_map(Pattern::Word),
        any::<u64>().prop_map(Pattern::Mixed),
    ]
}

/// A single limb holding 1, as a gap round writes.
const ONE_LIMB: LimbInput<Extent> = LimbInput {
    pattern: Pattern::Sparse,
    count: Extent::fixed(1),
};

/// Returns the single-limb update of `direction` at `position`.
fn unit_write(member: Member, direction: Direction, position: Extent) -> Step<Extent> {
    Step::Limbs {
        member,
        direction,
        shift: position,
        input: ONE_LIMB,
    }
}

/// Generates the steps of one generated step of `kind`; `fixed_charge`
/// restricts the step to its fixed-charge form.
fn arb_steps(kind: Kind, fixed_charge: bool) -> BoxedStrategy<Vec<Step<Extent>>> {
    let member = arb_member;
    let one = |strategy: BoxedStrategy<Step<Extent>>| strategy.prop_map(|step| vec![step]).boxed();
    match kind {
        Kind::Primitive => one((member(), arb_direction(), arb_primitive())
            .prop_map(|(member, direction, value)| Step::Primitive {
                member,
                direction,
                value,
            })
            .boxed()),
        Kind::Limbs => one((
            member(),
            arb_direction(),
            arb_position(2),
            arb_pattern(),
            arb_count(!fixed_charge),
        )
            .prop_map(|(member, direction, shift, pattern, count)| Step::Limbs {
                member,
                direction,
                shift,
                input: LimbInput { pattern, count },
            })
            .boxed()),
        Kind::GapRound => (member(), any::<bool>(), 0u64..=1)
            .prop_flat_map(move |(member, with_low, layout)| {
                // Positions in strictly increasing units, so their order
                // holds at every scale.
                let (low_units, inner_units, top_units) = (0, layout, layout + 1);
                (
                    arb_digit(low_units),
                    arb_digit(inner_units),
                    arb_digit(top_units),
                )
                    .prop_map(move |(low, inner, top)| {
                        let inner = if inner_units == low_units {
                            Extent::fixed(low.offset as u64 + 64)
                        } else {
                            inner
                        };
                        let low = with_low.then_some(low);
                        let mut steps = Vec::new();
                        steps.extend(low.map(|low| unit_write(member, Direction::Add, low)));
                        steps.push(unit_write(member, Direction::Add, top));
                        steps.push(unit_write(member, Direction::Add, inner));
                        steps.push(unit_write(member, Direction::Sub, top));
                        steps.push(unit_write(member, Direction::Sub, inner));
                        steps.extend(low.map(|low| unit_write(member, Direction::Sub, low)));
                        steps
                    })
            })
            .boxed(),
        Kind::ParkExtreme => (member(), 0u64..=2)
            .prop_flat_map(move |(member, units)| {
                arb_digit(units).prop_map(move |digit| {
                    // Subtracting 2^32 and then 2^32 - 1 at one digit, as
                    // two deposits, leaves -(2^33 - 1) on a zero digit.
                    [1u64 << 32, (1 << 32) - 1]
                        .map(|word| Step::Limbs {
                            member,
                            direction: Direction::Sub,
                            shift: digit,
                            input: LimbInput {
                                pattern: Pattern::Word(word),
                                count: Extent::fixed(1),
                            },
                        })
                        .to_vec()
                })
            })
            .boxed(),
        Kind::Operand => one((
            member(),
            member(),
            arb_direction(),
            prop_oneof![
                Just(Form::Borrowed),
                arb_position(1).prop_map(Form::Shifted),
                Just(Form::OwnedClone),
                Just(Form::OwnedTake),
                Just(Form::Reversed),
            ],
        )
            .prop_map(|(receiver, operand, direction, form)| Step::Operand {
                receiver,
                operand,
                direction,
                form,
            })
            .boxed()),
        Kind::Negate => one((
            member(),
            if fixed_charge {
                Just(false).boxed()
            } else {
                any::<bool>().boxed()
            },
        )
            .prop_map(|(member, borrowed)| Step::Negate { member, borrowed })
            .boxed()),
        Kind::Shift => one((member(), arb_position(1))
            .prop_map(|(member, shift)| Step::Shift { member, shift })
            .boxed()),
        Kind::Clear => one(member().prop_map(|member| Step::Clear { member }).boxed()),
        Kind::Sign => one((
            member(),
            prop_oneof![
                Just(SignQuery::Sign),
                Just(SignQuery::IsZero),
                Just(SignQuery::IsPositive),
                Just(SignQuery::IsNegative),
                arb_position(4).prop_map(SignQuery::StableUnder),
            ],
        )
            .prop_map(|(member, query)| Step::Sign { member, query })
            .boxed()),
        Kind::ComparePrimitive => one((
            member(),
            prop_oneof![arb_near_threshold(), any::<i64>().prop_map(i128::from)],
        )
            .prop_map(|(member, value)| Step::ComparePrimitive { member, value })
            .boxed()),
        Kind::Convert => one(member()
            .prop_map(|member| Step::Convert {
                member,
                target: Integer::I64,
            })
            .boxed()),
        Kind::Compare => one((member(), member())
            .prop_map(|(left, right)| Step::Compare { left, right })
            .boxed()),
        Kind::Read => one((
            member(),
            prop_oneof![Just(Read::Signed), Just(Read::Scaled), Just(Read::Hash)],
        )
            .prop_map(|(member, read)| Step::Read { member, read })
            .boxed()),
        Kind::Clone => one((member(), member())
            .prop_map(|(receiver, source)| Step::Clone { receiver, source })
            .boxed()),
        Kind::CloneFrom => one((member(), member())
            .prop_map(|(receiver, source)| Step::CloneFrom { receiver, source })
            .boxed()),
        Kind::AbsDiff => one((member(), member(), any::<bool>())
            .prop_map(|(receiver, operand, take)| Step::AbsDiff {
                receiver,
                operand,
                take,
            })
            .boxed()),
        Kind::Sum => one((
            member(),
            proptest::collection::vec(member(), 0..=MAX_SUM_TERMS),
            any::<bool>(),
        )
            .prop_map(|(receiver, members, owned)| Step::Sum {
                receiver,
                members,
                owned,
            })
            .boxed()),
    }
}

/// Generates one element of a block: a fixed-charge sequence of steps that
/// leaves every value as it found it.
///
/// Each element is an update and its inverse, a value-neutral gap round or
/// extreme-digit deposit and its inverse, a question about a sign or an
/// order, a conversion that refuses, or a negation applied twice. Its limb
/// operands have fixed counts,
/// though their positions scale, so once storage is grown no step of it
/// charges a term that grows with the unit, whatever the context left in
/// the members it touches.
fn arb_block_element() -> BoxedStrategy<Vec<Step<Extent>>> {
    let member = arb_member;
    prop_oneof![
        (member(), arb_primitive()).prop_map(|(member, value)| {
            [Direction::Add, Direction::Sub]
                .map(|direction| Step::Primitive {
                    member,
                    direction,
                    value,
                })
                .to_vec()
        }),
        (member(), arb_position(2), arb_pattern(), 1u64..=2).prop_map(
            |(member, shift, pattern, count)| {
                [Direction::Add, Direction::Sub]
                    .map(|direction| Step::Limbs {
                        member,
                        direction,
                        shift,
                        input: LimbInput {
                            pattern,
                            count: Extent::fixed(count),
                        },
                    })
                    .to_vec()
            }
        ),
        arb_steps(Kind::GapRound, true),
        arb_steps(Kind::ParkExtreme, true).prop_map(|park| {
            // Undo the two deposits in reverse order: the value is restored.
            let undo = park.iter().rev().map(|step| match step {
                Step::Limbs {
                    member,
                    shift,
                    input,
                    ..
                } => Step::Limbs {
                    member: *member,
                    direction: Direction::Add,
                    shift: *shift,
                    input: *input,
                },
                other => other.clone(),
            });
            park.iter().cloned().chain(undo).collect()
        }),
        arb_steps(Kind::Sign, true),
        arb_steps(Kind::ComparePrimitive, true),
        member().prop_map(|member| {
            // A large addend around the conversion makes it refuse unless
            // the value nearly cancels it; `Skeleton::new` drops a block
            // whose conversion would succeed.
            let addend = |direction| Step::Primitive {
                member,
                direction,
                value: Primitive::U128(1 << 100),
            };
            vec![
                addend(Direction::Add),
                Step::Convert {
                    member,
                    target: Integer::I64,
                },
                addend(Direction::Sub),
            ]
        }),
        member().prop_map(|member| vec![
            Step::Negate {
                member,
                borrowed: false
            };
            2
        ]),
    ]
    .boxed()
}

/// Generates a block: a few elements, possibly none, each value-neutral, so
/// the block is too.
fn arb_block() -> impl Strategy<Value = Vec<Step<Extent>>> {
    proptest::collection::vec(arb_block_element(), 0..=3)
        .prop_map(|elements| elements.into_iter().flatten().collect())
}

/// Generates a skeleton, normalized by [`Skeleton::new`].
///
/// A case draws a regime, a swarm of enabled kinds with weights, a context
/// of up to [`MAX_STEPS`] generated steps drawn from that swarm, and a block
/// of value-neutral, fixed-charge elements.
///
/// One case in four draws from the fixed-charge regime ([`Kind::FIXED_CHARGE`]).
/// Each kind is disabled in about a third of cases and otherwise weighted
/// from 1 to 5; a case that disables every writer enables [`Kind::Limbs`].
pub fn arb_skeleton() -> impl Strategy<Value = Skeleton> {
    let weights = proptest::array::uniform::<_, { Kind::ALL.len() }>(
        prop_oneof![1 => Just(0u32), 2 => 1u32..6],
    );
    (prop_oneof![3 => Just(false), 1 => Just(true)], weights).prop_flat_map(
        |(fixed_charge, mut weights)| {
            if fixed_charge {
                for kind in Kind::ALL {
                    if !Kind::FIXED_CHARGE.contains(kind) {
                        weights[*kind as usize] = 0;
                    }
                }
            }
            if Kind::WRITERS
                .iter()
                .all(|&kind| weights[kind as usize] == 0)
            {
                weights[Kind::Limbs as usize] = 3;
            }
            let arms: Vec<(u32, BoxedStrategy<Vec<Step<Extent>>>)> = Kind::ALL
                .iter()
                .copied()
                .filter(|&kind| weights[kind as usize] > 0)
                .map(|kind| (weights[kind as usize], arb_steps(kind, fixed_charge)))
                .collect();
            (
                proptest::collection::vec(
                    proptest::strategy::Union::new_weighted(arms),
                    1..=MAX_STEPS,
                ),
                arb_block(),
            )
                .prop_map(|(groups, block)| {
                    Skeleton::new(groups.into_iter().flatten().collect(), block)
                })
        },
    )
}
