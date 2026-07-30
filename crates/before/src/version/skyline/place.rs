//! The placement co-walk: one probe stream against one or two bound
//! streams, in a single fused merge, generic over its verdict.
//!
//! `causally`'s placement questions compare one version against up to two
//! bound versions. Composed from the pair sweep ([`sweep`](super::sweep)),
//! that costs one walk per bound — the probe stream decoded once per
//! bound. This module fuses them: one overlay walk over the probe and
//! both bound streams, each decoded once, maintaining one running
//! difference per bound. The pair-difference algebra is
//! [`sweep`](super::sweep)'s ([`OpenedPair`](super::sweep::OpenedPair)
//! seeds a difference the same way; [`fold`] orients every crossing);
//! only the arity and the verdict vocabulary are this walk's.
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
//! # Verdict closures
//!
//! The walk carries the joint relation state; the verdict vocabulary
//! decides how much of it each question needs. Each entry point passes
//! its question as a pair of per-side hooks reacting to the side's
//! surviving directions after every interval's sign fold — `Break` with
//! the determined verdict, or `Continue` with the side's fate: keep
//! sweeping, or drop its cursor (its stream is never scanned further) —
//! plus an exhaustion map over the decided relations. A refuted
//! direction stays refuted, so hooks acting only on refutations stop
//! exactly when their target verdict is determined: every early exit is
//! correct by construction, and each question's exits land where its
//! own verdict lattice says the answer is fixed.
//!
//! - [`range`], the six-way [`Ranged`] verdict under range
//!   semantics: **concurrent to the end bound** is the whole verdict
//!   (on a validated range a probe concurrent to the end cannot be
//!   below the start) — the walk returns at the deciding interval;
//!   **concurrent to the start bound** pins the start relation only,
//!   so the start cursor is dropped and the walk sweeps on over the
//!   probe and end streams alone (with no end bound the verdict is
//!   [`Ranged::Inside`] immediately).
//! - [`span`], the nine-way [`Placement`] verdict: no single
//!   concurrency is the whole verdict — `Concurrent(Start)` vs
//!   `Concurrent(Both)` needs the other endpoint's relation — so a
//!   concurrency-decided side is dropped, and the walk returns early
//!   only when both endpoints have refuted, with
//!   [`Placement::Concurrent`]`(Both)` at the second deciding interval.
//! - [`dominance`], the three-way [`Dominance`] verdict: the
//!   verdict reads only the bound-at-or-below-probe directions, so a
//!   *single* refuted direction acts — `lo <= probe` refuted returns
//!   [`Dominance::Before`] at the refuting interval, the earliest
//!   bail in the placement family, and `hi <= probe` refuted drops the
//!   end cursor while the start relation still decides
//!   [`Between`](Dominance::Between) vs
//!   [`Before`](Dominance::Before).
//!
//! Every other relation ((in)equality against a bound, domination either
//! way) is refutable early but confirmable only at exhaustion, again
//! exactly as in the pair sweep.
//!
//! The span walks speak the public vocabulary directly: the
//! nine-state [`Placement`] and its [`Dominance`] coarsening are raw
//! relation facts against two concrete versions — exactly this layer's
//! vocabulary — so a stream-level duplicate would add a 1:1 mapping
//! with no semantic content. The range walk keeps its own [`Ranged`]
//! vocabulary because what its verdicts *mean* (what `Inside` keeps) is
//! range semantics, stated by `causally`.
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
//! argument, unchanged). The verdict hooks are branch-only: no question
//! adds stream, decode, or accumulator work, so the relational pins in
//! `tests/meter.rs` hold every question to the same identities — the fused
//! walk to the composition minus one probe scan, and the one-bound
//! degenerate form to the pair sweep byte for byte.
//!
//! # Testing
//!
//! The two-walk composition is the oracle, once per question: the
//! `bounded_matches_bound_relations`, `span_place_matches_relations`,
//! and `span_dominance_coarsens_place` laws in [`crate::laws`] pin
//! each verdict to the raw `partial_cmp` verdicts on every law consumer
//! (generated, organic, exhaustive, and fuzzed populations), the
//! stream-level proptests beside this module drive all three entry
//! points against composed pair sweeps, and `causally`'s witness-matrix tests
//! pin every verdict, bound-kind combination, and the coincident corner
//! against constructed inputs. The resource identities are the meter
//! rows named above.

// The module doc names crate-private machinery by intra-doc link so a
// rename cannot rot the prose (the internal doc build resolves every
// link); on the public build those links render as plain code spans —
// the items are private — which this allow accepts.
#![allow(rustdoc::private_intra_doc_links)]

use core::cmp::Ordering;
use core::ops::ControlFlow;

use suanpan::Accumulator;

use crate::causally::{Dominance, Endpoint, Placement};
use crate::codec::{Base, BitsSlice};

use super::sweep::{fold, LeafCursor, PlateauCursor, Side, Step};

/// Where a probe stream's version sits relative to a validated pair of
/// bound streams under *range* semantics: the range walk's verdict, at
/// full resolution.
///
/// Stream-level vocabulary for `causally`'s six range placement
/// verdicts, in the same order; `causally` states the range semantics,
/// this module only computes the relations. `AtStart` is the
/// coincident-bounds canonicalization's home: the start relation is
/// examined first, so a probe equal to coinciding bounds reports
/// `AtStart`, never `AtEnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ranged {
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

/// A side's disposition when a verdict hook leaves the walk running:
/// keep sweeping its stream, or drop its cursor — the side's relation
/// is decided as far as the verdict cares, and its stream is never
/// scanned further.
enum Fate {
    Sweep,
    Drop,
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

    /// Fold this interval's sign into the surviving directions.
    fn read(&mut self) {
        match self.diff.sign() {
            Ordering::Greater => self.le = false,
            Ordering::Less => self.ge = false,
            Ordering::Equal => {}
        }
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

/// Place a probe stream against a validated pair of optional bound
/// streams under range semantics, each stream decoded once.
///
/// `start` and `end`, when both present, must satisfy `start <= end`
/// (`causally`'s composition gate); the verdict is unspecified otherwise.
/// With neither bound the verdict is [`Ranged::Inside`] at zero cost.
///
/// # Panics
///
/// Operands must be canonical skyline streams —
/// [`causal_cmp`](super::sweep::causal_cmp)'s contract exactly: the
/// violations the walk structurally notices panic, the rest sweep
/// silently with an unspecified verdict.
pub(crate) fn range(
    probe: &BitsSlice,
    start: Option<&BitsSlice>,
    end: Option<&BitsSlice>,
) -> Ranged {
    walk(
        probe,
        start,
        end,
        // Concurrent to the start pins the start relation only: drop
        // its cursor and sweep on for the end relation — and with no
        // end bound left in the question, past-or-concurrent to the
        // start is already the contained region.
        |le, ge, end_live| {
            if le || ge {
                ControlFlow::Continue(Fate::Sweep)
            } else if end_live {
                ControlFlow::Continue(Fate::Drop)
            } else {
                ControlFlow::Break(Ranged::Inside)
            }
        },
        // Concurrent to the end is the whole verdict: on a validated
        // range a probe concurrent to the end cannot be below the
        // start.
        |le, ge, _| {
            if le || ge {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(Ranged::ConcurrentToEnd)
            }
        },
        // The start relation speaks first (the coincident-bounds
        // canonicalization); a dropped start cursor is the concurrent
        // case, which falls through to the end relation like `Greater`
        // does, and an absent end keeps everything past the start
        // `Inside`. The `Some(None)` arm is the total non-canonical
        // corner.
        |start, end| {
            match start.flatten() {
                Some(Ordering::Less) => return Ranged::BelowStart,
                Some(Ordering::Equal) => return Ranged::AtStart,
                Some(Ordering::Greater) | None => {}
            }
            match end {
                None => Ranged::Inside,
                Some(Some(Ordering::Less)) => Ranged::Inside,
                Some(Some(Ordering::Equal)) => Ranged::AtEnd,
                Some(Some(Ordering::Greater)) => Ranged::AboveEnd,
                Some(None) => Ranged::ConcurrentToEnd,
            }
        },
    )
}

/// Place a probe stream against an ordered span's endpoint streams
/// at full resolution, each stream decoded once.
///
/// `lo` and `hi` must satisfy `lo <= hi` (`causally::Span`'s
/// construction contract); the verdict is unspecified otherwise.
///
/// # Panics
///
/// The canonical-stream contract of [`range`], on all three operands.
pub(crate) fn span(probe: &BitsSlice, lo: &BitsSlice, hi: &BitsSlice) -> Placement {
    /// Either endpoint's decided concurrency drops its own cursor
    /// while the other still sweeps.
    ///
    /// `Concurrent(Start)` vs `Concurrent(Both)` needs the other
    /// endpoint's relation; when the refuted side was the last one
    /// standing, both endpoints have refuted and the verdict is fixed.
    fn on_side(le: bool, ge: bool, other_live: bool) -> ControlFlow<Placement, Fate> {
        if le || ge {
            ControlFlow::Continue(Fate::Sweep)
        } else if other_live {
            ControlFlow::Continue(Fate::Drop)
        } else {
            ControlFlow::Break(Placement::Concurrent(Endpoint::Both))
        }
    }
    walk(
        probe,
        Some(lo),
        Some(hi),
        on_side,
        on_side,
        // A dropped side is a decided concurrency (the walk is always
        // given both endpoint streams), and so is the total
        // non-canonical `Some(None)` corner — `flatten` folds the two.
        |lo, hi| match (lo.flatten(), hi.flatten()) {
            (Some(Ordering::Less), _) => Placement::Before,
            (Some(Ordering::Equal), Some(Ordering::Equal)) => Placement::At(Endpoint::Both),
            (Some(Ordering::Equal), _) => Placement::At(Endpoint::Start),
            (Some(Ordering::Greater), Some(Ordering::Less)) => Placement::Between,
            (Some(Ordering::Greater), Some(Ordering::Equal)) => Placement::At(Endpoint::End),
            (Some(Ordering::Greater), Some(Ordering::Greater)) => Placement::After,
            (Some(Ordering::Greater), None) => Placement::Concurrent(Endpoint::End),
            (None, None) => Placement::Concurrent(Endpoint::Both),
            (None, _) => Placement::Concurrent(Endpoint::Start),
        },
    )
}

/// The dominance face of [`span`]: the three-way verdict, with the
/// placement family's earliest bail (a refuted `lo <= probe` returns at
/// the refuting interval; a refuted `hi <= probe` stops the end stream's
/// scan).
///
/// The same operand contract as [`span`].
///
/// # Panics
///
/// The canonical-stream contract of [`range`], on all three operands.
pub(crate) fn dominance(probe: &BitsSlice, lo: &BitsSlice, hi: &BitsSlice) -> Dominance {
    walk(
        probe,
        Some(lo),
        Some(hi),
        // `lo <= probe` refuted is the whole verdict — the probe
        // dominates not even the start, whatever the end relation: the
        // family's earliest bail.
        |_le, ge, _| {
            if ge {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(Dominance::Before)
            }
        },
        // `hi <= probe` refuted takes `Dominance::After` off the table, and the
        // verdict now rides the start relation alone — the end stream
        // is never scanned further.
        |_le, ge, _| {
            if ge {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Continue(Fate::Drop)
            }
        },
        // `hi <= probe` surviving to exhaustion is domination of the
        // whole span; otherwise `lo <= probe` surviving is the
        // start. `Dominance::Before` is unreachable here on canonical validated
        // inputs (a refuted start direction returned from the loop) but
        // keeps the map total.
        |lo, hi| {
            if matches!(hi.flatten(), Some(Ordering::Equal | Ordering::Greater)) {
                Dominance::After
            } else if matches!(lo.flatten(), Some(Ordering::Equal | Ordering::Greater)) {
                Dominance::Between
            } else {
                Dominance::Before
            }
        },
    )
}

/// The placement walk: sweep the probe against the present bound
/// streams, folding each elementary interval's signs and letting the
/// verdict hooks act.
///
/// `on_start` and `on_end` see their side's surviving directions after
/// its sign fold, plus whether the other side still sweeps: `Break`
/// carries a determined verdict out of the walk, and `Continue` carries
/// the side's [`Fate`]. The hooks act only on refutations — which are
/// permanent — so an early break or drop never moves a verdict the
/// completed sweep would have reached. `finish` maps the decided
/// relations at exhaustion: `None` for a side absent or dropped,
/// `Some(None)` for a side refuted in both directions at the last
/// interval (unreachable on canonical inputs — every caller keeps the
/// arm total so a non-canonical sweep stays a silent unspecified
/// verdict, per the panics contract).
fn walk<V>(
    probe: &BitsSlice,
    start: Option<&BitsSlice>,
    end: Option<&BitsSlice>,
    on_start: impl Fn(bool, bool, bool) -> ControlFlow<V, Fate>,
    on_end: impl Fn(bool, bool, bool) -> ControlFlow<V, Fate>,
    finish: impl FnOnce(Option<Option<Ordering>>, Option<Option<Ordering>>) -> V,
) -> V {
    if start.is_none() && end.is_none() {
        return finish(None, None);
    }
    let (mut probe, probe_first) = LeafCursor::open(probe);
    let mut start = start.map(|bits| BoundSide::open(bits, &probe_first));
    let mut end = end.map(|bits| BoundSide::open(bits, &probe_first));

    loop {
        // One read per live bound per elementary interval, end first:
        // the range walk's whole-verdict exit is an end-side fact, and
        // the shared order keeps every question's accumulator traffic
        // identical.
        if let Some(side) = &mut end {
            side.read();
            match on_end(side.le, side.ge, start.is_some()) {
                ControlFlow::Continue(Fate::Sweep) => {}
                ControlFlow::Continue(Fate::Drop) => end = None,
                ControlFlow::Break(verdict) => return verdict,
            }
        }
        if let Some(side) = &mut start {
            side.read();
            match on_start(side.le, side.ge, end.is_some()) {
                ControlFlow::Continue(Fate::Sweep) => {}
                ControlFlow::Continue(Fate::Drop) => start = None,
                ControlFlow::Break(verdict) => return verdict,
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

    finish(
        start.as_ref().map(BoundSide::relation),
        end.as_ref().map(BoundSide::relation),
    )
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
