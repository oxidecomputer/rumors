//! Frozen references for the running leaf height.
//!
//! [`HeightReferences`] keeps recent height changes in one live accumulator
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
//!   many bits wider than its own leaf's encoded change. When `L`'s working
//!   width alone exceeds the change by more than [`UNREAD_FREEZE_BITS`], its
//!   value is certainly too wide, and it freezes without being read.
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
//! fold ends. The first leaf's height `h_0` is a term of every leaf height and
//! every minimum, so it contributes exactly `h_0` to the answer. The fold adds
//! it once at the start, and the references measure only the signed height
//! relative to it.
//!
//! # Why merges cost linear time
//!
//! A merge adds the narrower chunk into the wider one's storage and moves the
//! coefficient through whichever chunk is narrower, so it reads only the
//! narrower operand. The sum is at most one digit wider than the wider
//! operand, so the total stored width of live chunks falls by at least the
//! width read, less one digit. Freezes add only drifts the input paid for.
//! Total merge work is therefore linear in the input. Burying the top
//! reference and exposing it again cost a constant: a chunk is copied only
//! when it is at most [`INLINE_CHUNK_BITS`] wide, and wider chunks move
//! whole without being read.

use core::cmp::Ordering;

use num_bigint::BigInt;
use suanpan::{Accumulator, Limbs};

use super::integers::{bit_length, IntegerStack};
use super::records::Popped;
use crate::bits::stack::{BitStack, PackedU64Stack};

/// Bits a stored offset may exceed its leaf's height change by before the
/// live change freezes into a new reference.
///
/// The allowance trades heap against freezing work. A wider allowance stores
/// wider offsets: on the jump-entered rising spine, whose levels each cost 6
/// input bits, `min_ticks` holds about 4.7 bytes per input byte at 4 bits,
/// 7.4 at 8, and 9.0 at 16, and at 32 bits the figure keeps growing with
/// depth, because offsets widen with the logarithm of the level count until
/// they reach the allowance. A narrower allowance freezes more often: at 2
/// bits and 1 bit the board's scan-hole families read about 15.2 and 16.0
/// suanpan digit touches per input byte, against 14.6 at 4 bits and 14.4 at
/// 8 and above, while no heap reading falls more than 0.2 bytes per input
/// byte below its value at 4 bits. These figures were measured on the
/// amplification board's families at the top of its acceptance ladder.
pub const RECORD_SLACK_BITS: u64 = 4;

/// The widest working width, in bits, at which a buried chunk is copied into
/// packed storage instead of moving whole.
///
/// Copying a chunk costs at most this many bits per move, a constant. A
/// chunk that moves whole costs nothing per move but keeps its accumulator:
/// a fixed-size structure, plus digits that take twice its working width. The
/// bound confines that form to chunks this wide, whose height changes the
/// input paid for in a stretch no other live reference shares.
///
/// The bound compares working width, while the integer stack's
/// [`PACKED_BITS`](super::integers::PACKED_BITS) compares the value's width.
/// A copied chunk's value is below `2^(INLINE_CHUNK_BITS + 2)`, so it can be
/// up to two bits wider than that bound and take its own allocation in the
/// stack; a copy still costs at most a constant.
pub const INLINE_CHUNK_BITS: u64 = 256;

/// Bits a live change's working width may exceed the leaf's height change by
/// before the change freezes without being read.
///
/// A read leaves the live change's working width within one 32-bit digit of
/// its value's width, and the fold reads it at every leaf it does not freeze.
/// Adding a change of `k` bits widens it by at most one digit past the wider
/// operand. A working width more than this many bits above `k` therefore
/// means a value more than `k + 192` bits wide, which the record trigger
/// would freeze anyway; freezing first skips reading a value the freeze
/// moves whole.
pub const UNREAD_FREEZE_BITS: u64 = 256;

#[cfg(test)]
thread_local! {
    /// Freezes of either kind on this test thread.
    pub static FREEZE_HITS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// Freezes that skipped reading the live change on this test thread.
    pub static UNREAD_FREEZES: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// The most working width, beyond its leaf's change, that any read of
    /// the live change paid for on this test thread.
    pub static MAX_READ_EXCESS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// Record-trigger freezes on this test thread.
    pub static RECORD_FREEZES: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// Reference removals that moved the coefficient to the newer neighbour.
    pub static MERGES_TO_NEWER: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// Reference removals that moved the coefficient to the older neighbour.
    pub static MERGES_TO_OLDER: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// References buried with a chunk too wide to copy.
    pub static WIDE_CHUNKS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// Negative coefficients pushed onto the coefficient stack.
    pub static NEGATIVE_STACKED: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// Negative coefficients popped off the coefficient stack.
    pub static NEGATIVE_UNSTACKED: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
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
    /// The offset's comparison with zero.
    sign: Ordering,
    /// The offset's magnitude, or `None` when the leaf froze the live change
    /// and so sits exactly at the new reference.
    magnitude: Option<Limbs>,
}

impl LeafHeight {
    /// The offset's comparison with zero.
    pub fn sign(&self) -> Ordering {
        self.sign
    }

    /// The offset's magnitude as little-endian limbs with no high zero limb.
    pub fn limbs(&self) -> &[u64] {
        self.magnitude.as_deref().unwrap_or(&[])
    }

    /// A leaf at offset zero from the reference it just froze.
    fn at_new_reference() -> Self {
        Self {
            sign: Ordering::Equal,
            magnitude: None,
        }
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

/// The references of the running height, their coefficients, and the
/// recent change above the newest.
pub struct HeightReferences {
    /// `L`: the height changes since the newest reference froze.
    live: Accumulator,
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
            live: Accumulator::new(),
            top: Top {
                chunk: Accumulator::new(),
                coefficient: 0,
                referenced: false,
            },
            second_coefficient: None,
            chunks: ChunkStack::new(),
            coefficients: PackedU64Stack::new(),
            resolved: 0,
        }
    }

    /// Advance the running height by one height change, folding it into the
    /// live change `L`.
    pub fn advance(&mut self, change: &BigInt) {
        self.live += change;
    }

    /// Add one leaf's offset to `total` and return it, freezing first if the
    /// offset would exceed the record allowance.
    ///
    /// A live change whose working width exceeds `change`'s bits by more than
    /// [`UNREAD_FREEZE_BITS`] freezes without being read.
    ///
    /// `change` is the height change that reached this leaf, already folded
    /// in by [`advance`](Self::advance); the first leaf passes zero.
    pub fn add_leaf(&mut self, change: &BigInt, total: &mut Accumulator) -> LeafHeight {
        let change_bits = change.bits();
        let leaf = if self.live.stored_bits() > change_bits + UNREAD_FREEZE_BITS {
            #[cfg(test)]
            bump(&UNREAD_FREEZES);
            self.freeze(total);
            LeafHeight::at_new_reference()
        } else {
            #[cfg(test)]
            MAX_READ_EXCESS.with(|most| {
                most.set(
                    most.get()
                        .max(self.live.stored_bits().saturating_sub(change_bits)),
                )
            });
            let (sign, magnitude) = self.live.signed_magnitude();
            if bit_length(&magnitude) > change_bits + RECORD_SLACK_BITS {
                #[cfg(test)]
                bump(&RECORD_FREEZES);
                self.freeze(total);
                LeafHeight::at_new_reference()
            } else {
                add_signed_limbs(total, sign, &magnitude);
                LeafHeight {
                    sign,
                    magnitude: Some(magnitude),
                }
            }
        };
        self.top.coefficient += 1;
        leaf
    }

    /// Move the live height change into a new top reference, then clear it.
    ///
    /// A referenced top is buried beneath the new one. An unreferenced top
    /// instead absorbs the change, so every reference below the top stays
    /// referenced.
    fn freeze(&mut self, total: &mut Accumulator) {
        #[cfg(test)]
        bump(&FREEZE_HITS);
        let drift = core::mem::take(&mut self.live);
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
        #[cfg(test)]
        MAX_BURIED.with(|most| most.set(most.get().max(self.chunks.len())));
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
    pub fn settle_record(&mut self, record: Popped, total: &mut Accumulator) {
        let Popped {
            sign,
            magnitude,
            closes,
            first,
        } = record;
        if closes != 0 {
            let closes = i128::from(closes);
            add_product(total, sign, magnitude.limbs(), -closes);
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
        if self.chunks.len() > 0 {
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
        debug_assert_eq!(
            self.chunks.len(),
            0,
            "settled records release every reference"
        );
        debug_assert_eq!(
            self.top.coefficient + self.resolved,
            1,
            "a nonempty binary tree has one more leaf than internal nodes"
        );
        let (sign, magnitude) = self.top.chunk.signed_magnitude();
        add_product(total, sign, magnitude.as_ref(), self.top.coefficient);
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
    #[cfg(test)]
    if value < 0 {
        bump(&NEGATIVE_STACKED);
    }
    let code = ((value << 1) ^ (value >> 127)) as u128;
    stack.push(code as u64);
    stack.push((code >> 64) as u64);
}

/// Pop a value pushed by [`push_i128`].
fn pop_i128(stack: &mut PackedU64Stack) -> i128 {
    let high = u128::from(stack.pop());
    let code = (high << 64) | u128::from(stack.pop());
    let value = ((code >> 1) as i128) ^ -((code & 1) as i128);
    #[cfg(test)]
    if value < 0 {
        bump(&NEGATIVE_UNSTACKED);
    }
    value
}

/// Chunks of buried references: packed copies, or whole accumulators.
struct ChunkStack {
    /// Whether each chunk moved whole.
    whole: BitStack,
    /// Chunks at most [`INLINE_CHUNK_BITS`] wide, at their exact width.
    packed: IntegerStack,
    /// Chunks that moved whole.
    whole_values: Vec<Accumulator>,
}

impl ChunkStack {
    /// Construct an empty stack.
    fn new() -> Self {
        Self {
            whole: BitStack::new(),
            packed: IntegerStack::new(),
            whole_values: Vec::new(),
        }
    }

    /// The number of buried chunks.
    fn len(&self) -> u64 {
        self.whole.len()
    }

    /// Push a chunk, copying it only if its working width is at most
    /// [`INLINE_CHUNK_BITS`].
    fn push(&mut self, chunk: Accumulator) {
        if chunk.stored_bits() <= INLINE_CHUNK_BITS {
            let (sign, magnitude) = chunk.signed_magnitude();
            self.whole.push(false);
            self.packed.push(sign, magnitude.as_ref().iter().copied());
            return;
        }
        #[cfg(test)]
        bump(&WIDE_CHUNKS);
        self.whole.push(true);
        self.whole_values.push(chunk);
    }

    /// Pop the newest chunk.
    fn pop(&mut self) -> Accumulator {
        if self.whole.pop().expect("a buried reference has a chunk") {
            return self.whole_values.pop().expect("each whole tag has a value");
        }
        let (sign, magnitude) = self.packed.pop();
        let mut chunk = Accumulator::new();
        add_signed_limbs(&mut chunk, sign, magnitude.limbs());
        chunk
    }
}
