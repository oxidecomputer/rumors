//! Places one version relative to an ordered pair of versions.
//!
//! Placement compares a probe with a span's start and end. Running two ordinary
//! comparisons would decode the probe twice. This module instead walks the
//! three versions together, maintaining `probe - start` and `probe - end`.
//!
//! # The walk
//!
//! One leaf cursor reads each stream. At any point their current regions all
//! overlap and therefore nest. The smallest region ends first; the walk folds
//! both differences' signs for that region, then advances every cursor whose
//! region ends at the same boundary.
//!
//! Each sign updates two facts: whether `probe <= bound` is still possible and
//! whether `bound <= probe` is still possible. A probe step updates both
//! differences; a bound step updates only its own.
//!
//! # Answering each question
//!
//! The walk tracks both possible orderings between the probe and each endpoint.
//! Once evidence disproves an ordering, later regions cannot restore it. Each
//! operation therefore stops reading an endpoint as soon as its remaining
//! regions cannot change the answer:
//!
//! - [`span`] needs the complete [`Placement`] relation. If the probe is known
//!   to be concurrent with one endpoint, that endpoint can be dropped, but the
//!   other still distinguishes concurrency with one endpoint from concurrency
//!   with both.
//! - [`dominance`] asks only whether `lo <= probe` and `hi <= probe`. Failure of
//!   the first condition decides [`Dominance::Before`] immediately. Failure of
//!   the second drops `hi`; `lo` then distinguishes
//!   [`Dominance::Between`] from [`Dominance::Before`].
//! - [`precedence`] is the mirror image: it asks whether `probe <= hi` and
//!   `probe <= lo`, and can drop whichever endpoint no longer affects that
//!   distinction.
//! - [`contains`] requires both `lo <= probe` and `probe <= hi`. Either failure
//!   returns `false`; success is known only after both comparisons finish.
//!
//! # Cost
//!
//! Every input bit is visited a constant number of times. Work is linear in the
//! three streams' combined size, while the probe is decoded only once.
//! Transient state is three compact cursor paths and two accumulators.

use core::cmp::Ordering;
use core::ops::ControlFlow;

use suanpan::Accumulator;

use num_bigint::BigUint;

use crate::span::{Dominance, Endpoint, Placement, Precedence};

use super::order::OrderState;
use super::overlay::{advance_set, CursorSet, Side};
use crate::version::io::regions::{RegionReader, VersionRegionReader};
use crate::Version;

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
    cursor: VersionRegionReader<'a>,
    /// `height_probe − height_bound`.
    diff: Accumulator,
    /// The surviving directions of this side's pair, the probe as the `a`
    /// operand: `le` is `probe <= bound` still possible, `ge` is `bound <=
    /// probe`.
    directions: OrderState,
}

impl<'a> BoundSide<'a> {
    /// Open one bound stream at its first leaf and seed its difference from the
    /// probe's absolute first height.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical Version encoding.
    fn open(bits: &'a Version, probe_first: &BigUint) -> BoundSide<'a> {
        let (cursor, first) = VersionRegionReader::open(bits);
        let mut diff = Accumulator::new();
        diff.add_shifted_limbs(0, probe_first.iter_u64_digits());
        diff.sub_shifted_limbs(0, first.iter_u64_digits());
        BoundSide {
            cursor,
            diff,
            directions: OrderState::new(),
        }
    }

    /// Fold this interval's sign into the surviving directions.
    fn read(&mut self) {
        self.directions.fold(self.diff.cmp_zero());
    }

    /// The relation the completed sweep decided, as the causal order.
    fn relation(&self) -> Option<Ordering> {
        self.directions.relation()
    }

    /// Step this bound past its plateau, folding its crossing into its own
    /// difference as the `B` operand; returns the flip level.
    fn step(&mut self) -> u64 {
        let (flip, step) = self.cursor.step();
        Side::B.fold(&mut self.diff, &step);
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
/// The canonical-stream contract of [`Version::partial_cmp`],
/// on all three operands.
pub fn span(probe: &Version, lo: &Version, hi: &Version) -> Placement {
    /// Either endpoint's decided concurrency drops its own cursor while the
    /// other still sweeps.
    ///
    /// `Concurrent(Start)` vs `Concurrent(Both)` needs the other endpoint's
    /// relation; when the refuted side was the last one standing, both
    /// endpoints have refuted and the verdict is fixed.
    fn on_side(state: OrderState, other_live: bool) -> ControlFlow<Placement, Fate> {
        if !state.is_concurrent() {
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
        // A dropped endpoint is concurrent with the probe. `on_side` drops
        // only while the other endpoint remains live; if that endpoint also
        // becomes concurrent, the walk returns Concurrent(Both) immediately.
        // Thus at most one relation can be absent here.
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

/// Coarsen [`span`] to how much of the span the probe dominates.
///
/// If `lo <= probe` fails, the answer is immediately [`Dominance::Before`].
/// If `hi <= probe` fails, the end cursor can be dropped while the start
/// relation distinguishes the remaining outcomes.
///
/// The same operand contract as [`span`].
///
/// # Panics
///
/// The canonical-stream contract of [`Version::partial_cmp`],
/// on all three operands.
pub fn dominance(probe: &Version, lo: &Version, hi: &Version) -> Dominance {
    walk(
        probe,
        lo,
        hi,
        // Failure to dominate the start determines the result.
        |directions, _| {
            if directions.allows_ge() {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(Dominance::Before)
            }
        },
        // Failure to dominate the end rules out After; only the start still
        // matters.
        |directions, _| {
            if directions.allows_ge() {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Continue(Fate::Drop)
            }
        },
        // A surviving end relation means the probe dominates the whole span.
        // Otherwise the start relation must have survived, because its hook
        // returns Before on the first refutation.
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

/// Coarsen [`span`] to how much of the span the probe precedes.
///
/// This mirrors [`dominance`]: failure of `probe <= hi` determines After,
/// while failure of `probe <= lo` lets the walk drop the start cursor.
///
/// The same operand contract as [`span`].
///
/// # Panics
///
/// The canonical-stream contract of [`Version::partial_cmp`],
/// on all three operands.
pub fn precedence(probe: &Version, lo: &Version, hi: &Version) -> Precedence {
    walk(
        probe,
        lo,
        hi,
        // Failure to precede the start rules out Before; only the end matters.
        |directions, _| {
            if directions.allows_le() {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Continue(Fate::Drop)
            }
        },
        // Failure to precede the end determines the result.
        |directions, _| {
            if directions.allows_le() {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(Precedence::After)
            }
        },
        // A surviving start relation means the probe precedes the whole span.
        // Otherwise the end relation must have survived, because its hook
        // returns After on the first refutation.
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
/// Either failed inequality returns `false` immediately; `true` requires both
/// to survive the complete traversal.
///
/// The same operand contract as [`span`].
///
/// # Panics
///
/// The canonical-stream contract of [`Version::partial_cmp`],
/// on all three operands.
pub fn contains(probe: &Version, lo: &Version, hi: &Version) -> bool {
    walk(
        probe,
        lo,
        hi,
        // `lo <= probe` refuted: the probe is below or beside the
        // start — outside the segment, whatever the end relation.
        |directions, _| {
            if directions.allows_ge() {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(false)
            }
        },
        // `probe <= hi` refuted: the probe is above or beside the end.
        |directions, _| {
            if directions.allows_le() {
                ControlFlow::Continue(Fate::Sweep)
            } else {
                ControlFlow::Break(false)
            }
        },
        // Both hooks return false on their first refutation, so reaching the
        // end proves both inequalities.
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

/// Walk the probe with both endpoints, delegating question-specific exits to
/// hooks.
///
/// After each constant region, a hook sees the surviving orderings for its
/// endpoint and whether the other endpoint is still live. It may return the
/// final answer, keep the endpoint, or drop it. `finish` maps the relations
/// left at exhaustion.
///
/// A dropped endpoint and a fully traversed concurrent endpoint both flatten
/// to `None`. A hook may therefore drop an endpoint only after the ordering
/// inspected by its `finish` arm has been permanently refuted.
fn walk<V>(
    probe: &Version,
    start: &Version,
    end: &Version,
    on_start: impl Fn(OrderState, bool) -> ControlFlow<V, Fate>,
    on_end: impl Fn(OrderState, bool) -> ControlFlow<V, Fate>,
    finish: impl FnOnce(Option<Option<Ordering>>, Option<Option<Ordering>>) -> V,
) -> V {
    let (probe, probe_first) = VersionRegionReader::open(probe);
    let mut set = Cursors {
        probe,
        start: Some(BoundSide::open(start, &probe_first)),
        end: Some(BoundSide::open(end, &probe_first)),
    };

    loop {
        // Each live endpoint contributes one sign on this region. End-first is
        // arbitrary because the two relations are independent.
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

/// The probe cursor and the two endpoints that may be dropped early.
struct Cursors<'a> {
    probe: VersionRegionReader<'a>,
    /// The span's minimum endpoint; `None` once a verdict hook drops it.
    start: Option<BoundSide<'a>>,
    /// The span's maximum endpoint, as `start`.
    end: Option<BoundSide<'a>>,
}

/// Slot numbers used by [`CursorSet`].
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

/// Advances the three cursors under the shared boundary rule.
///
/// The probe goes first when boundaries tie because each difference treats it
/// as the left operand. The endpoints do not share arithmetic state, so their
/// relative order is immaterial.
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
                    Side::A.fold(&mut side.diff, &step);
                }
                flip
            }
            Self::START => Self::step_bound(&mut self.start),
            Self::END => Self::step_bound(&mut self.end),
            _ => unreachable!("three cursor slots"),
        }
    }
}

pub mod filter;

#[cfg(test)]
mod tests;
