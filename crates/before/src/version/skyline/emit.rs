//! The join/meet emission sweep: pointwise max/min of two skyline
//! streams, re-delta-coded on the fly into a canonical output stream.
//!
//! Join is pointwise `max` and meet pointwise `min` over the unit id
//! interval, so both ride the comparison sweep's merge walk unchanged
//! (the [`sweep`](super::sweep) module doc carries the boundary
//! bookkeeping): two leaf cursors over the overlay partition, one
//! running signed difference `D = height_a − height_b` on the
//! cliff-immune [`Accum`]. What emission adds is an output leaf per
//! elementary interval — depth `max` of the two cursor depths, since
//! overlapping dyadic intervals nest — delivered to the collapsing
//! output builder (the crate-private `build` sibling module), which
//! derives the union topology from the depth sequence and truncates
//! equal sibling leaves back out. No height is
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
//! scan, decode, and fold bounds; each emitted code is written once and
//! truncated at most once (the builder's amortization, its module doc);
//! and the switch reads are each priced by the boundary's own input
//! codes. Transient state is the two cursor paths, the accumulator,
//! the builder's per-level bit stacks, and the output itself — no
//! working tree, no node array, no per-level value.
//!
//! # Testing
//!
//! The packed-form operators are the behavioral oracle: joining or
//! meeting through the transcoders must reproduce their output stream
//! byte for byte (canonical uniqueness makes that the whole contract),
//! over the adversarial families, arbitrary pairs, organic histories,
//! and the exhaustive small scope. A three-cursor overlay walk
//! additionally re-derives every output plateau's absolute height
//! against pointwise max/min of the inputs' — the direct witness that
//! no side switch was misread — and the algebraic laws (commutativity,
//! associativity, idempotence, absorption) are asserted on the emitted
//! streams themselves.

use core::cmp::Ordering;

use crate::codec::accum::Accum;
use crate::codec::{self, Base, Bits, BitsSlice};

use super::build::SkylineBuilder;
use super::sweep::{advance, LeafCursor, Side, Step};
use super::{zigzag_signed, Encoded};

/// The join (pointwise max) of the versions two skyline streams denote,
/// as a canonical skyline stream.
///
/// One merge over the two streams; the module doc carries the emission
/// algebra and the cost bounds. The output is byte-identical to
/// transcoding the packed-form join (the differential suite pins it).
///
/// # Panics
///
/// Panics if either operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes — or
/// declares more live bits than its bytes hold.
pub fn join(a: &Encoded, b: &Encoded) -> Encoded {
    emit(a, b, Op::Join)
}

/// The meet (pointwise min) of the versions two skyline streams denote,
/// as a canonical skyline stream.
///
/// [`join`]'s mirror: the same sweep with the side selection reversed.
///
/// # Panics
///
/// Panics on a non-canonical operand or an overrunning live-bit count,
/// exactly as [`join`] does.
pub fn meet(a: &Encoded, b: &Encoded) -> Encoded {
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
fn emit(a: &Encoded, b: &Encoded, op: Op) -> Encoded {
    let a_bits = live(a);
    let b_bits = live(b);
    let mut diff = Accum::new();
    let (mut ca, a_first) = LeafCursor::open(a_bits);
    let (mut cb, b_first) = LeafCursor::open(b_bits);
    diff.add_base(&a_first);
    diff.sub_base(&b_first);

    // The first interval: the winning side's absolute height opens the
    // output. Subadditivity caps the output at the inputs' total, so
    // their combined length is a one-allocation capacity.
    let mut side = op.pick(diff.sign(), Side::A);
    let mut out = SkylineBuilder::with_capacity(a_bits.len() + b_bits.len());
    let first = match side {
        Side::A => &a_first,
        Side::B => &b_first,
    };
    out.leaf(ca.depth().max(cb.depth()), gamma_code(first));

    while !(ca.done() && cb.done()) {
        let (da, db) = advance(&mut ca, &mut cb, &mut diff);
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

    let mut bits = out.finish();
    let live = bits.len();
    codec::zero_dead_bits(&mut bits);
    Encoded {
        bytes: bits.into_vec(),
        bits: live,
    }
}

/// Borrow one operand's live bits.
///
/// # Panics
///
/// Panics if the operand declares more live bits than its bytes hold.
fn live(enc: &Encoded) -> &BitsSlice {
    super::live_bits(&enc.bytes, enc.bits)
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
fn switch_delta(diff: &Accum, new_side: Side, old_delta: (bool, Base)) -> (bool, Base) {
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
/// zero, so the zigzag coding downstream stays canonical.
fn signed_sum(x_neg: bool, x: Base, y_neg: bool, y: &Base) -> (bool, Base) {
    if x_neg == y_neg {
        return (x_neg, &x + y);
    }
    match x.cmp(y) {
        Ordering::Greater => (x_neg, x - y),
        Ordering::Less => (y_neg, y.clone() - &x),
        Ordering::Equal => (false, Base::ZERO),
    }
}

/// A value's gamma code as a fresh payload-code buffer.
fn gamma_code(value: &Base) -> Bits {
    let mut code = Bits::new();
    codec::encode_int(&mut code, value);
    code
}

#[cfg(test)]
mod tests;
