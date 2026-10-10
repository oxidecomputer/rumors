//! Frozen references for the running leaf height.
//!
//! The surrounding fold keeps recent height changes in one live accumulator
//! `L`. Freezing moves `L` into a new *reference*: the height `F_j`, measured
//! from the first leaf's height, that the leaves had reached when it froze. A
//! leaf is then recorded as an offset from the newest reference, and a
//! subtree minimum keeps that offset until it is settled.
//!
//! # Two jobs, two rules
//!
//! Freezing serves two purposes that want opposite schedules. A stored
//! offset should be narrow, which wants eager freezes; each freeze creates a
//! reference to retain, which wants rare ones. This module separates them.
//!
//! - The *record trigger* in [`HeightReferences::add_leaf`] freezes whenever
//!   `L` is wider than the change that reached the leaf by more than
//!   [`RECORD_SLACK_BITS`]. Every stored offset is therefore at most that
//!   many bits wider than its own leaf's encoded change.
//! - The top reference, which receives every new leaf, stays even while no
//!   record refers to it; a reference below the top is retained only while a
//!   live minimum record refers to it. Records form a stack in leaf order,
//!   and settlement always removes the top record, so the references in use
//!   are monotone along the record stack. Settling the last record at the
//!   top reference only marks it unreferenced. Settling the last record at a
//!   reference below the top releases it: it is then the second-highest
//!   reference, and it merges into the top. A freeze that finds the top
//!   reference unreferenced merges it into the new top at once.
//!
//! Each reference `j` stores its *chunk* `F_j - F_{j-1}` (with `F_{-1} = 0`)
//! and a coefficient: the leaves recorded at it, less the subtree closes
//! charged to it, plus coefficients moved to it by merges. The fold owes
//! `Σ_j coefficient_j · F_j`. Removing reference `j`, between an older
//! neighbour `i` and the top `t`, keeps both neighbours' heights and moves
//! its coefficient using either identity
//!
//! `c_j · F_j = c_j · F_i + c_j · chunk_j = c_j · F_t − c_j · chunk_t`.
//!
//! A removed reference's coefficient is final, because no record refers to
//! it and it receives no further leaves. Below the oldest reference the
//! height is zero, so a coefficient moved there is settled outright.
//!
//! # The first leaf's height
//!
//! Every coefficient counts leaves and subtree closes, and a binary tree has
//! one more leaf than internal nodes, so the coefficients sum to one when the
//! fold ends. The first leaf's height `h_0` lies under every reference, so it
//! contributes exactly `h_0` to the answer. The fold adds it once at the
//! start, and the references measure only the height above it.
//!
//! # Why the cost stays linear
//!
//! A merge adds the narrower chunk into the wider one's storage and moves the
//! coefficient through whichever chunk is narrower, so it reads only the
//! narrower operand. The sum is at most one digit wider than the wider
//! operand, so the total stored width of live chunks falls by at least the
//! width read, less one digit. Freezes add only drifts the input paid for.
//! Total merge work is therefore linear in the input. Burying the top
//! reference and exposing it again cost a constant: a chunk is copied only
//! when its working width is at most 64 bits, and wider chunks move whole
//! without being read.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan::Accumulator;

use crate::bits::stack::{BitStack, PackedU64Stack};

/// Bits a stored offset may exceed its leaf's height change by before the
/// live change freezes into a new reference.
///
/// A smaller allowance narrows every stored minimum and creates references
/// more often; each reference needs about `2 · RECORD_SLACK_BITS` input bits
/// of height change, so both costs stay proportional to the input.
pub const RECORD_SLACK_BITS: u64 = 4;

#[cfg(test)]
thread_local! {
    /// Record-trigger freezes on this test thread.
    pub static RECORD_FREEZES: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// Reference removals that moved the coefficient to the newer neighbour.
    pub static MERGES_TO_NEWER: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// Reference removals that moved the coefficient to the older neighbour.
    pub static MERGES_TO_OLDER: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// References buried with a chunk wider than one inline word.
    pub static WIDE_CHUNKS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// The most references held below the top at once.
    pub static MAX_BURIED: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// Bump a test-thread counter.
#[cfg(test)]
fn bump(counter: &'static std::thread::LocalKey<core::cell::Cell<u64>>) {
    counter.with(|hits| hits.set(hits.get() + 1));
}

/// One leaf height expressed relative to the newest reference.
pub struct LeafHeight {
    /// Signed height change since the newest reference froze.
    offset: BigInt,
}

impl LeafHeight {
    /// Signed height change since the newest reference froze.
    pub fn offset(&self) -> &BigInt {
        &self.offset
    }
}

/// The newest reference, which receives every new leaf.
struct Top {
    /// `F_t - F_{t-1}`, or `F_t` when no reference lies below.
    chunk: Accumulator,
    /// Leaves recorded here, less closes settled here, plus moved coefficients.
    coefficient: i128,
    /// Whether a live record refers to this reference.
    referenced: bool,
}

/// The references of the running height and their coefficients.
pub struct HeightReferences {
    /// The newest reference.
    top: Top,
    /// Coefficient of the reference directly below the top, if one exists.
    /// It is explicit because settlements below the top charge it.
    second_coefficient: Option<i128>,
    /// Chunks of every reference below the top, oldest first.
    chunks: ChunkStack,
    /// Coefficients of the references below the second, oldest first, as
    /// zigzag codes split into two words.
    coefficients: PackedU64Stack,
    /// References below the top.
    buried: u64,
    /// Coefficients settled against the zero below the oldest reference.
    /// With the top's coefficient they sum to one when the fold ends.
    resolved: i128,
}

impl HeightReferences {
    /// Begin with one reference at the first leaf's height.
    ///
    /// The caller adds the first leaf's height to the answer itself; every
    /// reference measures height above it.
    pub fn new() -> Self {
        Self {
            top: Top {
                chunk: Accumulator::new(),
                coefficient: 0,
                referenced: false,
            },
            second_coefficient: None,
            chunks: ChunkStack::new(),
            coefficients: PackedU64Stack::new(),
            buried: 0,
            resolved: 0,
        }
    }

    /// Add one leaf's offset to `total` and return it, freezing first if the
    /// offset would exceed the record allowance.
    ///
    /// `change` is the height change that reached this leaf; the first leaf
    /// passes zero.
    pub fn add_leaf(
        &mut self,
        live: &mut Accumulator,
        change: &BigInt,
        total: &mut Accumulator,
    ) -> LeafHeight {
        let mut offset = BigInt::from(&*live);
        if offset.bits() > change.bits() + RECORD_SLACK_BITS {
            #[cfg(test)]
            bump(&RECORD_FREEZES);
            self.freeze(live, total);
            offset = BigInt::ZERO;
        }
        *total += &offset;
        self.top.coefficient += 1;
        LeafHeight { offset }
    }

    /// Move the live height change into a new top reference, then clear it.
    ///
    /// A referenced top is buried beneath the new one. An unreferenced top
    /// instead absorbs the change, so every reference below the top stays
    /// referenced.
    pub fn freeze(&mut self, live: &mut Accumulator, total: &mut Accumulator) {
        let drift = core::mem::take(live);
        let previous = core::mem::replace(
            &mut self.top,
            Top {
                chunk: drift,
                coefficient: 0,
                referenced: false,
            },
        );
        if !previous.referenced {
            // The previous top is released at once: the new top absorbs it,
            // and the reference below it, if any, stays the second.
            self.merge_into_top(previous.chunk, previous.coefficient, total);
            return;
        }
        if let Some(coefficient) = self.second_coefficient.take() {
            push_i128(&mut self.coefficients, coefficient);
        }
        self.chunks.push(previous.chunk);
        self.second_coefficient = Some(previous.coefficient);
        self.buried += 1;
        #[cfg(test)]
        MAX_BURIED.with(|most| most.set(most.get().max(self.buried)));
    }

    /// Note a new minimum record at the top reference, returning whether it
    /// is the first live record there.
    pub fn reference_top(&mut self) -> bool {
        !core::mem::replace(&mut self.top.referenced, true)
    }

    /// Settle the top minimum record: subtract `closes` uses of its height.
    ///
    /// The record lies at the top reference if any record does, and
    /// otherwise at the second. `first` marks the last record at its
    /// reference to be settled, which releases that reference.
    pub fn settle_record(
        &mut self,
        offset: &BigInt,
        closes: u64,
        first: bool,
        total: &mut Accumulator,
    ) {
        if closes != 0 {
            fold_repeated(
                total,
                offset.magnitude(),
                u128::from(closes),
                offset.sign() != Sign::Minus,
            );
            let closes = i128::from(closes);
            if self.top.referenced {
                self.top.coefficient -= closes;
            } else {
                *self
                    .second_coefficient
                    .as_mut()
                    .expect("a record below the top lies at the second reference") -= closes;
            }
        }
        if first {
            if self.top.referenced {
                self.top.referenced = false;
            } else {
                self.remove_second(total);
            }
        }
    }

    /// Remove the unreferenced reference directly below the top.
    ///
    /// The top's chunk absorbs the removed chunk, and the removed
    /// coefficient moves to whichever neighbour lets the product read the
    /// narrower chunk. Both neighbours keep their heights.
    fn remove_second(&mut self, total: &mut Accumulator) {
        let coefficient = self
            .second_coefficient
            .take()
            .expect("a removed reference lies below the top");
        let removed = self.chunks.pop();
        self.buried -= 1;
        if self.buried > 0 {
            self.second_coefficient = Some(pop_i128(&mut self.coefficients));
        }
        self.merge_into_top(removed, coefficient, total);
    }

    /// Fold a released reference's chunk and coefficient into the top.
    ///
    /// The released reference lies directly below the top, so the top's
    /// chunk becomes the sum of both chunks and every live record keeps its
    /// height. The coefficient moves to the top or to the reference below,
    /// whichever lets its product read the narrower chunk; that chunk's limbs
    /// are read once and serve both the product and the sum, which lands in
    /// the wider chunk's storage.
    fn merge_into_top(
        &mut self,
        released: Accumulator,
        coefficient: i128,
        total: &mut Accumulator,
    ) {
        let top = core::mem::take(&mut self.top.chunk);
        let top_is_narrower = top.stored_bits() < released.stored_bits();
        let (narrower, mut wider) = if top_is_narrower {
            (top, released)
        } else {
            (released, top)
        };
        let (sign, magnitude) = narrower.signed_magnitude();
        let magnitude = magnitude.as_ref();
        if top_is_narrower {
            #[cfg(test)]
            bump(&MERGES_TO_NEWER);
            self.top.coefficient += coefficient;
            add_product(total, sign, magnitude, -coefficient);
        } else {
            #[cfg(test)]
            bump(&MERGES_TO_OLDER);
            add_product(total, sign, magnitude, coefficient);
            self.charge_older(coefficient);
        }
        add_signed_limbs(&mut wider, sign, magnitude);
        self.top.chunk = wider;
    }

    /// Move `coefficient` to the reference below the top, or settle it
    /// against the zero below the oldest reference.
    fn charge_older(&mut self, coefficient: i128) {
        match self.second_coefficient.as_mut() {
            Some(older) => *older += coefficient,
            None => self.resolved += coefficient,
        }
    }

    /// Add the remaining reference's contribution to `total`.
    ///
    /// Every record has been settled, so every buried reference has merged
    /// into the top, whose chunk is the whole height above the first leaf.
    pub fn settle(self, total: &mut Accumulator) {
        debug_assert_eq!(self.buried, 0, "settled records release every reference");
        debug_assert_eq!(
            self.top.coefficient + self.resolved,
            1,
            "a nonempty binary tree has one more leaf than internal nodes"
        );
        let (sign, magnitude) = self.top.chunk.signed_magnitude();
        add_product(total, sign, magnitude.as_ref(), self.top.coefficient);
    }
}

/// Add `count · factor` to `total`, or subtract it.
///
/// `count` is at most word-scale, so this is one multiplication linear in
/// `factor`.
fn fold_repeated(total: &mut Accumulator, factor: &BigUint, count: u128, subtract: bool) {
    if count == 0 || *factor == BigUint::ZERO {
        return;
    }
    let mut product = factor.clone();
    product *= BigUint::from(count);
    if subtract {
        *total -= &product;
    } else {
        *total += &product;
    }
}

/// Little-endian limbs multiplied by a word, produced lazily.
///
/// Feeding a product straight into an accumulator this way builds no
/// temporary as wide as the product.
struct Scaled<'a> {
    /// The remaining limbs of the multiplicand.
    limbs: core::slice::Iter<'a, u64>,
    /// The multiplier.
    factor: u64,
    /// The carry into the next limb.
    carry: u64,
}

impl<'a> Scaled<'a> {
    /// The limbs of `limbs · factor`.
    fn new(limbs: &'a [u64], factor: u64) -> Self {
        Self {
            limbs: limbs.iter(),
            factor,
            carry: 0,
        }
    }
}

impl Iterator for Scaled<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        match self.limbs.next() {
            Some(&limb) => {
                let wide = u128::from(limb) * u128::from(self.factor) + u128::from(self.carry);
                self.carry = (wide >> 64) as u64;
                Some(wide as u64)
            }
            None => Some(core::mem::take(&mut self.carry)).filter(|&carry| carry != 0),
        }
    }
}

/// Add `±magnitude` to `target`, the sign given as the value's comparison
/// with zero.
fn add_signed_limbs(target: &mut Accumulator, sign: Ordering, magnitude: &[u64]) {
    let limbs = magnitude.iter().copied();
    if sign == Ordering::Less {
        target.sub_shifted_limbs(0, limbs);
    } else {
        target.add_shifted_limbs(0, limbs);
    }
}

/// Add `coefficient · (±magnitude)` to `total`.
fn add_product(total: &mut Accumulator, sign: Ordering, magnitude: &[u64], coefficient: i128) {
    if coefficient == 0 || magnitude.is_empty() {
        return;
    }
    let sign = if coefficient < 0 {
        sign.reverse()
    } else {
        sign
    };
    let count = u64::try_from(coefficient.unsigned_abs())
        .expect("a coefficient counts leaves and closes of a version held in memory");
    let limbs = Scaled::new(magnitude, count);
    if sign == Ordering::Less {
        total.sub_shifted_limbs(0, limbs);
    } else {
        total.add_shifted_limbs(0, limbs);
    }
}

/// Push a signed 128-bit value as its zigzag code, low word below.
fn push_i128(stack: &mut PackedU64Stack, value: i128) {
    let code = ((value << 1) ^ (value >> 127)) as u128;
    stack.push(code as u64);
    stack.push((code >> 64) as u64);
}

/// Pop a value pushed by [`push_i128`].
fn pop_i128(stack: &mut PackedU64Stack) -> i128 {
    let high = u128::from(stack.pop());
    let code = (high << 64) | u128::from(stack.pop());
    ((code >> 1) as i128) ^ -((code & 1) as i128)
}

/// The widest working width [`ChunkStack::push`] reads to try the inline form.
const INLINE_READ_BITS: u64 = 64;

/// Chunks of buried references: inline words, or whole accumulators.
///
/// A chunk whose working width is at most 64 bits is read (a constant
/// cost) and stored inline when it fits an `i64`; any other chunk moves
/// whole, without being read, so a wide chunk that moves between the top and
/// the stack repeatedly costs nothing per move.
struct ChunkStack {
    /// Whether each chunk is held whole.
    wide: BitStack,
    /// Zigzag codes of inline chunks.
    narrow: PackedU64Stack,
    /// Chunks held whole.
    wide_values: Vec<Accumulator>,
}

impl ChunkStack {
    /// Construct an empty stack.
    fn new() -> Self {
        Self {
            wide: BitStack::new(),
            narrow: PackedU64Stack::new(),
            wide_values: Vec::new(),
        }
    }

    /// Push a chunk, reading it only if its working width is at most 64 bits.
    fn push(&mut self, chunk: Accumulator) {
        if chunk.stored_bits() <= INLINE_READ_BITS {
            if let Ok(value) = i64::try_from(&BigInt::from(&chunk)) {
                self.wide.push(false);
                self.narrow.push(((value << 1) ^ (value >> 63)) as u64);
                return;
            }
        }
        #[cfg(test)]
        bump(&WIDE_CHUNKS);
        self.wide.push(true);
        self.wide_values.push(chunk);
    }

    /// Pop the newest chunk.
    fn pop(&mut self) -> Accumulator {
        if self.wide.pop().expect("a buried reference has a chunk") {
            self.wide_values.pop().expect("each wide tag has a value")
        } else {
            let code = self.narrow.pop();
            Accumulator::from(((code >> 1) as i64) ^ -((code & 1) as i64))
        }
    }
}
