//! The query filter co-walks one batch of bound streams against one or two
//! probe streams.
//!
//! `causally`'s queries hold a floor, a ceiling, and holes — each one bound
//! version with a [`Demand`] on its relation to a probe. Composed from the pair
//! sweep, evaluating a query would decode the probe once per bound. Each batch
//! instead shares one probe traversal and one running probe height. Each bound
//! also keeps its absolute height. A pair materializes their difference only
//! while their numeric widths overlap, and discards it before a later crossing
//! would copy a much wider shared height. The walk advances by the
//! overlay-advance law ([`advance_set`]), and the verdict hooks are branch-only.
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
//! Within a batch, every topology bit is read once and every leaf payload is
//! decoded once. Let `k` be the number of bounds, `i` the number of intervals
//! in the streams' common overlay, `p` the encoded payload bytes in the probe
//! stream or streams, and `n` all encoded input bytes. Topology work is
//! `O(n + k·i)`; numeric work is `O(n + k·p)` because a probe delta may feed
//! every live exact difference. The total is therefore `O(n + k·(i + p))`, or
//! `O(k·n)` using bytes alone.
//!
//! Auxiliary state is `O(n)`: the batch limit bounds the `O(k)` fixed-size
//! records, each absolute height is stored once, and a private difference
//! exists only while its two absolute heights have comparable widths. Its copy
//! is therefore funded by the bound's maximum width over its input stream. A
//! later crossing that separates the widths discards the difference before
//! folding the crossing, so a wide probe value is copied across the batch only
//! when the bounds carry corresponding width. If a wide absolute height's
//! redundant representation prevents a domination decision, it is normalized
//! in shared state rather than copied into every comparison.
//!
//! A difference may be rebuilt after the widths converge again. Reaching that
//! state requires a width-changing input crossing or normalization of a
//! redundant absolute height. The crossing's payload or the folds that created
//! the redundant width pay for that work. Thus rebuilding does not introduce
//! an uncharged width factor into either bound above.
//!
//! Compared with composing binary sweeps, the fused walk avoids decoding the
//! probe once per bound; it does not eliminate the work of evaluating `k`
//! independent demands at each interval.

use core::cmp::Ordering;

use num_bigint::BigUint;
use suanpan::Accumulator;

use crate::causally::Coverage;
use crate::codec::{accumulator, BitsView};

use super::super::overlay::{advance_set, CursorSet, LeafCursor, PlateauCursor, Side};
use super::super::sweep::Directions;

/// Heap reserved for live side records per encoded input byte.
///
/// Cursor paths and spilled accumulators use additional storage in proportion
/// to the topology and numeric payloads that fund them. Limiting the fixed
/// records separately prevents many tiny bounds from multiplying their
/// resident encoding into a much larger transient roster. Eight is one third
/// of the crate's enforced 24 B/B transient ceiling, leaving most of the budget
/// for those input-proportionate allocations and allocator rounding.
const SIDE_BYTES_PER_INPUT_BYTE: usize = 8;

/// How many side records fit within one walk's input-funded budget.
fn side_capacity<State>(input_bytes: usize) -> usize {
    (input_bytes.saturating_mul(SIDE_BYTES_PER_INPUT_BYTE) / std::mem::size_of::<State>()).max(1)
}

/// What a query demands of the relation between the probe and one bound stream,
/// in the probe-first orientation (`le` is `probe <= bound`).
///
/// The first two are *required* relations (a floor and a ceiling — both
/// inclusive, `causally`'s normal form): refuting them refutes membership. The
/// rest are *excluded* relations, the four hole kinds: membership survives
/// exactly when the named relation fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Demand {
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
/// While the heights are far apart, `difference` stays absent and the pair
/// reads their shared absolute accumulators through a domination certificate.
/// While their widths overlap, it stores `probe − bound` and receives both
/// streams' later deltas directly. A later crossing that separates the widths
/// returns to shared comparison before that crossing is folded.
///
/// Settlement — a comparison whose verdict contribution is fixed mid-walk —
/// belongs to the coverage walk ([`GatedComparison`]); the membership walk
/// never settles a comparison, so this type does not carry that state.
struct Comparison {
    difference: Option<Accumulator>,
    directions: Directions,
}

impl Comparison {
    /// A comparison before either stream has been read.
    fn new() -> Comparison {
        Comparison {
            difference: None,
            directions: Directions::new(),
        }
    }

    /// Fold this interval's height relation into the surviving directions.
    fn read(&mut self, probe: &mut Accumulator, bound: &mut Accumulator) {
        let sign = match &mut self.difference {
            Some(difference) => difference.sign(),
            None => {
                let (sign, difference) = compare_heights(probe, bound);
                self.difference = difference;
                sign
            }
        };
        self.directions.fold(sign);
    }

    /// Fold one crossing unless it separates the shared heights.
    fn fold(
        &mut self,
        side: Side,
        probe: &Accumulator,
        bound: &Accumulator,
        step: &super::super::overlay::Step,
    ) {
        if self.difference.is_some() && !widths_overlap(probe, bound) {
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

/// Put an absolute skyline height into the accumulator representation.
fn height(first: &BigUint) -> Accumulator {
    let mut height = Accumulator::new();
    accumulator::fold(&mut height, first, 0, false);
    height
}

/// Whether neither height is three accumulator digits wider than the other.
fn widths_overlap(a: &Accumulator, b: &Accumulator) -> bool {
    a.digit_count().abs_diff(b.digit_count()) < 3
}

/// Replace a redundant absolute-height spelling with its normalized value.
fn normalize_height(value: &mut Accumulator) {
    let (sign, magnitude) = accumulator::value(value);
    debug_assert_ne!(sign, Ordering::Less, "skyline heights are nonnegative");
    *value = height(&magnitude);
}

/// Compare two nonnegative heights without copying a much wider operand.
///
/// A three-digit width lead may certify the answer from the larger value's top
/// digits. If a redundant spelling prevents that decision, normalizing the
/// shared absolute height either makes the widths overlap or proves that its
/// magnitude is larger: three digits of separation exceed the accumulator's
/// 33-bit representation overhang. An exact private difference is therefore
/// built only from comparably wide operands. It copies live digits, not spare
/// capacity retained by either source.
fn compare_heights(
    probe: &mut Accumulator,
    bound: &mut Accumulator,
) -> (Ordering, Option<Accumulator>) {
    let mut probe_normalized = false;
    let mut bound_normalized = false;
    loop {
        if probe.digit_count() >= bound.digit_count().saturating_add(3) {
            if probe_normalized {
                return (Ordering::Greater, None);
            }
            let (sign, decided) = probe.sign_dominates_at(bound.digit_count() - 1);
            if decided {
                debug_assert_eq!(sign, Ordering::Greater, "skyline heights are nonnegative");
                return (Ordering::Greater, None);
            }
            normalize_height(probe);
            probe_normalized = true;
            continue;
        }
        if bound.digit_count() >= probe.digit_count().saturating_add(3) {
            if bound_normalized {
                return (Ordering::Less, None);
            }
            let (sign, decided) = bound.sign_dominates_at(probe.digit_count() - 1);
            if decided {
                debug_assert_eq!(sign, Ordering::Greater, "skyline heights are nonnegative");
                return (Ordering::Less, None);
            }
            normalize_height(bound);
            bound_normalized = true;
            continue;
        }
        break;
    }

    debug_assert!(widths_overlap(probe, bound));
    let mut difference = Accumulator::new();
    if probe.digit_count() <= bound.digit_count() {
        difference.add_accum(probe);
        difference.sub_accum(bound);
    } else {
        difference.add_accum(bound);
        difference.sub_accum(probe);
        difference.negate();
    }
    let sign = difference.sign();
    (sign, Some(difference))
}

// ───────────────────────────── membership ─────────────────────────────

/// One bound's side of the membership walk.
struct BoundSide<'a> {
    cursor: LeafCursor<'a>,
    /// The bound's absolute height.
    bound: Accumulator,
    comparison: Comparison,
    demand: Demand,
}

/// The maximum bounds in one membership walk over `input_bytes` of operands.
pub(crate) fn membership_capacity(input_bytes: usize) -> usize {
    side_capacity::<Option<BoundSide<'static>>>(input_bytes)
}

/// Whether the probe stream's version satisfies every demand, each stream
/// decoded once — `causally`'s membership predicate at the stream layer.
///
/// An empty demand list is vacuously `true` at zero cost. Demands are read in
/// their supplied order on each interval, so the first decisive one exits.
///
/// # Panics
///
/// Operands must be canonical skyline streams — the placement walk's contract
/// exactly: the violations the walk structurally notices panic, the rest sweep
/// silently with an unspecified verdict.
pub(crate) fn admits<'a>(
    probe: BitsView<'a>,
    bounds: impl IntoIterator<Item = (BitsView<'a>, Demand)>,
) -> bool {
    let mut bounds = bounds.into_iter().peekable();
    if bounds.peek().is_none() {
        return true;
    }
    let (probe, probe_first) = LeafCursor::open(probe);
    let sides: Vec<Option<BoundSide<'a>>> = bounds
        .map(|(bits, demand)| {
            let (cursor, first) = LeafCursor::open(bits);
            Some(BoundSide {
                cursor,
                bound: height(&first),
                comparison: Comparison::new(),
                demand,
            })
        })
        .collect();
    let mut live = sides.len();
    let mut walk = MemberCursors {
        probe,
        probe_height: height(&probe_first),
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
                Demand::After if !directions.ge => return false,
                Demand::Before if !directions.le => return false,
                // A hole's subtracting direction refuted satisfies the hole:
                // drop its cursor, its stream is never scanned further.
                Demand::NotBefore | Demand::NotStrictlyBefore if !directions.le => {
                    *slot = None;
                    live -= 1;
                }
                Demand::NotAfter | Demand::NotStrictlyAfter if !directions.ge => {
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
    probe: LeafCursor<'a>,
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
                accumulator::fold_signed(&mut self.probe_height, &step);
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
                accumulator::fold_signed(&mut side.bound, &step);
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
    cursor: LeafCursor<'a>,
    demand: Demand,
    /// The bound's absolute height.
    bound: Accumulator,
    /// The pair against the segment's minimum endpoint.
    lo: GatedComparison,
    /// The pair against the segment's maximum endpoint.
    hi: GatedComparison,
}

/// The maximum bounds in one coverage walk over `input_bytes` of operands.
pub(crate) fn coverage_capacity(input_bytes: usize) -> usize {
    side_capacity::<Option<SpanSide<'static>>>(input_bytes)
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
pub(crate) fn coverage<'a>(
    lo: BitsView<'a>,
    hi: BitsView<'a>,
    bounds: impl IntoIterator<Item = (BitsView<'a>, Demand)>,
) -> Coverage {
    let mut bounds = bounds.into_iter().peekable();
    if bounds.peek().is_none() {
        return Coverage::Full;
    }
    let (lo, lo_first) = LeafCursor::open(lo);
    let (hi, hi_first) = LeafCursor::open(hi);
    let sides: Vec<Option<SpanSide<'a>>> = bounds
        .map(|(bits, demand)| {
            let (cursor, first) = LeafCursor::open(bits);
            Some(SpanSide {
                cursor,
                demand,
                bound: height(&first),
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
        lo_height: height(&lo_first),
        lo_live: true,
        hi,
        hi_height: height(&hi_first),
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
                    if !side.hi.comparison.directions.ge {
                        return Coverage::Empty;
                    }
                    // Admitting everything needs `floor <= lo`; its refutation
                    // settles the lo pair (not Full, not this bound's emptiness
                    // — that reads the hi pair).
                    if side.lo.live && !side.lo.comparison.directions.ge {
                        full_possible = false;
                        side.lo.live = false;
                    }
                }
                // The ceiling dually: admitting nothing is a refutation on the
                // segment's minimum.
                Demand::Before => {
                    if !side.lo.comparison.directions.le {
                        return Coverage::Empty;
                    }
                    if side.hi.live && !side.hi.comparison.directions.le {
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
                    if side.hi.live && !side.hi.comparison.directions.le {
                        side.hi.live = false;
                    }
                    if !side.lo.comparison.directions.le {
                        side.lo.live = false;
                    }
                }
                Demand::NotAfter | Demand::NotStrictlyAfter => {
                    if side.lo.live && !side.lo.comparison.directions.ge {
                        side.lo.live = false;
                    }
                    if !side.hi.comparison.directions.ge {
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

    finish(&walk.sides, full_possible)
}

/// Map the exhausted coverage walk's decided relations to the verdict.
///
/// Division of labor with the walk: a settled *required* pair's refutation
/// already lives in `full_possible`, so the `!live` guards on the two
/// required arms keep `finish` from consulting a settled pair's stale
/// `directions`. The stale directions would happen to agree (a
/// settle-direction refutation is permanent), but that agreement is not part
/// of the contract: only a pair alive at exhaustion answers by its decided
/// relation. A hole needs no guard on its fullness endpoint: a surviving
/// hole's dominated pair — `lo` for a down-set, `hi` for an up-set — is
/// always alive here, because settling it forces the dominating pair's
/// settle at the same interval and the joint settle drops the side (the walk
/// arms' settle-order argument; the walk's per-interval assert keeps it
/// loud).
fn finish(sides: &[Option<SpanSide<'_>>], mut full_possible: bool) -> Coverage {
    for side in sides.iter().flatten() {
        // Emptiness first: any bound whose subtraction covers the whole segment
        // (or whose requirement admits none of it — returned inline during the
        // walk) empties the verdict.
        let (lo, hi) = (side.lo.comparison.relation(), side.hi.comparison.relation());
        let empty = match side.demand {
            // Their emptying refutations returned inline.
            Demand::After | Demand::Before => false,
            // The subtracted down-set covers the maximum…
            Demand::NotBefore => matches!(hi, Some(Ordering::Less | Ordering::Equal)),
            Demand::NotStrictlyBefore => hi == Some(Ordering::Less),
            // …or the subtracted up-set reaches the minimum.
            Demand::NotAfter => matches!(lo, Some(Ordering::Greater | Ordering::Equal)),
            Demand::NotStrictlyAfter => lo == Some(Ordering::Greater),
        };
        if empty {
            return Coverage::Empty;
        }
        // Fullness: the bound must admit everything the segment covers
        // (required demands inclusively: `After` admits equality).
        let admits_all = match side.demand {
            Demand::After => {
                !side.lo.live || matches!(lo, Some(Ordering::Greater | Ordering::Equal))
            }
            Demand::Before => !side.hi.live || matches!(hi, Some(Ordering::Less | Ordering::Equal)),
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

/// The coverage walk's owned cursor set: both probe endpoints' cursors, their
/// live flags, and every bound side — the walk state itself, as in the
/// placement walk's `Cursors`.
///
/// The read loop reaches the sides through the fields between advances, and the
/// whole set steps by the overlay-advance law ([`advance_set`]).
struct SpanCursors<'a> {
    lo: LeafCursor<'a>,
    /// The minimum endpoint's absolute height, shared by its bound comparisons.
    lo_height: Accumulator,
    /// Whether any pair still reads the `lo` endpoint; a settled endpoint's
    /// stream is never scanned further.
    lo_live: bool,
    hi: LeafCursor<'a>,
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
                accumulator::fold_signed(&mut self.hi_height, &step);
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
                accumulator::fold_signed(&mut self.lo_height, &step);
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
                accumulator::fold_signed(&mut side.bound, &step);
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
