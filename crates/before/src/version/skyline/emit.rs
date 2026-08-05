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
//! agree on). Join and meet are this one sweep with the side selection
//! reversed — pointwise max follows the higher side, pointwise min the
//! lower — and the selection is everything that distinguishes them:
//! each entry point passes its own picking closure and the sweep never
//! consults which operation it is running. Because the crossing
//! sequence and the running difference are selection-independent,
//! [`hull`] emits both outputs from one sweep, each operand decoded
//! once. The output's delta across a
//! boundary needs no absolute heights:
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

use crate::codec::{Base, BitsMut, BitsSlice, Code, Int};

use super::build::SkylineBuilder;
use super::sweep::{advance_diff, Directions, OpenedPair, PlateauCursor, Side, Step};
use super::{gamma_code_int, gamma_code_signed_int};

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
pub fn join(a: &BitsSlice, b: &BitsSlice) -> BitsMut {
    // Pointwise max: the higher side wins the interval, sticky at ties.
    emit(a, b, |sign, current| match sign {
        Ordering::Greater => Side::A,
        Ordering::Less => Side::B,
        Ordering::Equal => current,
    })
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
pub fn meet(a: &BitsSlice, b: &BitsSlice) -> BitsMut {
    // Pointwise min: the lower side wins the interval, sticky at ties.
    emit(a, b, |sign, current| match sign {
        Ordering::Less => Side::A,
        Ordering::Greater => Side::B,
        Ordering::Equal => current,
    })
}

/// The fused hull's product: the two endpoint streams beside the
/// pair's causal relation, all read off one sweep.
pub struct Hull {
    /// The causal order of the versions the operands denote — the
    /// comparison sweep's verdict, folded from the per-interval signs
    /// the emission walk reads anyway; `None` is concurrent.
    ///
    /// Byte-distinct equal operands report `Equal` here like any other
    /// pair: the fold sees only signs, never buffers.
    pub relation: Option<Ordering>,
    /// The meet (pointwise min) stream.
    pub lo: BitsMut,
    /// The join (pointwise max) stream.
    pub hi: BitsMut,
}

/// The hull `(meet, join)` of the versions two skyline streams denote,
/// as canonical skyline streams beside the pair's causal relation, all
/// from **one** fused sweep.
///
/// [`meet`] and [`join`] differ only in their side selection: both
/// consume the identical crossing sequence — the boundaries the pair
/// sweep's `advance_diff` yields and the one running difference `D` — and
/// pick a side per elementary interval. Emitting both from one sweep
/// therefore decodes each operand once and folds each crossing into
/// the accumulator once, where composing the two entry points does
/// both twice; only the per-interval side selections and the two
/// output builders are doubled. Each output is byte-identical to its
/// single-op entry point: the differential proptest beside this module
/// pins the identity stream-level, and the `span_is_the_pair_hull` law
/// pins it through the public door on every law consumer.
///
/// The relation rides for free: the per-interval sign the side picks
/// read is exactly what the comparison sweep folds, so one
/// surviving-directions fold beside the emissions decides the pair's
/// causal order at exhaustion (both directions surviving is equality,
/// one alone is domination, neither is concurrent). The differential
/// suite beside this module pins the verdict against the lattice
/// reading of the oracle's outputs on every witnessed pair. A caller
/// that already classified the pair (the span ladder reaches this walk
/// only on concurrent operands) reads it as a free cross-check.
///
/// # Panics
///
/// [`join`]'s contract exactly: canonical operands required, structural
/// violations panic, the rest yield an unspecified output triple.
pub fn hull(a_bits: &BitsSlice, b_bits: &BitsSlice) -> Hull {
    /// One output of the fused sweep: its side selection (pointwise min
    /// or max — the only point where the two outputs differ), the side
    /// it is currently following, and its builder.
    struct Emission {
        pick: fn(Ordering, Side) -> Side,
        side: Side,
        out: SkylineBuilder,
    }

    let OpenedPair {
        a: mut ca,
        b: mut cb,
        mut diff,
        a_first,
        b_first,
    } = OpenedPair::open(a_bits, b_bits);

    // One sign read serves the whole interval: both side picks and the
    // relation fold consume the same `sign(D)`, so the accumulator is
    // read once per elementary interval however many consumers share it.
    let mut dirs = Directions::new();
    let sign = diff.sign();
    dirs.fold(sign);

    // The first interval opens each output with its winning side's
    // absolute height; the capacity estimate is `emit`'s, per builder.
    let mut outputs = [
        // Pointwise min: the lower side wins, sticky at ties.
        Emission {
            pick: |sign, current| match sign {
                Ordering::Less => Side::A,
                Ordering::Greater => Side::B,
                Ordering::Equal => current,
            },
            side: Side::A,
            out: SkylineBuilder::with_capacity(a_bits.len() + b_bits.len()),
        },
        // Pointwise max: the higher side wins, sticky at ties.
        Emission {
            pick: |sign, current| match sign {
                Ordering::Greater => Side::A,
                Ordering::Less => Side::B,
                Ordering::Equal => current,
            },
            side: Side::A,
            out: SkylineBuilder::with_capacity(a_bits.len() + b_bits.len()),
        },
    ];
    for emission in &mut outputs {
        emission.side = (emission.pick)(sign, Side::A);
        let first = match emission.side {
            Side::A => &a_first,
            Side::B => &b_first,
        };
        emission
            .out
            .leaf(ca.depth().max(cb.depth()), gamma_code_int(first));
    }

    while !(ca.done() && cb.done()) {
        // One crossing, folded once, serves both outputs' deltas.
        let (da, db) = advance_diff(&mut ca, &mut cb, &mut diff);
        let sign = diff.sign();
        dirs.fold(sign);
        let depth = ca.depth().max(cb.depth());
        for emission in &mut outputs {
            let new_side = (emission.pick)(sign, emission.side);
            let code = delta_code(&diff, emission.side, new_side, &da, &db);
            emission.side = new_side;
            emission.out.leaf(depth, code);
        }
    }

    let [lo, hi] = outputs;
    Hull {
        relation: dirs.relation(),
        lo: lo.out.finish(),
        hi: hi.out.finish(),
    }
}

/// Run the emission sweep, generic over the side selection.
///
/// `pick` selects the side the output follows on each interval, from
/// the difference's sign and the current side — the winner by sign,
/// sticky at ties, and the only point where join and meet differ (see
/// the module doc's side-switch algebra).
fn emit(a_bits: &BitsSlice, b_bits: &BitsSlice, pick: impl Fn(Ordering, Side) -> Side) -> BitsMut {
    let OpenedPair {
        a: mut ca,
        b: mut cb,
        mut diff,
        a_first,
        b_first,
    } = OpenedPair::open(a_bits, b_bits);

    // The first interval: the winning side's absolute height opens the
    // output. The inputs' combined length is the capacity *estimate*:
    // the union topology and the carried-over step codes fit under it,
    // but a switch code is bounded by the boundary's input codes only up
    // to a constant, so a pathological switch-heavy pair could outgrow
    // it — costing one reallocation, never correctness. The envelope
    // rows (`tests/meter.rs`, `skyline_join_*`/`skyline_meet_*`) pin the
    // measured peak heap, switch-heavy families included.
    let mut side = pick(diff.sign(), Side::A);
    let mut out = SkylineBuilder::with_capacity(a_bits.len() + b_bits.len());
    let first = match side {
        Side::A => &a_first,
        Side::B => &b_first,
    };
    out.leaf(ca.depth().max(cb.depth()), gamma_code_int(first));

    while !(ca.done() && cb.done()) {
        let (da, db) = advance_diff(&mut ca, &mut cb, &mut diff);
        let new_side = pick(diff.sign(), side);
        let code = delta_code(&diff, side, new_side, &da, &db);
        side = new_side;
        out.leaf(ca.depth().max(cb.depth()), code);
    }

    // Canonicalizing the storage (zeroing dead pad bits) is the job of
    // `Version::from_bits`, the single gate a stream passes through when
    // it becomes a stored value; intermediate streams stay as built.
    out.finish()
}

/// The output's delta code at the boundary just crossed: the followed
/// side's own step on the same-side path, the switch algebra otherwise
/// (the module doc).
///
/// The same-side path borrows the step and codes it in place; only a
/// switch materializes a magnitude.
fn delta_code(
    diff: &Accumulator,
    side: Side,
    new_side: Side,
    da: &Option<Step>,
    db: &Option<Step>,
) -> Code {
    let step = match side {
        Side::A => da.as_ref(),
        Side::B => db.as_ref(),
    };
    if new_side == side {
        return match step {
            Some(step) => gamma_code_signed_int(step.negative, &step.magnitude),
            None => gamma_code_signed_int(false, &Int::ZERO),
        };
    }
    let (negative, magnitude) = switch_delta(diff, new_side, step);
    gamma_code_signed_int(negative, &magnitude)
}

/// The output delta across a side switch: `±D′` oriented toward the new
/// side, plus the old side's step delta (the module doc's algebra).
fn switch_delta(diff: &Accumulator, new_side: Side, old_step: Option<&Step>) -> (bool, Int) {
    let (sign, magnitude) = diff.sign_magnitude();
    debug_assert_ne!(sign, Ordering::Equal, "a tie never switches the side");
    let negative = match new_side {
        Side::A => sign == Ordering::Less,
        Side::B => sign == Ordering::Greater,
    };
    match old_step {
        Some(step) => signed_sum_int(
            negative,
            Int::from_ubig(magnitude),
            step.negative,
            &step.magnitude,
        ),
        None => (negative, Int::from_ubig(magnitude)),
    }
}

/// The sign and magnitude of a sum of two signed [`Int`] magnitudes:
/// [`signed_sum`]'s value form, word-scale pairs summed in machine
/// arithmetic.
///
/// Never yields a negative zero, as [`signed_sum`].
pub(super) fn signed_sum_int(x_neg: bool, x: Int, y_neg: bool, y: &Int) -> (bool, Int) {
    if let (Int::Small(a), Int::Small(b)) = (&x, y) {
        let a = if x_neg {
            -i128::from(*a)
        } else {
            i128::from(*a)
        };
        let b = if y_neg {
            -i128::from(*b)
        } else {
            i128::from(*b)
        };
        let sum = a + b;
        let magnitude = match u64::try_from(sum.unsigned_abs()) {
            Ok(word) => Int::Small(word),
            Err(_) => Int::Wide(Base::from(sum.unsigned_abs())),
        };
        return (sum < 0, magnitude);
    }
    let y_widened;
    let y = match y {
        Int::Wide(base) => base,
        Int::Small(n) => {
            y_widened = Base::from(*n);
            &y_widened
        }
    };
    let (negative, magnitude) = signed_sum(x_neg, x.into_base(), y_neg, y);
    (negative, Int::from_base(magnitude))
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
