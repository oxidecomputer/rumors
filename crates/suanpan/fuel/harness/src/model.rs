//! An exact model of a program's pool, which prices every step under the
//! crate's four cost rules.
//!
//! [`Model`] replays steps on `BigInt` values, independent of `suanpan`, and
//! returns a [`Record`] per step: the step's [`Charge`], which is what the
//! rules say it may cost; the answer the guest must observe; a floor on the
//! fuel the step must spend; and the hard regimes it realized.
//!
//! # The charge
//!
//! The crate's precise statement bounds each operation by
//! `c · (1 + in + out + Δh + σ · log₆₄(h + 1))`, in 64-bit words: `in` the
//! words it reads from operands, `out` the words it produces, `Δh` how far it
//! raises its storage's width `h`, and `σ` 1 for an update at a nonzero
//! shift. Composite steps are charged part by part, as the statement
//! decomposes them: a clone and then an owned operator, a negation and then
//! an addition, one addition per term of a sum.
//!
//! The rules define `h` as the highest bit written since the storage began,
//! which only the implementation sees. The model tracks the running maximum
//! of the value's width instead. An update writes no higher than a few
//! digits above the wider of the value before and after it (`suanpan`'s cost
//! derivation, "Where updates write"), so the two differ by a constant. An
//! owned operator keeps the storage of the operand with more stored digits;
//! where the model cannot tell which that is, it takes the lower storage
//! width (see `owned_height`). So the modeled `h` never exceeds the
//! implementation's by more than a constant, and where the model is unsure
//! it errs low, charging growth that may not happen rather than missing
//! growth that does. That is the direction the property needs: the charge
//! decides whether a skeleton's steady repetition charges any term that
//! grows with the unit, and it states readings per unit of charge in failure
//! messages.

use std::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan_fuel_protocol::{
    observe_bool, observe_conversion, observe_magnitude, observe_ordering, observe_stable,
    Direction, Form, Integer, LimbInput, Member, Primitive, Read, SignQuery, Step, NO_OBSERVATION,
    POOL,
};

/// What the cost rules charge one step, in their own units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Charge {
    /// `in`: the 64-bit words read from operands.
    pub input_words: u64,
    /// `out`: the 64-bit words produced.
    pub output_words: u64,
    /// `Δh`: how far the step raised its storage's width, in bits.
    pub growth_bits: u64,
    /// `σ`: whether the step is an update at a nonzero shift.
    pub shifted: bool,
    /// `h`: the width of the storage the step updated, after it, in bits.
    pub height_bits: u64,
}

impl Charge {
    /// Returns the charge in words: `1 + in + out + Δh / 64 + σ · log₆₄(h + 1)`.
    pub fn words(&self) -> f64 {
        let logarithm = if self.shifted {
            ((self.height_bits + 1) as f64).log(64.0)
        } else {
            0.0
        };
        1.0 + self.input_words as f64
            + self.output_words as f64
            + self.growth_bits as f64 / 64.0
            + logarithm
    }

    /// Returns the terms that grow with the operands' sizes: `in`, `out`,
    /// and `Δh`, the terms a step without one charges the same at every
    /// unit.
    pub fn sized_terms(&self) -> (u64, u64, u64) {
        (self.input_words, self.output_words, self.growth_bits)
    }
}

/// The hard regimes one step realized, each measured against the unit `u`
/// the model was built with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// An additive step whose receiver and contribution were nonzero with
    /// opposite signs.
    pub mixed_sign: bool,
    /// An additive step whose result is at least `u / 2` bits narrower than
    /// the wider of its receiver and contribution.
    pub cancellation: bool,
    /// A mixed-sign step whose result holds a run of one bits at least
    /// `u / 2` longer than the longest run in its receiver or contribution:
    /// a borrow that crossed a long span of digits.
    pub borrow_run: bool,
    /// A step that raised its storage's width by at least `u / 2` bits.
    pub growth: bool,
    /// A step after which the receiver's magnitude holds a run of at least
    /// `u` zero bits between two one bits: a sparse high digit.
    pub sparse: bool,
    /// An update at a nonzero shift.
    pub shifted: bool,
}

/// What the model knows about one step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Record {
    /// What the cost rules charge the step.
    pub charge: Charge,
    /// The observation word the guest must report for the step.
    pub observation: u64,
    /// Whether the guest may report `cmp_zero_stable_under`'s refusal
    /// instead: that method's `None` claims nothing, so it is a correct
    /// answer whatever the value, and only an order is a promise.
    pub may_refuse: bool,
    /// A floor on the fuel the step spends.
    ///
    /// The floor counts one for the step itself; one for each limb of a limb
    /// update's operand; and one for each limb of the value that a
    /// `signed_magnitude` read returns, a hash consumes, or a clone,
    /// `clone_from`, borrowed negation, or borrowed primitive on the left
    /// copies. The step reads or writes each of those limbs at least once, and
    /// each such access costs at least one wasm instruction or one byte of bulk
    /// memory traffic. Other work counts nothing toward the floor, which only
    /// proves that fuel is counting: a scaled read, which may leave out a span
    /// no update wrote, and the clones inside accumulator operands, sums, and
    /// `abs_diff` add nothing.
    pub floor: u64,
    /// The hard regimes the step realized; all false unless the model counts
    /// them.
    pub events: Events,
}

/// The pool's exact values and the model's storage widths.
#[derive(Clone, Debug)]
pub struct Model {
    /// The unit `u`, in bits, against which events are measured.
    unit: u64,
    /// Whether to detect [`Events`], which costs a pass over every result.
    census: bool,
    /// Each member's value.
    values: [BigInt; POOL],
    /// Each member's storage width `h`, in bits, as the module docs model it.
    heights: [u64; POOL],
}

/// Returns a value's bit length, zero for zero.
pub fn width(value: &BigInt) -> u64 {
    value.bits()
}

/// Returns the words the rules count for reading or producing a value of
/// `bits` bits: `1 + bits / 64`.
fn words(bits: u64) -> u64 {
    1 + bits / 64
}

/// Returns the number of 64-bit limbs a value of `bits` bits occupies.
fn limbs(bits: u64) -> u64 {
    bits.div_ceil(64)
}

/// Returns the sign of a value as the accumulator reports it.
fn sign(value: &BigInt) -> Ordering {
    match value.sign() {
        Sign::Minus => Ordering::Less,
        Sign::NoSign => Ordering::Equal,
        Sign::Plus => Ordering::Greater,
    }
}

/// Returns a primitive operand's value.
fn primitive(value: Primitive) -> BigInt {
    match value {
        Primitive::I64(v) => BigInt::from(v),
        Primitive::U64(v) => BigInt::from(v),
        Primitive::I128(v) => BigInt::from(v),
        Primitive::U128(v) => BigInt::from(v),
    }
}

/// Returns a limb operand's value.
fn limb_value(input: &LimbInput) -> BigUint {
    let digits: Vec<u32> = input
        .expand()
        .into_iter()
        .flat_map(|limb| [limb as u32, (limb >> 32) as u32])
        .collect();
    BigUint::new(digits)
}

/// Returns whether `value` fits the primitive type `target`.
pub fn fits(value: &BigInt, target: Integer) -> bool {
    match target {
        Integer::I64 => i64::try_from(value).is_ok(),
        Integer::U64 => u64::try_from(value).is_ok(),
        Integer::I128 => i128::try_from(value).is_ok(),
        Integer::U128 => u128::try_from(value).is_ok(),
    }
}

/// Applies a direction to a contribution.
fn directed(direction: Direction, contribution: BigInt) -> BigInt {
    match direction {
        Direction::Add => contribution,
        Direction::Sub => -contribution,
    }
}

/// Returns the length of the longest run of one bits in `word`.
fn longest_ones_in(mut word: u64) -> u64 {
    // Each round shortens every run by one, so the rounds until none is left
    // count the longest run.
    let mut length = 0;
    while word != 0 {
        word &= word << 1;
        length += 1;
    }
    length
}

/// The longest run of one bits and the longest run of zero bits strictly
/// inside a magnitude (between its lowest and highest one bits).
fn runs(value: &BigInt) -> (u64, u64) {
    let (mut longest_ones, mut longest_inner_zeros) = (0u64, 0u64);
    // The runs of ones and of zeros that reach the top of the limbs seen so
    // far, and whether a one bit lies below the zero run.
    let (mut ones, mut zeros, mut seen_one) = (0u64, 0u64, false);
    for limb in value.magnitude().iter_u64_digits() {
        if limb == 0 {
            zeros += 64;
            longest_ones = longest_ones.max(ones);
            ones = 0;
            continue;
        }
        let (low_zeros, high_zeros) = (limb.trailing_zeros(), limb.leading_zeros());
        if seen_one {
            longest_inner_zeros = longest_inner_zeros.max(zeros + u64::from(low_zeros));
        }
        // Zero runs between this limb's lowest and highest one bits.
        let span = 64 - low_zeros - high_zeros;
        let inside = (!limb >> low_zeros) & (u64::MAX >> (64 - span));
        longest_inner_zeros = longest_inner_zeros.max(longest_ones_in(inside));
        zeros = u64::from(high_zeros);
        seen_one = true;

        if limb == u64::MAX {
            ones += 64;
        } else {
            longest_ones = longest_ones
                .max(ones + u64::from(limb.trailing_ones()))
                .max(longest_ones_in(limb));
            ones = u64::from(limb.leading_ones());
        }
    }
    (longest_ones.max(ones), longest_inner_zeros)
}

impl Model {
    /// Returns a model of a pool of zeros, measuring events against `unit`
    /// bits and detecting them only if `census`.
    pub fn new(unit: u64, census: bool) -> Model {
        Model {
            unit,
            census,
            values: Default::default(),
            heights: [0; POOL],
        }
    }

    /// Returns each member's value.
    pub fn values(&self) -> &[BigInt; POOL] {
        &self.values
    }

    /// Returns each member's modeled storage width, in bits.
    pub fn heights(&self) -> &[u64; POOL] {
        &self.heights
    }

    /// Raises member `m`'s storage width to its value's width, returning the
    /// rise.
    fn grow(&mut self, m: usize) -> u64 {
        let reached = self.heights[m].max(width(&self.values[m]));
        let growth = reached - self.heights[m];
        self.heights[m] = reached;
        growth
    }

    /// Starts member `m`'s storage again at its value's width, as a clone,
    /// `clone_from`, a nonzero shift, or a construction does.
    fn restart(&mut self, m: usize) {
        self.heights[m] = width(&self.values[m]);
    }

    /// Records the events of an additive step on member `m`, from its value
    /// `before` and the `contribution` it received.
    fn additive_events(
        &self,
        events: &mut Events,
        before: &BigInt,
        contribution: &BigInt,
        m: usize,
    ) {
        if !self.census {
            return;
        }
        let after = &self.values[m];
        let opposite = matches!(
            (sign(before), sign(contribution)),
            (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less)
        );
        events.mixed_sign |= opposite;
        let half = self.unit / 2;
        events.cancellation |= width(after) + half <= width(before).max(width(contribution));
        if opposite {
            let (after_ones, _) = runs(after);
            let (before_ones, _) = runs(before);
            let (contribution_ones, _) = runs(contribution);
            events.borrow_run |= after_ones >= before_ones.max(contribution_ones) + half;
        }
    }

    /// Records the events every writing step shares: storage growth and a
    /// sparse result on member `m`.
    fn written_events(&self, events: &mut Events, growth: u64, m: usize) {
        if !self.census {
            return;
        }
        events.growth |= growth >= self.unit / 2;
        let (_, inner_zeros) = runs(&self.values[m]);
        events.sparse |= inner_zeros >= self.unit;
    }

    /// Replays one step and returns what the model knows about it.
    pub fn step(&mut self, step: &Step) -> Record {
        let mut charge = Charge::default();
        let mut events = Events::default();
        let mut observation = NO_OBSERVATION;
        let mut may_refuse = false;
        let mut floor = 1;
        // The member whose storage the step updates, if it writes one.
        let mut written = None;
        match step {
            Step::Primitive {
                member,
                direction,
                value,
            } => {
                let m = member.index();
                let contribution = directed(*direction, primitive(*value));
                let before = self.values[m].clone();
                self.values[m] += &contribution;
                charge.input_words = 1;
                self.additive_events(&mut events, &before, &contribution, m);
                written = Some(m);
            }
            Step::Limbs {
                member,
                direction,
                shift,
                input,
            } => {
                let m = member.index();
                let contribution = directed(*direction, BigInt::from(limb_value(input) << *shift));
                let before = self.values[m].clone();
                self.values[m] += &contribution;
                charge.input_words = input.count;
                charge.shifted = *shift != 0;
                floor += input.count;
                self.additive_events(&mut events, &before, &contribution, m);
                written = Some(m);
            }
            Step::Operand {
                receiver,
                operand,
                direction,
                form,
            } => {
                let (r, o) = (receiver.index(), operand.index());
                let before = self.values[r].clone();
                self.operand(&mut charge, r, o, *direction, *form);
                let contribution = &self.values[r] - &before;
                self.additive_events(&mut events, &before, &contribution, r);
                written = Some(r);
            }
            Step::Negate { member, borrowed } => {
                let m = member.index();
                self.values[m] = -std::mem::take(&mut self.values[m]);
                if *borrowed {
                    charge.output_words = words(width(&self.values[m]));
                    floor += limbs(width(&self.values[m]));
                    self.restart(m);
                } else {
                    charge.output_words = 1;
                }
            }
            Step::ReversedPrimitive {
                member,
                direction,
                value,
                borrowed,
            } => {
                let m = member.index();
                let before = self.values[m].clone();
                if *borrowed {
                    // `value ± &member` clones the member, a new storage of
                    // its width, and then updates the clone.
                    let bits = width(&before);
                    charge.output_words = words(bits);
                    floor += limbs(bits);
                    self.restart(m);
                }
                // `value - member` negates the member first.
                if *direction == Direction::Sub {
                    charge.output_words += 1;
                    self.values[m] = -std::mem::take(&mut self.values[m]);
                }
                let negated = self.values[m].clone();
                let contribution = primitive(*value);
                self.values[m] += &contribution;
                charge.input_words = 1;
                self.additive_events(&mut events, &negated, &contribution, m);
                written = Some(m);
            }
            Step::Shift { member, shift } => {
                let m = member.index();
                self.values[m] <<= *shift;
                charge.output_words = words(width(&self.values[m]));
                if *shift != 0 && sign(&self.values[m]) != Ordering::Equal {
                    self.restart(m);
                }
            }
            Step::Clear { member } => {
                self.values[member.index()] = BigInt::ZERO;
            }
            Step::Sign { member, query } => {
                let value = &self.values[member.index()];
                charge.output_words = 1;
                observation = match *query {
                    SignQuery::Sign => observe_ordering(sign(value)),
                    SignQuery::IsZero => observe_bool(sign(value) == Ordering::Equal),
                    SignQuery::IsPositive => observe_bool(sign(value) == Ordering::Greater),
                    SignQuery::IsNegative => observe_bool(sign(value) == Ordering::Less),
                    SignQuery::StableUnder(bits) => {
                        // The order is promised only when the magnitude is at
                        // least 2^bits.
                        may_refuse = true;
                        let stable = sign(value) != Ordering::Equal && width(value) > bits;
                        observe_stable(stable.then(|| sign(value)))
                    }
                };
            }
            Step::ComparePrimitive { member, value } => {
                charge.input_words = 1;
                charge.output_words = 1;
                observation =
                    observe_ordering(self.values[member.index()].cmp(&BigInt::from(*value)));
            }
            Step::Convert { member, target } => {
                let m = member.index();
                charge.output_words = 1;
                let value = &self.values[m];
                let converted = fits(value, *target).then(|| {
                    let magnitude = u128::try_from(value.magnitude())
                        .expect("a value that fits a primitive has a magnitude below 2^128");
                    (sign(value), magnitude)
                });
                if converted.is_some() {
                    self.restart(m);
                }
                observation = observe_conversion(converted);
            }
            Step::Compare { left, right } => {
                let (l, r) = (&self.values[left.index()], &self.values[right.index()]);
                charge.input_words = words(width(l).min(width(r)));
                charge.output_words = 1;
                observation = observe_ordering(l.cmp(r));
            }
            Step::Read { member, read } => {
                let value = &self.values[member.index()];
                let bits = width(value);
                charge.output_words = words(bits);
                observation = match read {
                    Read::Signed => {
                        floor += limbs(bits);
                        let low = value.magnitude().iter_u64_digits().next().unwrap_or(0);
                        observe_magnitude(sign(value), bits, low)
                    }
                    Read::Scaled => observe_magnitude(sign(value), bits, 0),
                    Read::Hash => {
                        floor += limbs(bits);
                        NO_OBSERVATION
                    }
                };
            }
            Step::Clone { receiver, source } | Step::CloneFrom { receiver, source } => {
                let (r, s) = (receiver.index(), source.index());
                self.values[r] = self.values[s].clone();
                let bits = width(&self.values[r]);
                charge.output_words = words(bits);
                floor += limbs(bits);
                self.restart(r);
            }
            Step::AbsDiff {
                receiver,
                operand,
                take,
            } => {
                let (r, o) = (receiver.index(), operand.index());
                let (wr, wo) = (width(&self.values[r]), width(&self.values[o]));
                let taken = *take && r != o;
                if !taken {
                    // The operand is a clone, a new storage of its width.
                    charge.output_words += words(wo);
                }
                charge.input_words = words(wr.min(wo));
                // The result negates a negative difference: one more word.
                charge.output_words += 1;
                observation = observe_ordering(self.values[r].cmp(&self.values[o]));
                let before = self.values[r].clone();
                let contribution = -self.values[o].clone();
                let operand_height = if taken { self.heights[o] } else { wo };
                let height = owned_height(wr, self.heights[r], wo, operand_height);
                self.values[r] = BigInt::from((&before + &contribution).magnitude().clone());
                self.heights[r] = height;
                if taken {
                    self.values[o] = BigInt::ZERO;
                    self.heights[o] = 0;
                }
                self.additive_events(&mut events, &before, &contribution, r);
                written = Some(r);
            }
            Step::Sum {
                receiver,
                members,
                owned,
            } => {
                let r = receiver.index();
                let mut total = BigInt::ZERO;
                let mut height = 0;
                for (index, member) in members.iter().enumerate() {
                    let term = &self.values[member.index()];
                    let bits = width(term);
                    let kept = if !*owned {
                        // A borrowed sum adds each term into the new
                        // accumulator it starts with, keeping that storage.
                        charge.input_words += words(bits);
                        height
                    } else if index == 0 {
                        // An owned sum reduces clones with owned additions:
                        // the first clone is the running total, a storage
                        // of its own width.
                        charge.output_words += words(bits);
                        bits
                    } else {
                        // Each later clone, a storage of its width, meets the
                        // total in an owned addition, which reads the
                        // narrower and keeps the storage `owned_height`
                        // chooses.
                        charge.output_words += words(bits);
                        let total_bits = width(&total);
                        charge.input_words += words(bits.min(total_bits));
                        owned_height(total_bits, height, bits, bits)
                    };
                    total += term;
                    height = kept.max(width(&total));
                    charge.growth_bits += height - kept;
                }
                self.values[r] = total;
                self.heights[r] = height;
            }
        }
        if let Some(m) = written {
            charge.growth_bits = self.grow(m);
            self.written_events(&mut events, charge.growth_bits, m);
        }
        if let Some(member) = written_member(step) {
            charge.height_bits = self.heights[member.index()];
        }
        events.shifted = charge.shifted;
        Record {
            charge,
            observation,
            may_refuse,
            floor,
            events,
        }
    }

    /// Applies an accumulator operand to member `r` in `form`, charging the
    /// parts the crate's precise statement decomposes it into.
    fn operand(
        &mut self,
        charge: &mut Charge,
        r: usize,
        o: usize,
        direction: Direction,
        form: Form,
    ) {
        let (wr, wo) = (width(&self.values[r]), width(&self.values[o]));
        let operand = self.values[o].clone();
        if r == o {
            // Every form but `Borrowed` lends a clone of the member; the
            // borrowed `&v ± &v` clones its left operand.
            charge.output_words = words(wr);
            charge.input_words = words(wr);
            match form {
                Form::Borrowed => {
                    self.values[r] = match direction {
                        Direction::Add => &operand + &operand,
                        Direction::Sub => BigInt::ZERO,
                    };
                    // The result keeps the clone's storage.
                    self.restart(r);
                    self.heights[r] = self.heights[r].max(wr);
                }
                Form::Shifted(shift) => {
                    charge.shifted = shift != 0;
                    self.values[r] += directed(direction, operand << shift);
                }
                Form::Reversed if direction == Direction::Sub => {
                    // `&copy - take(v)` negates the receiver first.
                    charge.output_words += 1;
                    self.values[r] = BigInt::ZERO;
                }
                Form::OwnedClone | Form::OwnedTake => {
                    // The clone has the receiver's width, so either storage
                    // may be kept (see `owned_height`).
                    self.heights[r] = owned_height(wr, self.heights[r], wr, wr);
                    self.values[r] += directed(direction, operand);
                }
                Form::Reversed => {
                    self.values[r] += directed(direction, operand);
                }
            }
            return;
        }
        match form {
            Form::Borrowed => {
                charge.input_words = words(wo);
                self.values[r] += directed(direction, operand);
            }
            Form::Shifted(shift) => {
                charge.input_words = words(wo);
                charge.shifted = shift != 0;
                self.values[r] += directed(direction, operand << shift);
            }
            Form::OwnedClone | Form::OwnedTake => {
                let taken = form == Form::OwnedTake;
                if !taken {
                    charge.output_words = words(wo);
                }
                charge.input_words = words(wr.min(wo));
                // The operand's storage is the taken member's, or the
                // clone's, which starts at its width.
                let operand_height = if taken { self.heights[o] } else { wo };
                self.heights[r] = owned_height(wr, self.heights[r], wo, operand_height);
                self.values[r] += directed(direction, operand);
                if taken {
                    self.values[o] = BigInt::ZERO;
                    self.heights[o] = 0;
                }
            }
            Form::Reversed => {
                charge.input_words = words(wo);
                if direction == Direction::Sub {
                    // `&o - take(r)` is `-take(r) + &o`: a negation first.
                    charge.output_words = 1;
                }
                let receiver = std::mem::take(&mut self.values[r]);
                self.values[r] = match direction {
                    Direction::Add => operand + receiver,
                    Direction::Sub => operand - receiver,
                };
            }
        }
    }
}

/// Returns the storage width an owned addition's or `abs_diff`'s result
/// keeps, from the widths and storage widths of its receiver and its operand.
///
/// Both read one operand into the other's storage, choosing by stored digit
/// counts: the receiver keeps its storage unless the operand holds more
/// digits, after compacting whichever holds more. The operand that keeps its
/// storage has been compacted, so its count exceeds its value's digit count
/// by at most one, and any count is at least its value's digit count less
/// one; so the operand that gives up its storage has at most
/// [`DIGIT_SLACK`] more value digits than the one that keeps it (owned
/// addition first returns a known zero's partner, which the same bound
/// covers). The
/// model sees value widths, not stored forms, so when the two values' digit
/// counts lie within [`DIGIT_SLACK`] of each other either storage may
/// survive, and the model takes the lower storage width: that can only
/// charge more growth later, never hide growth an implementation pays again.
/// Beyond that slack, the operand with more digits keeps its storage.
fn owned_height(
    receiver_width: u64,
    receiver_height: u64,
    operand_width: u64,
    operand_height: u64,
) -> u64 {
    let (receiver, operand) = (digits(receiver_width), digits(operand_width));
    if receiver.abs_diff(operand) <= DIGIT_SLACK {
        receiver_height.min(operand_height)
    } else if receiver > operand {
        receiver_height
    } else {
        operand_height
    }
}

/// How many more value digits the operand an owned addition reads can have
/// than the operand whose storage it keeps.
const DIGIT_SLACK: u64 = 2;

/// Returns the 32-bit digit positions a value of `bits` bits occupies, at
/// least one, as an accumulator's working width counts them.
fn digits(bits: u64) -> u64 {
    bits.div_ceil(32).max(1)
}

/// Returns the member a step writes, if it writes one: the storage a step's
/// logarithm is charged against, and the member a budget rewrite clears in
/// the step's place.
pub fn written_member(step: &Step<impl Sized>) -> Option<Member> {
    match step {
        Step::Primitive { member, .. }
        | Step::Limbs { member, .. }
        | Step::Negate { member, .. }
        | Step::ReversedPrimitive { member, .. }
        | Step::Shift { member, .. }
        | Step::Clear { member }
        | Step::Convert { member, .. } => Some(*member),
        Step::Operand { receiver, .. }
        | Step::Clone { receiver, .. }
        | Step::CloneFrom { receiver, .. }
        | Step::AbsDiff { receiver, .. }
        | Step::Sum { receiver, .. } => Some(*receiver),
        Step::Sign { .. }
        | Step::ComparePrimitive { .. }
        | Step::Compare { .. }
        | Step::Read { .. } => None,
    }
}
