//! The range placement co-walk: one probe stream against a range's bound
//! streams, in a single fused merge.
//!
//! `causally`'s placement questions compare one version against up to two
//! bound versions. Composed from the pair sweep ([`sweep`](super::sweep)),
//! that costs two walks — the probe stream decoded once per bound. This
//! module fuses them: one overlay walk over the probe and both bound
//! streams, each decoded once, maintaining one running difference per
//! bound. The pair-difference algebra is [`sweep`](super::sweep)'s
//! ([`OpenedPair`](super::sweep::OpenedPair) seeds a difference the same
//! way; [`fold`] orients every crossing); only the
//! arity and the verdict vocabulary are this walk's.
//!
//! # The walk
//!
//! One [`LeafCursor`] per present stream. All
//! current leaves contain the sweep point, so they nest by depth, and the
//! comparison sweep's boundary bookkeeping generalizes verbatim (the
//! [`masked`](super::masked) walk's precedent): the deepest cursor
//! advances, and every other cursor whose depth reaches the advanced
//! cursor's flip level advances in the same step, tied boundaries closing
//! to one shared flip level (debug-asserted at every tie). The generic
//! [`advance`](super::sweep::advance) law is binary, so — exactly as the
//! masked walk does at arity four — the loop here restates the law at
//! this arity rather than instantiating it; the advance rule and its tie
//! assert are the law's, and only the slot dispatch is this walk's.
//!
//! Per bound, the walk maintains the running difference
//! `D = height_probe − height_bound` on the cliff-immune
//! [`Accumulator`] and folds its sign once per elementary interval into
//! the bound's two surviving-direction flags — the same
//! `(probe <= bound, bound <= probe)` pair the comparison sweep folds. A
//! probe crossing folds into both differences; a bound crossing folds
//! into its own. Each accumulator therefore sees exactly the write
//! sequence the corresponding pair sweep would commit, which is what
//! keeps the one-bound degenerate walk's meter readings identical to
//! [`causal_cmp`](super::sweep::causal_cmp)'s (the resource pins in
//! `tests/meter.rs`'s placement rows hold the identity).
//!
//! # Early exit
//!
//! Two relations resolve before exhaustion, and the walk acts on both:
//!
//! - **Concurrent to the end bound** (both end directions refuted): the
//!   verdict is [`Placement::ConcurrentToEnd`] outright — on a validated
//!   range (`start <= end`) a probe concurrent to the end cannot be below
//!   the start — and the walk returns at the deciding interval, exactly
//!   where the pair sweep's order mode would.
//! - **Concurrent to the start bound** (both start directions refuted):
//!   the start relation is pinned, but the end relation is still open, so
//!   the walk *drops the start cursor* — its stream is never scanned
//!   further, no coarser than the two-walk composition's early exit — and
//!   sweeps on over the probe and end streams alone. With no end bound
//!   the verdict is [`Placement::Inside`] immediately.
//!
//! Every other relation ((in)equality against a bound, domination either
//! way) is refutable early but confirmable only at exhaustion, again
//! exactly as in the pair sweep.
//!
//! # Cost
//!
//! Derived, mirroring [`sweep`](super::sweep)'s argument stream by
//! stream: every topology bit of every present stream is read at most
//! once, every path bit pushed and popped at most once, and every leaf
//! payload decoded once and folded into at most two accumulators (a
//! constant factor over the pair sweep, paid only by the probe's own
//! deltas). Scan, decode, and stack work are linear in the present
//! streams' bits — `O(|v| + |s| + |e|)` against the two-walk
//! composition's `O(2|v| + |s| + |e|)` — and the per-interval sign reads
//! ride the accumulator's amortized-O(1) collapse ([`suanpan`]'s
//! argument, unchanged). The relational pins in `tests/meter.rs` hold the
//! fused walk to the composition minus one probe scan, and the one-bound
//! degenerate form to the pair sweep byte for byte.
//!
//! # Testing
//!
//! The two-walk composition is the oracle: the
//! `bounded_matches_bound_relations` law in [`crate::laws`] pins the
//! fused verdict to the two `partial_cmp` verdicts on every law consumer
//! (generated, organic, exhaustive, and fuzzed populations), and
//! `causally`'s witness-matrix tests pin every verdict, bound-kind
//! combination, and the coincident-bounds corner against constructed
//! inputs. The resource identities are the meter rows named above.

// The module doc names crate-private machinery by intra-doc link so a
// rename cannot rot the prose (the internal doc build resolves every
// link); on the public build those links render as plain code spans —
// the items are private — which this allow accepts.
#![allow(rustdoc::private_intra_doc_links)]

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{Base, BitsSlice};

use super::sweep::{fold, LeafCursor, PlateauCursor, Side, Step};

/// Where a probe stream's version sits relative to a validated pair of
/// bound streams: the walk's verdict, at full resolution.
///
/// Stream-level vocabulary for `causally`'s six placement verdicts, in
/// the same order; `causally` states the range semantics, this module
/// only computes the relations. `AtStart` is the coincident-bounds
/// canonicalization's home: the start relation is examined first, so a
/// probe equal to coinciding bounds reports `AtStart`, never `AtEnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// Strictly below the start bound: `v < s`.
    BelowStart,
    /// Equal to the start bound: `v == s` (also covers `v == s == e`).
    AtStart,
    /// Past or concurrent to any start bound, and strictly below any end
    /// bound: the contained region.
    Inside,
    /// Equal to the end bound: `v == e` (and not equal to the start).
    AtEnd,
    /// Strictly above the end bound: `e < v`.
    AboveEnd,
    /// Concurrent to the end bound: neither contains the other.
    ConcurrentToEnd,
}

/// One bound's side of the walk: its cursor, its running difference
/// `D = height_probe − height_bound`, and the two surviving directions.
struct BoundSide<'a> {
    cursor: LeafCursor<'a>,
    /// `height_probe − height_bound`, on the cliff-immune accumulator.
    diff: Accumulator,
    /// `probe <= bound` still possible (no interval put the probe above).
    le: bool,
    /// `bound <= probe` still possible (no interval put the bound above).
    ge: bool,
}

impl<'a> BoundSide<'a> {
    /// Open one bound stream at its first leaf and seed its difference
    /// from the probe's absolute first height.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    fn open(bits: &'a BitsSlice, probe_first: &Base) -> BoundSide<'a> {
        let (cursor, first) = LeafCursor::open(bits);
        let mut diff = Accumulator::new();
        diff.add_magnitude(probe_first);
        diff.sub_magnitude(&first);
        BoundSide {
            cursor,
            diff,
            le: true,
            ge: true,
        }
    }

    /// Fold this interval's sign into the surviving directions; `true`
    /// when both directions are refuted (the probe and this bound are
    /// concurrent — the relation is decided).
    fn read(&mut self) -> bool {
        match self.diff.sign() {
            Ordering::Greater => self.le = false,
            Ordering::Less => self.ge = false,
            Ordering::Equal => {}
        }
        !self.le && !self.ge
    }

    /// The relation the completed sweep decided, as the causal order.
    fn relation(&self) -> Option<Ordering> {
        match (self.le, self.ge) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

/// Place a probe stream against a validated pair of bound streams, each
/// stream decoded once.
///
/// `start` and `end`, when both present, must satisfy `start <= end`
/// (`causally`'s composition gate); the verdict is unspecified otherwise.
/// With neither bound the verdict is [`Placement::Inside`] at zero cost.
///
/// # Panics
///
/// Operands must be canonical skyline streams —
/// [`causal_cmp`](super::sweep::causal_cmp)'s contract exactly: the
/// violations the walk structurally notices panic, the rest sweep
/// silently with an unspecified verdict.
pub(crate) fn place(
    probe: &BitsSlice,
    start: Option<&BitsSlice>,
    end: Option<&BitsSlice>,
) -> Placement {
    if start.is_none() && end.is_none() {
        return Placement::Inside;
    }
    let (mut probe, probe_first) = LeafCursor::open(probe);
    let mut start = start.map(|bits| BoundSide::open(bits, &probe_first));
    let mut end = end.map(|bits| BoundSide::open(bits, &probe_first));

    loop {
        // One read per bound per elementary interval, end first: a probe
        // concurrent to the end is the whole verdict (on a validated
        // range it cannot also be below the start), while a probe
        // concurrent to the start only pins that side — drop its cursor,
        // never scanning the start stream further, and sweep on for the
        // end relation.
        if let Some(side) = &mut end {
            if side.read() {
                return Placement::ConcurrentToEnd;
            }
        }
        if let Some(side) = &mut start {
            if side.read() {
                start = None;
                if end.is_none() {
                    return Placement::Inside;
                }
            }
        }
        let exhausted = probe.done()
            && start.as_ref().is_none_or(|side| side.cursor.done())
            && end.as_ref().is_none_or(|side| side.cursor.done());
        if exhausted {
            break;
        }
        advance(&mut probe, &mut start, &mut end);
    }

    // The start relation speaks first (the coincident-bounds
    // canonicalization); a dropped start cursor is the concurrent case,
    // which falls through to the end relation like `Greater` does.
    match start.as_ref().and_then(BoundSide::relation) {
        Some(Ordering::Less) => return Placement::BelowStart,
        Some(Ordering::Equal) => return Placement::AtStart,
        Some(Ordering::Greater) | None => {}
    }
    match end.as_ref().map(BoundSide::relation) {
        None => Placement::Inside,
        Some(Some(Ordering::Less)) => Placement::Inside,
        Some(Some(Ordering::Equal)) => Placement::AtEnd,
        Some(Some(Ordering::Greater)) => Placement::AboveEnd,
        // Unreachable on canonical inputs: a decided concurrency returned
        // from the loop. Kept total so a non-canonical sweep stays a
        // silent unspecified verdict, per the panics contract.
        Some(None) => Placement::ConcurrentToEnd,
    }
}

/// Advance the overlay one boundary: the deepest cursor steps, and every
/// other cursor whose depth reaches the flip level steps in the same
/// round.
///
/// The overlay-advance law ([`super::sweep::advance`]) restated at this
/// arity, the masked walk's idiom.
///
/// Slot order puts the probe last so it wins depth ties and steps first
/// (the binary law's equal-depth arm steps its first operand first, and
/// the probe is every pair's first operand), keeping each accumulator's
/// write sequence — and with it the committed touch-meter readings of the
/// one-bound degenerate walk — identical to the pair sweep's.
fn advance<'a>(
    probe: &mut LeafCursor<'a>,
    start: &mut Option<BoundSide<'a>>,
    end: &mut Option<BoundSide<'a>>,
) {
    /// An absent (or dropped) side never steps: depth zero, like the
    /// masked walk's absent mask.
    fn depth(side: &Option<BoundSide<'_>>) -> usize {
        side.as_ref().map_or(0, |side| side.cursor.depth())
    }

    // Fold one probe crossing into every live difference, positively:
    // the probe is the `A` side of both pairs.
    fn fold_probe<'a>(
        step: &Step,
        start: &mut Option<BoundSide<'a>>,
        end: &mut Option<BoundSide<'a>>,
    ) {
        for side in [start, end].into_iter().flatten() {
            fold(&mut side.diff, Side::A, step.negative, &step.magnitude);
        }
    }

    // Step one bound side, folding its crossing into its own difference
    // as the `B` operand; returns the flip level for the tie test.
    fn step_bound(side: &mut BoundSide<'_>) -> usize {
        let (flip, step) = side.cursor.step();
        fold(&mut side.diff, Side::B, step.negative, &step.magnitude);
        flip
    }

    let depths = [depth(start), depth(end), probe.depth()];
    // Last maximum wins: the probe (slot 2) steps first on any tie
    // involving it.
    let deepest = (0..3)
        .max_by_key(|&slot| depths[slot])
        .expect("three cursor slots");

    let flip = match deepest {
        0 => step_bound(start.as_mut().expect("slot 0 is the present start")),
        1 => step_bound(end.as_mut().expect("slot 1 is the present end")),
        _ => {
            let (flip, step) = probe.step();
            fold_probe(&step, start, end);
            flip
        }
    };
    if deepest != 2 && depths[2] >= flip {
        let (tied, step) = probe.step();
        debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
        fold_probe(&step, start, end);
    }
    for (slot, side) in [(0, start), (1, end)] {
        if slot != deepest && depths[slot] >= flip {
            if let Some(side) = side {
                let tied = step_bound(side);
                debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
            }
        }
    }
}

#[cfg(test)]
mod tests;
