//! The join/meet emission sweep: pointwise max/min of two skyline
//! streams, re-delta-coded on the fly into a canonical output stream.
//!
//! Join is pointwise `max` and meet pointwise `min` over the unit id
//! interval, so both ride the comparison sweep's merge walk unchanged
//! (the [`sweep`](super::sweep) module doc carries the boundary
//! bookkeeping): two leaf cursors over the overlay partition, one
//! running signed difference `D = height_a − height_b` on the
//! cliff-immune [`Accumulator`]. What emission adds is an output leaf per
//! elementary interval — depth `max` of the two cursor depths: overlapping
//! dyadic intervals nest, so the elementary interval *is* the deeper
//! side's leaf. The walk therefore emits a left-to-right dyadic tiling of
//! the unit interval, which is exactly the preorder leaf sequence the
//! collapsing output builder (the crate-private `build` sibling module)
//! demands; the builder derives the common refinement's topology from
//! the depth sequence and truncates equal sibling leaves back out. No height is
//! ever materialized along the way; the output is delta-coded from
//! quantities the boundary itself supplies.
//!
//! # The side-switch algebra
//!
//! Per elementary interval the output equals one input — the *side*,
//! `a` when `sign(D)` favors it, `b` when it favors the other, sticky
//! at ties (`D = 0` keeps the current side, which both inputs then
//! agree on). The output's delta across a boundary needs no absolute
//! heights:
//!
//! - **Same side**: the output moves with its side, so the delta is
//!   that side's own step delta — zero when the boundary belonged to
//!   the other stream alone.
//! - **Switch**: the output jumps from the old side's plateau to the
//!   new side's. With `D′` the difference *after* the boundary's folds
//!   and `δ` the old side's step delta at this boundary (zero if it
//!   did not step), the jump is `+D′ + δ` switching to `a`, `−D′ + δ`
//!   switching to `b` — both from one sign-and-magnitude read of the
//!   accumulator plus one signed sum. A switch means `D` crossed or
//!   left zero at this boundary, so `|D′|` is bounded by the deltas
//!   just folded, and the read is priced by the codes that carried
//!   them (the accumulator's sign fold has already collapsed any
//!   cancelling prefix by the time the side is picked).
//!
//! # Cost
//!
//! Derived, with the constants pinned by the `skyline_join_*` and
//! `skyline_meet_*` rows of the resource-envelope suite
//! (`tests/meter.rs`): the walk inherits the comparison sweep's linear
//! scan, decode, and fold bounds; the builder's collapse stays amortized
//! O(1) per output bit because every repair is subtractive — each
//! cascade step copies only a code its own deletions have already
//! priced (the builder's module doc carries the argument); and the
//! switch reads are each priced by the boundary's own input codes.
//! Transient state is the two cursor paths, the accumulator, the
//! builder's per-level bit stacks, and the output itself — no working
//! tree, no node array, no per-level machine word.
//!
//! # Testing
//!
//! The recursive oracle's join and meet are the behavioral witness: the
//! emitted stream must reproduce the oracle's encoded result bit for
//! bit (canonical uniqueness makes that the whole contract), over the
//! adversarial families, arbitrary pairs, organic histories, and the
//! exhaustive small scope (every ordered pair to the depth bound the
//! test-only `testing::exhaustive` module states and argues). A three-cursor overlay walk additionally
//! re-derives every output plateau's absolute height against pointwise
//! max/min of the inputs' — the direct witness that no side switch was
//! misread — and the algebraic laws (commutativity, associativity,
//! idempotence, absorption) are asserted on the emitted streams
//! themselves.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{Base, Bits, BitsSlice};

use super::build::SkylineBuilder;
use super::sweep::{advance_diff, LeafCursor, PlateauCursor, Side, Step};
use super::{gamma_code, zigzag_signed};

/// The join (pointwise max) of the versions two skyline streams denote,
/// as a canonical skyline stream.
///
/// One merge over the two streams; the module doc carries the emission
/// algebra and the cost bounds. The output stream equals the recursive
/// oracle's join bit for bit (the differential suite pins it); its dead
/// pad bits are left as built — zeroing them is the storage gate's job
/// (`Version::from_bits`).
///
/// # Panics
///
/// Operands must be canonical skyline streams — run
/// [`validate`](fn@super::validate) first on untrusted bytes. The
/// violations the walk structurally notices (truncation, malformation)
/// panic; the rest (a collapsible sibling pair, a delta driving the
/// running height negative) sweep silently, and the output is then
/// unspecified.
pub fn join(a: &BitsSlice, b: &BitsSlice) -> Bits {
    emit(a, b, Op::Join)
}

/// The meet (pointwise min) of the versions two skyline streams denote,
/// as a canonical skyline stream.
///
/// [`join`]'s mirror: the same sweep with the side selection reversed.
///
/// # Panics
///
/// [`join`]'s contract exactly: canonical operands required, structural
/// violations panic, the rest yield an unspecified output.
pub fn meet(a: &BitsSlice, b: &BitsSlice) -> Bits {
    emit(a, b, Op::Meet)
}

/// Which pointwise operation the sweep emits.
#[derive(Clone, Copy)]
enum Op {
    /// Pointwise max: the higher side wins.
    Join,
    /// Pointwise min: the lower side wins.
    Meet,
}

impl Op {
    /// The side the output follows on the next interval: the winner by
    /// the difference's sign, sticky at ties.
    fn pick(self, sign: Ordering, current: Side) -> Side {
        match (self, sign) {
            (Op::Join, Ordering::Greater) | (Op::Meet, Ordering::Less) => Side::A,
            (Op::Join, Ordering::Less) | (Op::Meet, Ordering::Greater) => Side::B,
            (_, Ordering::Equal) => current,
        }
    }
}

/// Run the emission sweep.
fn emit(a_bits: &BitsSlice, b_bits: &BitsSlice, op: Op) -> Bits {
    let mut diff = Accumulator::new();
    let (mut ca, a_first) = LeafCursor::open(a_bits);
    let (mut cb, b_first) = LeafCursor::open(b_bits);
    diff.add_magnitude(&a_first);
    diff.sub_magnitude(&b_first);

    // The first interval: the winning side's absolute height opens the
    // output. The inputs' combined length is the capacity *estimate*:
    // the union topology and the carried-over step codes fit under it,
    // but a switch code is bounded by the boundary's input codes only up
    // to a constant, so a pathological switch-heavy pair could outgrow
    // it — costing one reallocation, never correctness. The envelope
    // rows (`tests/meter.rs`, `skyline_join_*`/`skyline_meet_*`) pin the
    // measured peak heap, switch-heavy families included.
    let mut side = op.pick(diff.sign(), Side::A);
    let mut out = SkylineBuilder::with_capacity(a_bits.len() + b_bits.len());
    let first = match side {
        Side::A => &a_first,
        Side::B => &b_first,
    };
    out.leaf(ca.depth().max(cb.depth()), gamma_code(first));

    while !(ca.done() && cb.done()) {
        let (da, db) = advance_diff(&mut ca, &mut cb, &mut diff);
        let new_side = op.pick(diff.sign(), side);
        let (negative, magnitude) = if new_side == side {
            step_delta(side, &da, &db)
        } else {
            switch_delta(&diff, new_side, step_delta(side, &da, &db))
        };
        side = new_side;
        out.leaf(
            ca.depth().max(cb.depth()),
            gamma_code(&zigzag_signed(negative, magnitude)),
        );
    }

    // Canonicalizing the storage (zeroing dead pad bits) is the job of
    // `Version::from_bits`, the single gate a stream passes through when
    // it becomes a stored value; intermediate streams stay as built.
    out.finish()
}

/// One side's signed step delta at the boundary just crossed: zero when
/// that side's cursor did not step.
fn step_delta(side: Side, da: &Option<Step>, db: &Option<Step>) -> (bool, Base) {
    let step = match side {
        Side::A => da,
        Side::B => db,
    };
    match step {
        Some(step) => (step.negative, step.magnitude.clone()),
        None => (false, Base::ZERO),
    }
}

/// The output delta across a side switch: `±D′` oriented toward the new
/// side, plus the old side's step delta (the module doc's algebra).
fn switch_delta(diff: &Accumulator, new_side: Side, old_delta: (bool, Base)) -> (bool, Base) {
    let (sign, magnitude) = diff.sign_magnitude();
    debug_assert_ne!(sign, Ordering::Equal, "a tie never switches the side");
    let negative = match new_side {
        Side::A => sign == Ordering::Less,
        Side::B => sign == Ordering::Greater,
    };
    signed_sum(negative, Base::from(magnitude), old_delta.0, &old_delta.1)
}

/// The sign and magnitude of a sum of two signed magnitudes.
///
/// Never yields a negative zero: a cancelling pair returns the positive
/// zero, so the zigzag coding downstream stays canonical. Shared with the
/// projection sweep, whose leaving-the-owned-region delta is the same
/// signed combination.
pub(super) fn signed_sum(x_neg: bool, x: Base, y_neg: bool, y: &Base) -> (bool, Base) {
    if x_neg == y_neg {
        return (x_neg, &x + y);
    }
    match x.cmp(y) {
        Ordering::Greater => (x_neg, x - y),
        Ordering::Less => (y_neg, y.clone() - &x),
        Ordering::Equal => (false, Base::ZERO),
    }
}

#[cfg(test)]
mod tests;
