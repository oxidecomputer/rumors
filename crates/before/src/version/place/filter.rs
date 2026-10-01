//! The query filter co-walks every bound stream against one or two probe
//! streams.
//!
//! `causally`'s queries hold a floor, a ceiling, and holes — each one bound
//! version with a [`Demand`] on its relation to a probe. Composed from the pair
//! sweep, evaluating a query would decode the probe once per bound. The fused
//! walk instead shares one probe traversal and one running probe height. Each
//! bound also keeps its absolute height.
//!
//! Comparing those heights is cheap while they retain similar numbers of
//! digits: the pair keeps `probe - bound` and applies later changes directly to
//! it. This difference is private to one bound, whereas the probe height is
//! shared by every bound. If the probe grows much wider, copying it into every
//! private difference would multiply memory by the number of bounds. The
//! filter therefore discards a difference before such a change and compares
//! the two shared heights without copying either one. This policy belongs here
//! because it controls the query walk's per-bound cache; it is not part of
//! Version arithmetic itself.
//!
//! The walk advances by the overlay-advance law ([`advance_set`]), and the
//! verdict hooks are branch-only.
//!
//! # Early exit
//!
//! A refuted direction is permanent, so every verdict acts at the earliest
//! interval its lattice allows:
//!
//! - [`admits`]: a floor or ceiling *requires* its direction — the
//!   first interval refuting it returns `false`, the membership walk's
//!   earliest bail. A hole is satisfied by a refutation — its stream is
//!   dropped and never scanned further — and a walk left holding only
//!   satisfied holes returns `true` without exhausting the probe.
//!   Subtractions and dominations confirm only at exhaustion, exactly
//!   as in the pair sweep.
//! - [`coverage`]: a floor refuting `floor <= hi` (or a ceiling
//!   refuting `lo <= ceiling`) proves no covered version is admitted —
//!   [`Coverage::Empty`] at the refuting interval, the verdict a
//!   pruning tree walk consumes. A hole whose subtraction is refuted at
//!   both endpoints is settled and drops its stream, a probe endpoint
//!   whose every pair is settled drops its own cursor, and a walk left
//!   holding only settled holes returns [`Coverage::Full`] without
//!   exhausting anything. `Partial` alone always confirms at
//!   exhaustion: refuting `Full` mid-walk takes a required bound, and
//!   a required bound keeps `Empty` possible to the last interval.
//!
//! # Cost
//!
//! Every topology bit is read once and every leaf payload is decoded once. Let
//! `k` be the number of bounds, `i` the number of intervals in the streams'
//! common overlay, `p` the payload bytes in the probe stream or streams, and
//! `n` all input bytes. Topology work is
//! `O(n + k·i)`; numeric work is `O(n + k·p)` because a probe delta may feed
//! every live exact difference. The total is therefore `O(n + k·(i + p))`, or
//! `O(k·n)` using bytes alone.
//!
//! Auxiliary state is `O(n + k)`: the walk keeps one fixed-size record and one
//! absolute height per bound. A private difference exists only while its two
//! absolute heights retain similar numbers of digits, so its size is bounded
//! by the bound height it accompanies. If a change makes the shared probe much
//! wider, the filter discards the difference before applying that change. When
//! cancellation makes a height retain many leading digits that no longer
//! affect its value, the stability comparison compacts that one shared height
//! rather than copying it into every comparison.
//!
//! Two parts of this bound are amortized. First, an accumulator comparison may
//! scan leading cancellation, but it compacts the digits it scans; they cannot
//! impose that cost again until later input rewrites them. Second, after a
//! private difference is discarded, rebuilding it requires the heights to
//! become close again. That takes either another width-changing payload or the
//! removal of cancellation created by earlier payloads. The intervening input
//! therefore pays for the copied width. Across `k` bounds, a shared probe
//! payload may still be processed `k` times, as the `k·p` time term states, but
//! those copies are not all retained when the bounds are narrow.
//!
//! Compared with composing binary sweeps, the fused walk avoids decoding the
//! probe once per bound; it does not eliminate the work of evaluating `k`
//! independent demands at each interval.

use core::cmp::Ordering;

use num_bigint::BigUint;
use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;
use crate::causally::Coverage;

use super::super::order::OrderState;
use super::super::overlay::{advance_set, CursorSet, Side};
use crate::version::io::regions::{HeightChange, RegionReader, VersionRegionReader};
use crate::Version;

#[cfg(test)]
mod tests;

/// What a query demands of the relation between the probe and one bound stream,
/// in the probe-first orientation (`le` is `probe <= bound`).
///
/// The first two are *required* relations (a floor and a ceiling — both
/// inclusive, `causally`'s normal form): refuting them refutes membership. The
/// rest are *excluded* relations, the four hole kinds: membership survives
/// exactly when the named relation fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Demand {
    /// `bound <= probe` must hold: a floor.
    After,
    /// `probe <= bound` must hold: a ceiling.
    Before,
    /// `probe <= bound` must fail: a hole subtracting an inclusive down-set.
    NotBefore,
    /// `probe < bound` must fail: a hole subtracting a strict down-set.
    NotStrictlyBefore,
    /// `bound <= probe` must fail: a hole subtracting an inclusive up-set.
    NotAfter,
    /// `bound < probe` must fail: a hole subtracting a strict up-set.
    NotStrictlyAfter,
}

/// One (probe, bound) comparison and its surviving directions.
///
/// While the heights retain similar numbers of digits, this stores
/// `probe - bound` and applies both streams' later changes to it. If one height
/// becomes much wider, the difference is discarded before that change is
/// applied; later reads compare the two absolute heights without copying them.
///
/// Settlement — a comparison whose verdict contribution is fixed mid-walk —
/// belongs to the coverage walk ([`GatedComparison`]); the membership walk
/// never settles a comparison, so this type does not carry that state.
struct Comparison {
    difference: Option<Accumulator>,
    directions: OrderState,
}

/// Whether an exact difference remains cheap enough to keep per bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeightWidths {
    /// The wider height occupies at most two more accumulator positions.
    Close,
    /// The probe may be arbitrarily wider than this bound.
    ProbeFarWider,
    /// The bound may be arbitrarily wider than the probe.
    BoundFarWider,
}

impl HeightWidths {
    /// The greatest width lead permitted in a private difference.
    ///
    /// Accumulators use base-2^32 signed digits. Their fast stability check may
    /// descend one digit before the sign is settled, then needs two positions
    /// of clearance over the other value. A lead greater than this limit is
    /// therefore where the no-copy comparison becomes useful. Keeping the
    /// limit here also caps a private difference at two digits beyond the
    /// narrower height; the `ProbeFarWider` case has no such upper bound.
    const MAX_CLOSE_LEAD: usize = 2;

    /// Classify the heights by their retained working widths.
    fn between(probe: &Accumulator, bound: &Accumulator) -> Self {
        let probe_digits = probe.stored_digit_count();
        let bound_digits = bound.stored_digit_count();
        if probe_digits > bound_digits.saturating_add(Self::MAX_CLOSE_LEAD) {
            Self::ProbeFarWider
        } else if bound_digits > probe_digits.saturating_add(Self::MAX_CLOSE_LEAD) {
            Self::BoundFarWider
        } else {
            Self::Close
        }
    }
}

impl Comparison {
    /// A comparison before either stream has been read.
    fn new() -> Comparison {
        Comparison {
            difference: None,
            directions: OrderState::new(),
        }
    }

    /// Fold this interval's height relation into the surviving directions.
    fn read(&mut self, probe: &mut Accumulator, bound: &mut Accumulator) {
        let sign = match &mut self.difference {
            Some(difference) => difference.cmp_zero(),
            None => {
                let (sign, difference) = Self::compare_heights(probe, bound);
                self.difference = difference;
                sign
            }
        };
        self.directions.fold(sign);
    }

    /// Fold one crossing unless it makes a private difference too wide.
    fn fold(&mut self, side: Side, probe: &Accumulator, bound: &Accumulator, step: &HeightChange) {
        if self.difference.is_some() && HeightWidths::between(probe, bound) != HeightWidths::Close {
            self.difference = None;
            return;
        }
        if let Some(difference) = &mut self.difference {
            side.fold(difference, step);
        }
    }

    /// The relation the completed sweep decided, as the causal order.
    fn relation(&self) -> Option<Ordering> {
        self.directions.relation()
    }

    /// Put an absolute Version height into the accumulator representation.
    fn absolute_height(first: &BigUint) -> Accumulator {
        let mut height = Accumulator::new();
        height.add_shifted_limbs(0, first.iter_u64_digits());
        height
    }

    /// Compare two nonnegative heights without copying a much wider operand.
    ///
    /// Close heights get one exact difference, which later crossings update in
    /// place. For a much wider height, `cmp_zero_stable_under` either proves
    /// that the narrower height cannot affect the order or compacts the wider
    /// height. A refusal can reveal another layer of cancellation on the other
    /// side, so the wider side may alternate. Every refusal strictly shortens
    /// the side it examines, bounding the loop by the two heights' initial
    /// combined width. An exact difference is never built from a far-wider
    /// operand.
    fn compare_heights(
        probe: &mut Accumulator,
        bound: &mut Accumulator,
    ) -> (Ordering, Option<Accumulator>) {
        loop {
            match HeightWidths::between(probe, bound) {
                HeightWidths::Close => break,
                HeightWidths::ProbeFarWider => {
                    let previous_width = probe.stored_digit_count();
                    if let Some(sign) = probe.cmp_zero_stable_under(bound.stored_bits()) {
                        debug_assert_eq!(
                            sign,
                            Ordering::Greater,
                            "Version heights are nonnegative"
                        );
                        return (Ordering::Greater, None);
                    }
                    debug_assert!(
                        probe.stored_digit_count() < previous_width,
                        "a refused stability check compacts the examined height"
                    );
                }
                HeightWidths::BoundFarWider => {
                    let previous_width = bound.stored_digit_count();
                    if let Some(sign) = bound.cmp_zero_stable_under(probe.stored_bits()) {
                        debug_assert_eq!(
                            sign,
                            Ordering::Greater,
                            "Version heights are nonnegative"
                        );
                        return (Ordering::Less, None);
                    }
                    debug_assert!(
                        bound.stored_digit_count() < previous_width,
                        "a refused stability check compacts the examined height"
                    );
                }
            }
            // The shorter representation may expose the other side as wider;
            // classifying again removes its next layer of cancellation.
        }

        debug_assert_eq!(HeightWidths::between(probe, bound), HeightWidths::Close);
        let mut difference = Accumulator::new();
        if probe.stored_digit_count() <= bound.stored_digit_count() {
            difference += &*probe;
            difference -= &*bound;
        } else {
            difference += &*bound;
            difference -= &*probe;
            difference = -difference;
        }
        let sign = difference.cmp_zero();
        (sign, Some(difference))
    }
}

/// A coverage pair and whether it still feeds the verdict.
///
/// A pair is *settled* — `live == false` — when a refutation fixed everything
/// the verdict will ever need from it: a settled pair stops folding and
/// reading (its stream may still advance for the other pair riding the same
/// cursor).
struct GatedComparison {
    comparison: Comparison,
    live: bool,
}

impl GatedComparison {
    /// A live comparison before either stream has been read.
    fn new() -> GatedComparison {
        GatedComparison {
            comparison: Comparison::new(),
            live: true,
        }
    }
}

// ───────────────────────────── membership ─────────────────────────────

/// One bound's side of the membership walk.
struct BoundSide<'a> {
    cursor: VersionRegionReader<'a>,
    /// The bound's absolute height.
    bound: Accumulator,
    comparison: Comparison,
    demand: Demand,
}

/// Whether the probe stream's version satisfies every demand, each stream
/// decoded once — `causally`'s membership predicate at the stream layer.
///
/// An empty demand list is vacuously `true` at zero cost. Demands are read in
/// their supplied order on each interval, so the first decisive one exits.
///
/// # Panics
///
/// Operands must be canonical Version streams — the placement walk's contract
/// exactly: the violations the walk structurally notices panic, the rest sweep
/// silently with an unspecified verdict.
pub fn admits<'a>(
    probe: &'a Version,
    bounds: impl IntoIterator<Item = (&'a Version, Demand)>,
) -> bool {
    let mut bounds = bounds.into_iter().peekable();
    if bounds.peek().is_none() {
        return true;
    }
    let (probe, probe_first) = VersionRegionReader::open(probe);
    let sides: Vec<Option<BoundSide<'a>>> = bounds
        .map(|(bits, demand)| {
            let (cursor, first) = VersionRegionReader::open(bits);
            Some(BoundSide {
                cursor,
                bound: Comparison::absolute_height(&first),
                comparison: Comparison::new(),
                demand,
            })
        })
        .collect();
    let mut live = sides.len();
    let mut walk = MemberCursors {
        probe,
        probe_height: Comparison::absolute_height(&probe_first),
        sides,
    };

    loop {
        // One read per live bound per elementary interval, in demand order.
        for slot in &mut walk.sides {
            let Some(side) = slot else { continue };
            side.comparison
                .read(&mut walk.probe_height, &mut side.bound);
            let directions = side.comparison.directions;
            match side.demand {
                // A required direction refuted refutes membership: the walk's
                // earliest bail. (Both required demands are inclusive —
                // `After` is `bound <= probe`, equality admitted.)
                Demand::After if !directions.allows_ge() => return false,
                Demand::Before if !directions.allows_le() => return false,
                // A hole's subtracting direction refuted satisfies the hole:
                // drop its cursor, its stream is never scanned further.
                Demand::NotBefore | Demand::NotStrictlyBefore if !directions.allows_le() => {
                    *slot = None;
                    live -= 1;
                }
                Demand::NotAfter | Demand::NotStrictlyAfter if !directions.allows_ge() => {
                    *slot = None;
                    live -= 1;
                }
                _ => {}
            }
        }
        // Required demands never drop, so an emptied walk is holes all
        // satisfied: membership holds with the probe unexhausted.
        if live == 0 {
            return true;
        }
        let exhausted =
            walk.probe.done() && walk.sides.iter().flatten().all(|side| side.cursor.done());
        if exhausted {
            break;
        }
        advance_set(&mut walk);
    }

    // Exhaustion: dominations confirm. A required side reaching here kept its
    // direction alive, so it holds; a live inclusive hole means its subtraction
    // held (`le` surviving is `Less` or `Equal`, both subtracted); a strict
    // hole subtracts only the strict relation.
    walk.sides.iter().flatten().all(|side| match side.demand {
        Demand::After | Demand::Before => true,
        Demand::NotBefore | Demand::NotAfter => false,
        Demand::NotStrictlyBefore => side.comparison.relation() != Some(Ordering::Less),
        Demand::NotStrictlyAfter => side.comparison.relation() != Some(Ordering::Greater),
    })
}

/// The membership walk's owned cursor set: the probe's cursor and every
/// bound side — the walk state itself, as in the placement walk's `Cursors`.
///
/// The read loop reaches the sides through the fields between advances, and
/// the whole set steps by the overlay-advance law ([`advance_set`]).
struct MemberCursors<'a> {
    probe: VersionRegionReader<'a>,
    /// The probe's absolute height, shared by every comparison that remains
    /// numerically far from its bound.
    probe_height: Accumulator,
    sides: Vec<Option<BoundSide<'a>>>,
}

impl MemberCursors<'_> {
    /// The probe stream's slot; bound `i` occupies slot `i + 1`.
    const PROBE: usize = 0;
}

/// The membership walk's slot roster.
///
/// Priority `[PROBE, bound 0, bound 1, …]`: the probe steps first on every
/// tie because it is every comparison's first operand and the overlay law's
/// equal-depth arm steps that operand first. The bounds' order only resolves
/// simultaneous crossings; it does not change the intervals observed.
impl CursorSet for MemberCursors<'_> {
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static {
        0..self.sides.len() + 1
    }

    /// A dropped (satisfied-hole) bound reads zero and never steps.
    fn depth(&self, slot: usize) -> u64 {
        match slot {
            Self::PROBE => self.probe.depth(),
            _ => self.sides[slot - 1]
                .as_ref()
                .map_or(0, |side| side.cursor.depth()),
        }
    }

    /// The probe's step folds its crossing into every surviving side's pair as
    /// the `A` operand; a bound's step folds into its own pair as the `B`
    /// operand.
    ///
    /// Every present side's pair reads every interval: membership never
    /// settles a pair — a satisfied hole drops its whole side — so presence
    /// is the only gate.
    fn step(&mut self, slot: usize) -> u64 {
        match slot {
            Self::PROBE => {
                let (flip, step) = self.probe.step();
                self.probe_height.add_bigint(&step);
                for side in self.sides.iter_mut().flatten() {
                    side.comparison
                        .fold(Side::A, &self.probe_height, &side.bound, &step);
                }
                flip
            }
            _ => {
                let side = self.sides[slot - 1]
                    .as_mut()
                    .expect("an absent side reads depth zero and never steps");
                let (flip, step) = side.cursor.step();
                side.bound.add_bigint(&step);
                side.comparison
                    .fold(Side::B, &self.probe_height, &side.bound, &step);
                flip
            }
        }
    }
}

// ────────────────────────────── coverage ──────────────────────────────

/// One bound's side of the coverage walk: its cursor and its two pair
/// comparisons, one against each probe endpoint.
struct SpanSide<'a> {
    cursor: VersionRegionReader<'a>,
    demand: Demand,
    /// The bound's absolute height.
    bound: Accumulator,
    /// The pair against the segment's minimum endpoint.
    lo: GatedComparison,
    /// The pair against the segment's maximum endpoint.
    hi: GatedComparison,
}

/// How much of the segment `[lo, hi]` a query's demands admit, every stream
/// decoded once — `causally`'s [`Coverage`] verdict at the stream layer.
///
/// `lo` and `hi` must satisfy `lo <= hi` (`Span`'s construction
/// contract); the verdict is unspecified otherwise. An empty demand list is
/// [`Coverage::Full`] at zero cost. The demand list's order is the read order
/// per elementary interval; callers supply a deterministic order.
///
/// # Panics
///
/// The canonical-stream contract of [`admits`], on all operands.
pub fn coverage<'a>(
    lo: &'a Version,
    hi: &'a Version,
    bounds: impl IntoIterator<Item = (&'a Version, Demand)>,
) -> Coverage {
    let mut bounds = bounds.into_iter().peekable();
    if bounds.peek().is_none() {
        return Coverage::Full;
    }
    let (lo, lo_first) = VersionRegionReader::open(lo);
    let (hi, hi_first) = VersionRegionReader::open(hi);
    let sides: Vec<Option<SpanSide<'a>>> = bounds
        .map(|(bits, demand)| {
            let (cursor, first) = VersionRegionReader::open(bits);
            Some(SpanSide {
                cursor,
                demand,
                bound: Comparison::absolute_height(&first),
                lo: GatedComparison::new(),
                hi: GatedComparison::new(),
            })
        })
        .collect();
    let mut live = sides.len();
    // Refuted the moment any bound provably misses part of the segment; `Full`
    // needs every bound to survive to exhaustion with its admit-everything
    // relation intact.
    let mut full_possible = true;

    let mut walk = SpanCursors {
        lo,
        lo_height: Comparison::absolute_height(&lo_first),
        lo_live: true,
        hi,
        hi_height: Comparison::absolute_height(&hi_first),
        hi_live: true,
        sides,
    };
    loop {
        for slot in &mut walk.sides {
            let Some(side) = slot else { continue };
            if side.lo.live {
                side.lo
                    .comparison
                    .read(&mut walk.lo_height, &mut side.bound);
            }
            if side.hi.live {
                side.hi
                    .comparison
                    .read(&mut walk.hi_height, &mut side.bound);
            }
            match side.demand {
                // The floor admitting nothing — not even the segment's maximum
                // — is a refutation: the earliest bail, the verdict a pruning
                // walk wants fastest.
                Demand::After => {
                    if !side.hi.comparison.directions.allows_ge() {
                        return Coverage::Empty;
                    }
                    // Admitting everything needs `floor <= lo`; its refutation
                    // settles the lo pair (not Full, not this bound's emptiness
                    // — that reads the hi pair).
                    if side.lo.live && !side.lo.comparison.directions.allows_ge() {
                        full_possible = false;
                        side.lo.live = false;
                    }
                }
                // The ceiling dually: admitting nothing is a refutation on the
                // segment's minimum.
                Demand::Before => {
                    if !side.lo.comparison.directions.allows_le() {
                        return Coverage::Empty;
                    }
                    if side.hi.live && !side.hi.comparison.directions.allows_le() {
                        full_possible = false;
                        side.hi.live = false;
                    }
                }
                // A hole subtracts all of the segment only by covering its
                // maximum (confirmed at exhaustion), and none of it once it
                // provably misses the minimum: missing the minimum is missing
                // everything, since a covered `v` with `v <= bound` would give
                // `lo <= v <= bound`, forcing the refuted `lo <= bound` (the
                // up-set arm below dually, through `hi`). Both pairs settle by
                // refutation, and a hole settled both ways drops its stream.
                //
                // Each arm re-tests only its dominated pair's liveness through
                // the settle order: the segment's endpoints satisfy `lo <= hi`
                // pointwise (`Span`'s construction contract), so a
                // refutation of the dominated endpoint forces the dominating
                // one's at the same interval — checked first, in the same pass
                // — and the joint settle drops the side before any later pass
                // could find the dominated pair settled alone.
                Demand::NotBefore | Demand::NotStrictlyBefore => {
                    if side.hi.live && !side.hi.comparison.directions.allows_le() {
                        side.hi.live = false;
                    }
                    if !side.lo.comparison.directions.allows_le() {
                        side.lo.live = false;
                    }
                }
                Demand::NotAfter | Demand::NotStrictlyAfter => {
                    if side.lo.live && !side.lo.comparison.directions.allows_ge() {
                        side.lo.live = false;
                    }
                    if !side.hi.comparison.directions.allows_ge() {
                        side.hi.live = false;
                    }
                }
            }
            if !side.lo.live && !side.hi.live {
                *slot = None;
                live -= 1;
            }
        }
        // A walk left holding only settled holes is decided: every hole was
        // refuted both ways, so nothing subtracts from the segment and nothing
        // more can change the verdict.
        if live == 0 {
            break;
        }
        // A probe endpoint whose every pair is settled stops being scanned.
        walk.lo_live = walk.lo_live && walk.sides.iter().flatten().any(|side| side.lo.live);
        walk.hi_live = walk.hi_live && walk.sides.iter().flatten().any(|side| side.hi.live);
        debug_assert!(
            walk.sides.iter().flatten().all(|side| match side.demand {
                Demand::After | Demand::Before => true,
                Demand::NotBefore | Demand::NotStrictlyBefore => side.lo.live,
                Demand::NotAfter | Demand::NotStrictlyAfter => side.hi.live,
            }),
            "a surviving hole keeps its dominated endpoint's pair live: settling \
             it forces the dominating pair's settle at the same interval, and the \
             joint settle drops the side"
        );
        let exhausted = (!walk.lo_live || walk.lo.done())
            && (!walk.hi_live || walk.hi.done())
            && walk.sides.iter().flatten().all(|side| side.cursor.done());
        if exhausted {
            break;
        }
        advance_set(&mut walk);
    }

    walk.finish(full_possible)
}

/// The coverage walk's owned cursor set: both probe endpoints' cursors, their
/// live flags, and every bound side — the walk state itself, as in the
/// placement walk's `Cursors`.
///
/// The read loop reaches the sides through the fields between advances, and the
/// whole set steps by the overlay-advance law ([`advance_set`]).
struct SpanCursors<'a> {
    lo: VersionRegionReader<'a>,
    /// The minimum endpoint's absolute height, shared by its bound comparisons.
    lo_height: Accumulator,
    /// Whether any pair still reads the `lo` endpoint; a settled endpoint's
    /// stream is never scanned further.
    lo_live: bool,
    hi: VersionRegionReader<'a>,
    /// The maximum endpoint's absolute height, shared by its bound comparisons.
    hi_height: Accumulator,
    /// Whether any pair still reads the `hi` endpoint, as `lo_live`.
    hi_live: bool,
    sides: Vec<Option<SpanSide<'a>>>,
}

impl SpanCursors<'_> {
    /// The `hi` endpoint's slot.
    const HI: usize = 0;
    /// The `lo` endpoint's slot; bound `i` occupies slot `i + 2`.
    const LO: usize = 1;

    /// Map the exhausted comparisons to the query's coverage verdict.
    ///
    /// A required comparison that settled early has already made full coverage
    /// impossible, so only live comparisons contribute another relation here.
    /// A surviving hole still has the endpoint relation needed to decide
    /// whether it removes all, some, or none of the span.
    fn finish(&self, mut full_possible: bool) -> Coverage {
        for side in self.sides.iter().flatten() {
            // A hole that covers the entire span decides emptiness. Required
            // bounds return Empty at their first refutation during the walk.
            let (lo, hi) = (side.lo.comparison.relation(), side.hi.comparison.relation());
            let empty = match side.demand {
                Demand::After | Demand::Before => false,
                Demand::NotBefore => matches!(hi, Some(Ordering::Less | Ordering::Equal)),
                Demand::NotStrictlyBefore => hi == Some(Ordering::Less),
                Demand::NotAfter => matches!(lo, Some(Ordering::Greater | Ordering::Equal)),
                Demand::NotStrictlyAfter => lo == Some(Ordering::Greater),
            };
            if empty {
                return Coverage::Empty;
            }

            // Full coverage requires this one demand to admit both endpoints.
            let admits_all = match side.demand {
                Demand::After => {
                    !side.lo.live || matches!(lo, Some(Ordering::Greater | Ordering::Equal))
                }
                Demand::Before => {
                    !side.hi.live || matches!(hi, Some(Ordering::Less | Ordering::Equal))
                }
                Demand::NotBefore => !matches!(lo, Some(Ordering::Less | Ordering::Equal)),
                Demand::NotStrictlyBefore => lo != Some(Ordering::Less),
                Demand::NotAfter => !matches!(hi, Some(Ordering::Greater | Ordering::Equal)),
                Demand::NotStrictlyAfter => hi != Some(Ordering::Greater),
            };
            full_possible &= admits_all;
        }
        if full_possible {
            Coverage::Full
        } else {
            Coverage::Partial
        }
    }
}

/// The coverage walk's slot roster.
///
/// Priority `[HI, LO, bound 0, bound 1, …]`: probe endpoints step first on
/// every tie because each is its comparison's first operand and the overlay
/// law's equal-depth arm steps that operand first. `hi` before `lo`, and the
/// bounds' order among themselves, only resolve simultaneous crossings; they
/// do not change the intervals observed.
impl CursorSet for SpanCursors<'_> {
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static {
        0..self.sides.len() + 2
    }

    /// A settled probe endpoint or dropped bound reads zero and never steps.
    fn depth(&self, slot: usize) -> u64 {
        match slot {
            Self::HI => {
                if self.hi_live {
                    self.hi.depth()
                } else {
                    0
                }
            }
            Self::LO => {
                if self.lo_live {
                    self.lo.depth()
                } else {
                    0
                }
            }
            _ => self.sides[slot - 2]
                .as_ref()
                .map_or(0, |side| side.cursor.depth()),
        }
    }

    /// An endpoint's step folds its crossing into its own live pairs as the
    /// `A` operand; a bound's step folds into both its live pairs as the `B`
    /// operand (settled pairs advance unread).
    fn step(&mut self, slot: usize) -> u64 {
        match slot {
            Self::HI => {
                let (flip, step) = self.hi.step();
                self.hi_height.add_bigint(&step);
                for side in self.sides.iter_mut().flatten() {
                    if side.hi.live {
                        side.hi
                            .comparison
                            .fold(Side::A, &self.hi_height, &side.bound, &step);
                    }
                }
                flip
            }
            Self::LO => {
                let (flip, step) = self.lo.step();
                self.lo_height.add_bigint(&step);
                for side in self.sides.iter_mut().flatten() {
                    if side.lo.live {
                        side.lo
                            .comparison
                            .fold(Side::A, &self.lo_height, &side.bound, &step);
                    }
                }
                flip
            }
            _ => {
                let side = self.sides[slot - 2]
                    .as_mut()
                    .expect("an absent side reads depth zero and never steps");
                let (flip, step) = side.cursor.step();
                side.bound.add_bigint(&step);
                if side.lo.live {
                    side.lo
                        .comparison
                        .fold(Side::B, &self.lo_height, &side.bound, &step);
                }
                if side.hi.live {
                    side.hi
                        .comparison
                        .fold(Side::B, &self.hi_height, &side.bound, &step);
                }
                flip
            }
        }
    }
}
