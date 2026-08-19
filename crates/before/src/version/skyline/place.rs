//! The placement co-walk: one probe stream against a span's two bound streams,
//! in a single fused merge, generic over its verdict.
//!
//! `causally`'s placement questions compare one version against a span's two
//! bound versions. Composed from the pair sweep ([`sweep`](super::sweep)),
//! that costs one walk per bound — the probe stream decoded once per bound.
//! This module fuses them: one overlay walk over the probe and both bound
//! streams, each decoded once, maintaining one running difference per bound.
//! The pair-difference algebra is [`overlay`](super::overlay)'s
//! ([`OpenedPair`](super::overlay::OpenedPair) seeds a difference the same way;
//! [`fold`] orients every crossing); only the arity and the verdict vocabulary
//! are this walk's.
//!
//! # The walk
//!
//! One [`LeafCursor`] per stream. All current leaves contain the sweep
//! point, so they nest by depth, and the walk advances by the overlay-advance
//! law at arity three ([`advance_set`], the [`masked`](super::masked) walk's
//! precedent); this walk contributes only its slot roster ([`Cursors`]) and
//! what each slot's step folds.
//!
//! Per bound, the walk maintains the running difference `D = height_probe −
//! height_bound` on the cliff-free [`Accumulator`] and folds its sign once
//! per elementary interval into the bound's two surviving-direction flags — the
//! same `(probe <= bound, bound <= probe)` pair the comparison sweep folds, in
//! that orientation everywhere: the probe is every pair's `a` operand. A probe
//! crossing folds into both differences; a bound crossing folds into its own.
//! Each accumulator therefore sees exactly the write sequence the corresponding
//! pair sweep would commit — the identity the resource pins in
//! `tests/meter.rs`'s placement rows rest on, pricing each fused walk against
//! its pair-sweep composition.
//!
//! # Verdict closures
//!
//! The walk carries the joint relation state; the verdict vocabulary decides
//! how much of it each question needs. Each entry point passes its question as
//! a pair of per-side hooks reacting to the side's surviving directions after
//! every interval's sign fold — `Break` with the determined verdict, or
//! `Continue` with the side's fate: keep sweeping, or drop its cursor (its
//! stream is never scanned further) — plus an exhaustion map over the decided
//! relations. A refuted direction stays refuted, so hooks acting only on
//! refutations stop exactly when their target verdict is determined: every
//! early exit is correct by construction, and each question's exits land where
//! its own verdict lattice says the answer is fixed.
//!
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
//! - [`precedence`], the three-way [`Precedence`] verdict:
//!   [`dominance`] mirrored — the verdict reads only the
//!   probe-at-or-below-bound directions, so `probe <= hi` refuted
//!   returns [`Precedence::After`] at the refuting interval, and
//!   `probe <= lo` refuted drops the start cursor while the end
//!   relation still decides [`Between`](Precedence::Between) vs
//!   [`After`](Precedence::After).
//! - [`contains`], the membership verdict `lo <= probe <= hi`: both
//!   watched directions are required, so either side's refutation is
//!   the whole verdict — `false` at the refuting interval — and `true`
//!   confirms only at exhaustion.
//!
//! Every other relation ((in)equality against a bound, domination either way)
//! is refutable early but confirmable only at exhaustion, again exactly as in
//! the pair sweep.
//!
//! The span walks speak the public vocabulary directly: the nine-state
//! [`Placement`], its [`Dominance`] and [`Precedence`] coarsenings, and the
//! membership verdict are raw relation facts against two concrete versions —
//! exactly this layer's vocabulary — so a stream-level duplicate would add a
//! 1:1 mapping with no semantic content. The query filter walks, which sweep
//! one or two probes against any number of demand-carrying bound streams in the
//! same idiom, live in [`filter`].
//!
//! # Cost
//!
//! Derived, mirroring [`overlay`](super::overlay)'s argument stream by stream:
//! every topology bit of every stream is read at most once, every path
//! bit pushed and popped at most once, and every leaf payload decoded once and
//! folded into at most two accumulators (a constant factor over the pair sweep,
//! paid only by the probe's own deltas). Scan, decode, and stack work are
//! linear in the streams' bits — `O(|v| + |s| + |e|)` against the
//! two-walk composition's `O(2|v| + |s| + |e|)` — and the per-interval sign
//! reads ride the accumulator's amortized-O(1) collapse ([`suanpan`]'s
//! argument, unchanged). The verdict hooks are branch-only: no question adds
//! stream, decode, or accumulator work, so the relational pins in
//! `tests/meter.rs` hold every question to the same identity — the fused walk
//! to the composition minus one probe scan.
//!
//! # Testing
//!
//! The two-walk composition is the oracle, once per question: the
//! `span_place_matches_relations`, `span_dominance_coarsens_place`,
//! `span_precedence_coarsens_place`, and `span_contains_matches_place` laws in
//! [`crate::laws`] pin each verdict to the raw `partial_cmp` verdicts on every
//! law consumer (generated, organic, exhaustive, and fuzzed populations), the
//! stream-level proptests beside this module drive the entry points against
//! composed pair sweeps, and `causally`'s witness-matrix tests pin every
//! verdict and the coincident corner against constructed inputs. The resource
//! identities are the meter rows named above.

use core::cmp::Ordering;
use core::ops::ControlFlow;

use suanpan::Accumulator;

use crate::codec::{BitsView, Int};
use crate::span::{Dominance, Endpoint, Placement, Precedence};

use super::overlay::{advance_set, fold, CursorSet, LeafCursor, PlateauCursor, Side};
use super::signed::Sign;
use super::sweep::Directions;

/// A side's disposition when a verdict hook leaves the walk running: keep
/// sweeping its stream, or drop its cursor — the side's relation is decided as
/// far as the verdict cares, and its stream is never scanned further.
enum Fate {
    Sweep,
    Drop,
}

/// One bound's side of the walk: its cursor, its running difference `D =
/// height_probe − height_bound`, and the two surviving directions.
struct BoundSide<'a> {
    cursor: LeafCursor<'a>,
    /// `height_probe − height_bound`, on the cliff-free accumulator.
    diff: Accumulator,
    /// The surviving directions of this side's pair, the probe as the `a`
    /// operand: `le` is `probe <= bound` still possible, `ge` is `bound <=
    /// probe`.
    directions: Directions,
}

impl<'a> BoundSide<'a> {
    /// Open one bound stream at its first leaf and seed its difference from the
    /// probe's absolute first height.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    fn open(bits: BitsView<'a>, probe_first: &Int) -> BoundSide<'a> {
        let (cursor, first) = LeafCursor::open(bits);
        let mut diff = Accumulator::new();
        super::signed::fold_signed_int(&mut diff, Sign::Positive, probe_first);
        super::signed::fold_signed_int(&mut diff, Sign::Negative, &first);
        BoundSide {
            cursor,
            diff,
            directions: Directions::new(),
        }
    }

    /// Fold this interval's sign into the surviving directions.
    fn read(&mut self) {
        self.directions.fold(self.diff.sign());
    }

    /// The relation the completed sweep decided, as the causal order.
    fn relation(&self) -> Option<Ordering> {
        self.directions.relation()
    }

    /// Step this bound past its plateau, folding its crossing into its own
    /// difference as the `B` operand; returns the flip level.
    fn step(&mut self) -> u64 {
        let (flip, step) = self.cursor.step();
        fold(&mut self.diff, Side::B, step.sign, &step.magnitude);
        flip
    }
}

/// Place a probe stream against an ordered span's endpoint streams at full
/// resolution, each stream decoded once.
///
/// `lo` and `hi` must satisfy `lo <= hi` (`Span`'s construction
/// contract); the verdict is unspecified otherwise.
///
/// # Panics
///
/// The canonical-stream contract of [`causal_cmp`](super::sweep::causal_cmp),
/// on all three operands.
pub(crate) fn span(probe: BitsView<'_>, lo: BitsView<'_>, hi: BitsView<'_>) -> Placement {
    /// Either endpoint's decided concurrency drops its own cursor while the
    /// other still sweeps.
    ///
    /// `Concurrent(Start)` vs `Concurrent(Both)` needs the other endpoint's
    /// relation; when the refuted side was the last one standing, both
    /// endpoints have refuted and the verdict is fixed.
    fn on_side(directions: Directions, other_live: bool) -> ControlFlow<Placement, Fate> {
        if directions.le || directions.ge {
            ControlFlow::Continue(Fate::Sweep)
        } else if other_live {
            ControlFlow::Continue(Fate::Drop)
        } else {
            ControlFlow::Break(Placement::Concurrent(Endpoint::Both))
        }
    }
    walk(
        probe,
        lo,
        hi,
        on_side,
        on_side,
        // A dropped side is a decided concurrency — `flatten` folds the two
        // spellings, soundly per `walk`'s obligation: `on_side` drops only at
        // a both-directions refutation, exactly the relation the `Concurrent`
        // arms below read a `None` as. Both sides `None` is control-flow
        // impossible, on any input: `on_side` drops a side only while the
        // other is live, and the last live side to refute breaks
        // `Concurrent(Both)` from the loop itself — so an undecided start
        // leaves the end decided. The assertion keeps that argument loud: a
        // hook or loop change that ever lets both sides go undecided fails
        // debug builds at the seam instead of silently mislabeling the
        // concurrency's extent.
        |lo, hi| match (lo.flatten(), hi.flatten()) {
            (Some(Ordering::Less), _) => Placement::Before,
            (Some(Ordering::Equal), Some(Ordering::Equal)) => Placement::At(Endpoint::Both),
            (Some(Ordering::Equal), _) => Placement::At(Endpoint::Start),
            (Some(Ordering::Greater), Some(Ordering::Less)) => Placement::Between,
            (Some(Ordering::Greater), Some(Ordering::Equal)) => Placement::At(Endpoint::End),
            (Some(Ordering::Greater), Some(Ordering::Greater)) => Placement::After,
            (Some(Ordering::Greater), None) => Placement::Concurrent(Endpoint::End),
            (None, hi) => {
                debug_assert!(
                    hi.is_some(),
                    "the last live side to refute breaks Concurrent(Both) from the loop, so an undecided start leaves the end decided"
                );
                Placement::Concurrent(Endpoint::Start)
            }
        },
    )
}

/// The dominance face of [`span`]: the three-way verdict, with the placement
/// family's earliest bail (a refuted `lo <= probe` returns at the refuting
/// interval; a refuted `hi <= probe` stops the end stream's scan).
///
/// The same operand contract as [`span`].
///
/// # Panics
///
/// The canonical-stream contract of [`causal_cmp`](super::sweep::causal_cmp),
/// on all three operands.
pub(crate) fn dominance(probe: BitsView<'_>, lo: BitsView<'_>, hi: BitsView<'_>) -> Dominance {
    walk(
        probe,
        lo,
        hi,
        // `lo <= probe` refuted is the whole verdict — the probe dominates not
        // even the start, whatever the end relation: the family's earliest
        // bail.
        |directions, _| {
            if directions.ge {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(Dominance::Before)
            }
        },
        // `hi <= probe` refuted takes `Dominance::After` off the table, and the
        // verdict now rides the start relation alone — the end stream is never
        // scanned further.
        |directions, _| {
            if directions.ge {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Continue(Fate::Drop)
            }
        },
        // `hi <= probe` surviving to exhaustion is domination of the whole
        // span; otherwise `lo <= probe` survived — the start hook breaks
        // `Dominance::Before` at any interval refuting it (and never drops
        // the start side) before the exhaustion check runs, on any input, so
        // reaching this arm proves the start relation. The assertion keeps
        // that control-flow argument loud: a hook or loop change that ever
        // admits a refuted start here fails debug builds at the seam instead
        // of silently re-deciding the verdict. The `flatten` merge is sound
        // (`walk`'s obligation): the end side drops only when `hi <= probe`
        // is refuted, so a dropped end flattens to the same
        // not-`Equal`/`Greater` answer its decided relation could ever have
        // given.
        |lo, hi| {
            if matches!(hi.flatten(), Some(Ordering::Equal | Ordering::Greater)) {
                Dominance::After
            } else {
                debug_assert!(
                    matches!(lo.flatten(), Some(Ordering::Equal | Ordering::Greater)),
                    "dominance's start hook breaks Before on refutation, so exhaustion proves lo <= probe"
                );
                Dominance::Between
            }
        },
    )
}

/// The precedence face of [`span`]: [`dominance`] mirrored — the three-way
/// verdict over the probe-at-or-below-bound directions.
///
/// The bail mirrors too: a refuted `probe <= hi` returns at the refuting
/// interval; a refuted `probe <= lo` stops the start stream's scan.
///
/// The same operand contract as [`span`].
///
/// # Panics
///
/// The canonical-stream contract of [`causal_cmp`](super::sweep::causal_cmp),
/// on all three operands.
pub(crate) fn precedence(probe: BitsView<'_>, lo: BitsView<'_>, hi: BitsView<'_>) -> Precedence {
    walk(
        probe,
        lo,
        hi,
        // `probe <= lo` refuted takes `Precedence::Before` off the table, and
        // the verdict now rides the end relation alone — the start stream is
        // never scanned further.
        |directions, _| {
            if directions.le {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Continue(Fate::Drop)
            }
        },
        // `probe <= hi` refuted is the whole verdict — the probe precedes not
        // even the end, whatever the start relation: the dominance bail,
        // mirrored.
        |directions, _| {
            if directions.le {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(Precedence::After)
            }
        },
        // `probe <= lo` surviving to exhaustion is precedence of the whole
        // span; otherwise `probe <= hi` survived — the end hook breaks
        // `Precedence::After` at any interval refuting it (and never drops
        // the end side) before the exhaustion check runs, on any input, so
        // reaching this arm proves the end relation. The assertion keeps
        // that control-flow argument loud: a hook or loop change that ever
        // admits a refuted end here fails debug builds at the seam instead
        // of silently re-deciding the verdict. The `flatten` merge is sound
        // (`walk`'s obligation): the start side drops only when `probe <=
        // lo` is refuted, so a dropped start flattens to the same
        // not-`Equal`/`Less` answer its decided relation could ever have
        // given.
        |lo, hi| {
            if matches!(lo.flatten(), Some(Ordering::Equal | Ordering::Less)) {
                Precedence::Before
            } else {
                debug_assert!(
                    matches!(hi.flatten(), Some(Ordering::Equal | Ordering::Less)),
                    "precedence's end hook breaks After on refutation, so exhaustion proves probe <= hi"
                );
                Precedence::Between
            }
        },
    )
}

/// The membership face of [`span`]: whether `lo <= probe <= hi`.
///
/// Both watched directions are required, so either side's refutation is the
/// whole verdict — the walk bails at the first interval refuting `lo <= probe`
/// or `probe <= hi` — while `true` confirms only at exhaustion, exactly as the
/// pair sweep confirms domination.
///
/// The same operand contract as [`span`].
///
/// # Panics
///
/// The canonical-stream contract of [`causal_cmp`](super::sweep::causal_cmp),
/// on all three operands.
pub(crate) fn contains(probe: BitsView<'_>, lo: BitsView<'_>, hi: BitsView<'_>) -> bool {
    walk(
        probe,
        lo,
        hi,
        // `lo <= probe` refuted: the probe is below or beside the
        // start — outside the segment, whatever the end relation.
        |directions, _| {
            if directions.ge {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(false)
            }
        },
        // `probe <= hi` refuted: the probe is above or beside the end.
        |directions, _| {
            if directions.le {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(false)
            }
        },
        // Both watched directions survived to exhaustion, and the hooks
        // break on any refutation before the exhaustion check runs — so
        // reaching this arm IS the membership verdict, on any input. The
        // assertion keeps that control-flow argument loud: a hook or loop
        // change that ever admits a refuted sweep here fails debug builds
        // at the seam instead of silently walking full streams to re-derive
        // (or corrupt) the verdict.
        |lo, hi| {
            debug_assert!(
                matches!(lo.flatten(), Some(Ordering::Equal | Ordering::Greater))
                    && matches!(hi.flatten(), Some(Ordering::Equal | Ordering::Less)),
                "contains' hooks break on refutation, so exhaustion admits the probe"
            );
            true
        },
    )
}

/// The placement walk: sweep the probe against the two bound streams, folding
/// each elementary interval's signs and letting the verdict hooks act.
///
/// `on_start` and `on_end` see their side's surviving [`Directions`] after its
/// sign fold, plus whether the other side still sweeps: `Break` carries a
/// determined verdict out of the walk, and `Continue` carries the side's
/// [`Fate`]. The hooks act only on refutations — which are permanent — so an
/// early break or drop never moves a verdict the completed sweep would have
/// reached. `finish` maps the decided relations at exhaustion: `None` for a
/// dropped side, `Some(None)` for a side refuted in both directions at the
/// last interval — deliverable only by hooks that leave a refutation
/// standing. Each side's hook runs after its final interval's fold, before
/// the exhaustion check, so hooks that act on the refutations their finish
/// arm tests rule that corner out by control flow, on any input; every
/// entry point's hooks do, and each finish arm debug-asserts what its own
/// hooks prove.
///
/// Obligation on every caller: a `finish` arm that reads a side through
/// `flatten` merges "dropped" with "swept to concurrent", so the side's drop
/// condition must agree with that arm's reading — the hook may drop a side only
/// when the direction the finish arm tests is already refuted, making the
/// flattened `None` and the decided relation give the same answer. Each entry
/// point carries the per-verdict argument at its closures.
fn walk<V>(
    probe: BitsView<'_>,
    start: BitsView<'_>,
    end: BitsView<'_>,
    on_start: impl Fn(Directions, bool) -> ControlFlow<V, Fate>,
    on_end: impl Fn(Directions, bool) -> ControlFlow<V, Fate>,
    finish: impl FnOnce(Option<Option<Ordering>>, Option<Option<Ordering>>) -> V,
) -> V {
    let (probe, probe_first) = LeafCursor::open(probe);
    let mut set = Cursors {
        probe,
        start: Some(BoundSide::open(start, &probe_first)),
        end: Some(BoundSide::open(end, &probe_first)),
    };

    loop {
        // One read per live bound per elementary interval, end first — a fixed
        // order, so every question's accumulator traffic is identical (the
        // committed meter rows pin the write sequences). End-first is
        // arbitrary; only fixedness matters — each side's directions fold only
        // its own difference's sign, so read order cannot move a verdict.
        if let Some(side) = &mut set.end {
            side.read();
            match on_end(side.directions, set.start.is_some()) {
                ControlFlow::Continue(Fate::Sweep) => {}
                ControlFlow::Continue(Fate::Drop) => set.end = None,
                ControlFlow::Break(verdict) => return verdict,
            }
        }
        if let Some(side) = &mut set.start {
            side.read();
            match on_start(side.directions, set.end.is_some()) {
                ControlFlow::Continue(Fate::Sweep) => {}
                ControlFlow::Continue(Fate::Drop) => set.start = None,
                ControlFlow::Break(verdict) => return verdict,
            }
        }
        let exhausted = set.probe.done()
            && set.start.as_ref().is_none_or(|side| side.cursor.done())
            && set.end.as_ref().is_none_or(|side| side.cursor.done());
        if exhausted {
            break;
        }
        advance_set(&mut set);
    }

    finish(
        set.start.as_ref().map(BoundSide::relation),
        set.end.as_ref().map(BoundSide::relation),
    )
}

/// The placement walk's cursor roster: the probe stream's cursor and the two
/// droppable bound sides, advancing under the overlay-advance law
/// ([`advance_set`]).
struct Cursors<'a> {
    probe: LeafCursor<'a>,
    /// The span's minimum endpoint; `None` once a verdict hook drops it.
    start: Option<BoundSide<'a>>,
    /// The span's maximum endpoint, as `start`.
    end: Option<BoundSide<'a>>,
}

/// The walk's cursor slots, named for [`CursorSet`]'s numbered vocabulary.
impl Cursors<'_> {
    /// The probe stream's slot.
    const PROBE: usize = 0;
    /// The start bound's slot.
    const START: usize = 1;
    /// The end bound's slot.
    const END: usize = 2;

    /// Step one bound slot; a dropped side never steps (its depth reads zero,
    /// and every flip level is at least one).
    fn step_bound(side: &mut Option<BoundSide<'_>>) -> u64 {
        side.as_mut()
            .expect("a dropped side reads depth zero and never steps")
            .step()
    }
}

/// The placement walk's slot roster for the overlay-advance law
/// ([`advance_set`]).
///
/// Priority `[PROBE, START, END]`: the probe steps first on every tie — it is
/// every pair's first operand, and the binary law's equal-depth arm steps its
/// first operand first — keeping each accumulator's write sequence identical
/// to its pair sweep's (the placement identity rows in `tests/meter.rs` pin
/// each fused walk against its composed sweeps). The start/end order among
/// themselves moves no committed reading: the two bounds share no accumulator.
impl CursorSet for Cursors<'_> {
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static {
        [Self::PROBE, Self::START, Self::END].into_iter()
    }

    /// A dropped side reads zero, like the masked walk's absent mask.
    fn depth(&self, slot: usize) -> u64 {
        match slot {
            Self::PROBE => self.probe.depth(),
            Self::START => self.start.as_ref().map_or(0, |side| side.cursor.depth()),
            Self::END => self.end.as_ref().map_or(0, |side| side.cursor.depth()),
            _ => unreachable!("three cursor slots"),
        }
    }

    /// The probe's step folds its crossing into every live difference as the
    /// `A` operand (the probe is every pair's first operand); a bound's step
    /// folds into its own difference as the `B` operand.
    fn step(&mut self, slot: usize) -> u64 {
        match slot {
            Self::PROBE => {
                let (flip, step) = self.probe.step();
                for side in [&mut self.start, &mut self.end].into_iter().flatten() {
                    fold(&mut side.diff, Side::A, step.sign, &step.magnitude);
                }
                flip
            }
            Self::START => Self::step_bound(&mut self.start),
            Self::END => Self::step_bound(&mut self.end),
            _ => unreachable!("three cursor slots"),
        }
    }
}

pub(crate) mod filter;

#[cfg(test)]
mod tests;
