//! The query filter co-walks: one or two probe streams against any number of
//! bound streams, in a single fused merge, each stream decoded once.
//!
//! `causally`'s queries hold a floor, a ceiling, and holes — each one bound
//! version with a [`Demand`] on its relation to a probe. Composed from the pair
//! sweep, evaluating a query would decode the probe once per bound; these walks
//! decode every stream exactly once, maintaining one running difference per
//! (probe, bound) pair, in the placement walk's idiom ([`place`](super)): the
//! overlay advance restates the generic binary law at this arity, each pair's
//! accumulator sees exactly the write sequence its pair sweep would commit, and
//! the verdict hooks are branch-only.
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
//! Derived, by the placement walk's argument stream by stream: every topology
//! bit of every stream read at most once, every leaf payload decoded once and
//! folded into at most one accumulator per pair it participates in — the
//! probe's deltas into each live bound's pair, a bound's deltas into its own —
//! and the per-interval sign reads ride the accumulator's amortized-O(1)
//! collapse. `O(|v| + Σ|bound|)` for membership, `O(|lo| + |hi| + Σ|bound|)`
//! for coverage, against the composed sweeps' one probe decode per bound.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::causally::Coverage;
use crate::codec::{BitsSlice, Int};

use super::super::sweep::{fold, Directions, LeafCursor, PlateauCursor, Side, Step};

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

/// One (probe, bound) pair's running comparison: the difference `height_probe −
/// height_bound` on the cliff-immune accumulator, and the pair's surviving
/// directions.
struct Pair {
    diff: Accumulator,
    dirs: Directions,
    /// Whether the pair still feeds a verdict: a settled pair stops folding and
    /// reading (its stream may still advance for the other pair riding the same
    /// cursor).
    live: bool,
}

impl Pair {
    /// Seed the pair from the two streams' absolute first heights.
    fn open(probe_first: &Int, bound_first: &Int) -> Pair {
        let mut diff = Accumulator::new();
        super::super::fold_signed_int(&mut diff, false, probe_first);
        super::super::fold_signed_int(&mut diff, true, bound_first);
        Pair {
            diff,
            dirs: Directions::new(),
            live: true,
        }
    }

    /// Fold this interval's sign into the surviving directions.
    fn read(&mut self) {
        self.dirs.fold(self.diff.sign());
    }

    /// The relation the completed sweep decided, as the causal order.
    fn relation(&self) -> Option<Ordering> {
        self.dirs.relation()
    }
}

// ───────────────────────────── membership ─────────────────────────────

/// One bound's side of the membership walk.
struct BoundSide<'a> {
    cursor: LeafCursor<'a>,
    pair: Pair,
    demand: Demand,
}

/// Whether the probe stream's version satisfies every demand, each stream
/// decoded once — `causally`'s membership predicate at the stream layer.
///
/// An empty demand list is vacuously `true` at zero cost. The demand list's
/// order is the read order per elementary interval, which fixes the accumulator
/// write sequence; callers supply a deterministic order.
///
/// # Panics
///
/// Operands must be canonical skyline streams — the placement walk's contract
/// exactly: the violations the walk structurally notices panic, the rest sweep
/// silently with an unspecified verdict.
pub(crate) fn admits<'a>(
    probe: &'a BitsSlice,
    bounds: impl IntoIterator<Item = (&'a BitsSlice, Demand)>,
) -> bool {
    let mut bounds = bounds.into_iter().peekable();
    if bounds.peek().is_none() {
        return true;
    }
    let (mut probe, probe_first) = LeafCursor::open(probe);
    let mut sides: Vec<Option<BoundSide<'a>>> = bounds
        .map(|(bits, demand)| {
            let (cursor, first) = LeafCursor::open(bits);
            Some(BoundSide {
                cursor,
                pair: Pair::open(&probe_first, &first),
                demand,
            })
        })
        .collect();
    let mut live = sides.len();

    loop {
        // One read per live bound per elementary interval, in demand order.
        for slot in &mut sides {
            let Some(side) = slot else { continue };
            side.pair.read();
            let dirs = side.pair.dirs;
            match side.demand {
                // A required direction refuted refutes membership: the walk's
                // earliest bail.
                Demand::After if !dirs.ge => return false,
                Demand::Before if !dirs.le => return false,
                // A hole's subtracting direction refuted satisfies the hole:
                // drop its cursor, its stream is never scanned further.
                Demand::NotBefore | Demand::NotStrictlyBefore if !dirs.le => {
                    *slot = None;
                    live -= 1;
                }
                Demand::NotAfter | Demand::NotStrictlyAfter if !dirs.ge => {
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
        let exhausted = probe.done() && sides.iter().flatten().all(|side| side.cursor.done());
        if exhausted {
            break;
        }
        advance(&mut probe, &mut sides);
    }

    // Exhaustion: dominations confirm. A required side reaching here kept its
    // direction alive, so it holds; a live inclusive hole means its subtraction
    // held (`le` surviving is `Less` or `Equal`, both subtracted); a strict
    // hole subtracts only the strict relation.
    sides.iter().flatten().all(|side| match side.demand {
        Demand::After | Demand::Before => true,
        Demand::NotBefore | Demand::NotAfter => false,
        Demand::NotStrictlyBefore => side.pair.relation() != Some(Ordering::Less),
        Demand::NotStrictlyAfter => side.pair.relation() != Some(Ordering::Greater),
    })
}

/// Advance the membership overlay one boundary.
///
/// The deepest cursor steps, and every other cursor whose depth reaches the
/// flip level steps in the same round — the placement walk's advance restated
/// at this arity, the probe winning depth ties (it is every pair's first
/// operand, so it steps first).
fn advance<'a>(probe: &mut LeafCursor<'a>, sides: &mut [Option<BoundSide<'a>>]) {
    fn step_bound(side: &mut BoundSide<'_>) -> usize {
        let (flip, step) = side.cursor.step();
        if side.pair.live {
            fold(&mut side.pair.diff, Side::B, step.negative, &step.magnitude);
        }
        flip
    }
    fn fold_probe<'a>(step: &Step, sides: &mut [Option<BoundSide<'a>>]) {
        for side in sides.iter_mut().flatten() {
            if side.pair.live {
                fold(&mut side.pair.diff, Side::A, step.negative, &step.magnitude);
            }
        }
    }

    let deepest_bound = sides
        .iter()
        .enumerate()
        .filter_map(|(slot, side)| side.as_ref().map(|side| (slot, side.cursor.depth())))
        .max_by_key(|&(_, depth)| depth);
    let (flip, stepped) = match deepest_bound {
        Some((slot, depth)) if probe.depth() < depth => {
            let side = sides[slot].as_mut().expect("the deepest slot is live");
            (step_bound(side), Some(slot))
        }
        _ => {
            let (flip, step) = probe.step();
            fold_probe(&step, sides);
            (flip, None)
        }
    };
    if stepped.is_some() && probe.depth() >= flip {
        let (tied, step) = probe.step();
        debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
        fold_probe(&step, sides);
    }
    for (slot, side) in sides.iter_mut().enumerate() {
        if stepped == Some(slot) {
            continue;
        }
        let Some(side) = side else {
            continue;
        };
        if side.cursor.depth() >= flip {
            let tied = step_bound(side);
            debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
        }
    }
}

// ────────────────────────────── coverage ──────────────────────────────

/// One bound's side of the coverage walk: its cursor and its two pair
/// comparisons, one against each probe endpoint.
struct SpanSide<'a> {
    cursor: LeafCursor<'a>,
    demand: Demand,
    /// The pair against the segment's minimum endpoint.
    lo: Pair,
    /// The pair against the segment's maximum endpoint.
    hi: Pair,
}

/// How much of the segment `[lo, hi]` a query's demands admit, every stream
/// decoded once — `causally`'s [`Coverage`] verdict at the stream layer.
///
/// `lo` and `hi` must satisfy `lo <= hi` (`causally::Span`'s construction
/// contract); the verdict is unspecified otherwise. An empty demand list is
/// [`Coverage::Full`] at zero cost. The demand list's order is the read order
/// per elementary interval; callers supply a deterministic order.
///
/// # Panics
///
/// The canonical-stream contract of [`admits`], on all operands.
pub(crate) fn coverage<'a>(
    lo: &'a BitsSlice,
    hi: &'a BitsSlice,
    bounds: impl IntoIterator<Item = (&'a BitsSlice, Demand)>,
) -> Coverage {
    let mut bounds = bounds.into_iter().peekable();
    if bounds.peek().is_none() {
        return Coverage::Full;
    }
    let (mut lo, lo_first) = LeafCursor::open(lo);
    let (mut hi, hi_first) = LeafCursor::open(hi);
    let mut sides: Vec<Option<SpanSide<'a>>> = bounds
        .map(|(bits, demand)| {
            let (cursor, first) = LeafCursor::open(bits);
            Some(SpanSide {
                cursor,
                demand,
                lo: Pair::open(&lo_first, &first),
                hi: Pair::open(&hi_first, &first),
            })
        })
        .collect();
    let mut live = sides.len();
    // Refuted the moment any bound provably misses part of the segment; `Full`
    // needs every bound to survive to exhaustion with its admit-everything
    // relation intact.
    let mut full_possible = true;

    let (mut lo_live, mut hi_live) = (true, true);
    loop {
        for slot in &mut sides {
            let Some(side) = slot else { continue };
            if side.lo.live {
                side.lo.read();
            }
            if side.hi.live {
                side.hi.read();
            }
            match side.demand {
                // The floor admitting nothing — not even the segment's maximum
                // — is a refutation: the earliest bail, the verdict a pruning
                // walk wants fastest.
                Demand::After => {
                    if !side.hi.dirs.ge {
                        return Coverage::Empty;
                    }
                    // Admitting everything needs `floor <= lo`; its refutation
                    // settles the lo pair (not Full, not this bound's emptiness
                    // — that reads the hi pair).
                    if side.lo.live && !side.lo.dirs.ge {
                        full_possible = false;
                        side.lo.live = false;
                    }
                }
                // The ceiling dually: admitting nothing is a refutation on the
                // segment's minimum.
                Demand::Before => {
                    if !side.lo.dirs.le {
                        return Coverage::Empty;
                    }
                    if side.hi.live && !side.hi.dirs.le {
                        full_possible = false;
                        side.hi.live = false;
                    }
                }
                // A hole subtracts all of the segment only by covering its
                // maximum (confirmed at exhaustion), and none of it once it
                // provably misses the minimum — both pairs settle by
                // refutation, and a hole settled both ways drops its stream.
                Demand::NotBefore | Demand::NotStrictlyBefore => {
                    if side.hi.live && !side.hi.dirs.le {
                        side.hi.live = false;
                    }
                    if side.lo.live && !side.lo.dirs.le {
                        side.lo.live = false;
                    }
                }
                Demand::NotAfter | Demand::NotStrictlyAfter => {
                    if side.lo.live && !side.lo.dirs.ge {
                        side.lo.live = false;
                    }
                    if side.hi.live && !side.hi.dirs.ge {
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
        lo_live = lo_live && sides.iter().flatten().any(|side| side.lo.live);
        hi_live = hi_live && sides.iter().flatten().any(|side| side.hi.live);
        let exhausted = (!lo_live || lo.done())
            && (!hi_live || hi.done())
            && sides.iter().flatten().all(|side| side.cursor.done());
        if exhausted {
            break;
        }
        advance_span((&mut lo, lo_live), (&mut hi, hi_live), &mut sides);
    }

    finish(&sides, full_possible)
}

/// Map the exhausted coverage walk's decided relations to the verdict.
fn finish(sides: &[Option<SpanSide<'_>>], mut full_possible: bool) -> Coverage {
    for side in sides.iter().flatten() {
        // Emptiness first: any bound whose subtraction covers the whole segment
        // (or whose requirement admits none of it — returned inline during the
        // walk) empties the verdict.
        let (lo, hi) = (side.lo.relation(), side.hi.relation());
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
        // Fullness: the bound must admit everything the segment covers. A
        // settled pair already recorded its refutation in `full_possible`; a
        // pair alive at exhaustion answers by its decided relation.
        let admits_all = match side.demand {
            Demand::After => {
                !side.lo.live || matches!(lo, Some(Ordering::Greater | Ordering::Equal))
            }
            Demand::Before => !side.hi.live || matches!(hi, Some(Ordering::Less | Ordering::Equal)),
            Demand::NotBefore => {
                !side.lo.live || !matches!(lo, Some(Ordering::Less | Ordering::Equal))
            }
            Demand::NotStrictlyBefore => !side.lo.live || lo != Some(Ordering::Less),
            Demand::NotAfter => {
                !side.hi.live || !matches!(hi, Some(Ordering::Greater | Ordering::Equal))
            }
            Demand::NotStrictlyAfter => !side.hi.live || hi != Some(Ordering::Greater),
        };
        full_possible &= admits_all;
    }
    if full_possible {
        Coverage::Full
    } else {
        Coverage::Partial
    }
}

/// Advance the coverage overlay one boundary: [`advance`]'s law with two probe
/// endpoints.
///
/// The deepest cursor steps, tied cursors step in the same round, probe
/// endpoints win depth ties over bounds (each is its pairs' first operand), and
/// `hi` wins a tie with `lo` (deterministically; the two never share a pair, so
/// the order moves no accumulator reading).
fn advance_span<'a>(
    (lo, lo_live): (&mut LeafCursor<'a>, bool),
    (hi, hi_live): (&mut LeafCursor<'a>, bool),
    sides: &mut [Option<SpanSide<'a>>],
) {
    fn step_bound(side: &mut SpanSide<'_>) -> usize {
        let (flip, step) = side.cursor.step();
        for pair in [&mut side.lo, &mut side.hi] {
            if pair.live {
                fold(&mut pair.diff, Side::B, step.negative, &step.magnitude);
            }
        }
        flip
    }
    fn fold_lo<'a>(step: &Step, sides: &mut [Option<SpanSide<'a>>]) {
        for side in sides.iter_mut().flatten() {
            if side.lo.live {
                fold(&mut side.lo.diff, Side::A, step.negative, &step.magnitude);
            }
        }
    }
    fn fold_hi<'a>(step: &Step, sides: &mut [Option<SpanSide<'a>>]) {
        for side in sides.iter_mut().flatten() {
            if side.hi.live {
                fold(&mut side.hi.diff, Side::A, step.negative, &step.magnitude);
            }
        }
    }

    let bound_depth = sides
        .iter()
        .flatten()
        .map(|side| side.cursor.depth())
        .max()
        .unwrap_or(0);
    let lo_depth = if lo_live { lo.depth() } else { 0 };
    let hi_depth = if hi_live { hi.depth() } else { 0 };

    // Deepest wins; probes beat bounds on ties, `hi` beats `lo`.
    enum Deepest {
        Hi,
        Lo,
        Bound,
    }
    let deepest = if hi_live && hi_depth >= lo_depth && hi_depth >= bound_depth {
        Deepest::Hi
    } else if lo_live && lo_depth >= bound_depth {
        Deepest::Lo
    } else {
        Deepest::Bound
    };
    let (flip, stepped_bound) = match deepest {
        Deepest::Hi => {
            let (flip, step) = hi.step();
            fold_hi(&step, sides);
            (flip, None)
        }
        Deepest::Lo => {
            let (flip, step) = lo.step();
            fold_lo(&step, sides);
            (flip, None)
        }
        Deepest::Bound => {
            let slot = sides
                .iter()
                .enumerate()
                .filter_map(|(slot, side)| side.as_ref().map(|side| (slot, side.cursor.depth())))
                .max_by_key(|&(_, depth)| depth)
                .map(|(slot, _)| slot)
                .expect("a bound is deepest only when one is live");
            let side = sides[slot].as_mut().expect("the deepest slot is live");
            (step_bound(side), Some(slot))
        }
    };
    // Tied cursors close to the shared flip level, probes first.
    if !matches!(deepest, Deepest::Hi) && hi_live && hi.depth() >= flip {
        let (tied, step) = hi.step();
        debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
        fold_hi(&step, sides);
    }
    if !matches!(deepest, Deepest::Lo) && lo_live && lo.depth() >= flip {
        let (tied, step) = lo.step();
        debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
        fold_lo(&step, sides);
    }
    for (slot, side) in sides.iter_mut().enumerate() {
        if stepped_bound == Some(slot) {
            continue;
        }
        let Some(side) = side else {
            continue;
        };
        if side.cursor.depth() >= flip {
            let tied = step_bound(side);
            debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
        }
    }
}
