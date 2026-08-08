//! The query filter co-walks: one or two probe streams against any number of
//! bound streams, in a single fused merge, each stream decoded once.
//!
//! `causally`'s queries hold a floor, a ceiling, and holes — each one bound
//! version with a [`Demand`] on its relation to a probe. Composed from the pair
//! sweep, evaluating a query would decode the probe once per bound; these walks
//! decode every stream exactly once, maintaining one running difference per
//! (probe, bound) pair, in the placement walk's idiom ([`place`](super)): each
//! walk advances by the overlay-advance law ([`advance_set`]), contributing
//! only its slot roster and what each slot's step folds; each pair's
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
//! collapse. The coverage walk additionally recomputes its endpoint-liveness
//! flags and sweeps the settled flags once per interval — O(#bounds)
//! bookkeeping absorbed by the same per-interval read loop. `O(|v| + Σ|bound|)`
//! for membership, `O(|lo| + |hi| + Σ|bound|)` for coverage, against the
//! composed sweeps' one probe decode per bound.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::causally::Coverage;
use crate::codec::{BitsSlice, Int};

use super::super::overlay::{advance_set, fold, CursorSet, LeafCursor, PlateauCursor, Side};
use super::super::signed::Sign;
use super::super::sweep::Directions;

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
    directions: Directions,
    /// Whether the pair still feeds a verdict.
    ///
    /// A pair is *settled* — `live == false` — when a refutation fixed
    /// everything the verdict will ever need from it: a settled pair stops
    /// folding and reading (its stream may still advance for the other pair
    /// riding the same cursor).
    live: bool,
}

impl Pair {
    /// Seed the pair from the two streams' absolute first heights.
    fn open(probe_first: &Int, bound_first: &Int) -> Pair {
        let mut diff = Accumulator::new();
        super::super::signed::fold_signed_int(&mut diff, Sign::Positive, probe_first);
        super::super::signed::fold_signed_int(&mut diff, Sign::Negative, bound_first);
        Pair {
            diff,
            directions: Directions::new(),
            live: true,
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
    let (probe, probe_first) = LeafCursor::open(probe);
    let sides: Vec<Option<BoundSide<'a>>> = bounds
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
    let mut walk = MemberCursors { probe, sides };

    loop {
        // One read per live bound per elementary interval, in demand order.
        for slot in &mut walk.sides {
            let Some(side) = slot else { continue };
            side.pair.read();
            let directions = side.pair.directions;
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
        Demand::NotStrictlyBefore => side.pair.relation() != Some(Ordering::Less),
        Demand::NotStrictlyAfter => side.pair.relation() != Some(Ordering::Greater),
    })
}

/// The membership walk's owned cursor set: the probe's cursor and every
/// bound side — the walk state itself, as in the placement walk's `Cursors`.
///
/// The read loop reaches the sides through the fields between advances, and
/// the whole set steps by the overlay-advance law ([`advance_set`]).
struct MemberCursors<'a> {
    probe: LeafCursor<'a>,
    sides: Vec<Option<BoundSide<'a>>>,
}

impl MemberCursors<'_> {
    /// The probe stream's slot; bound `i` occupies slot `i + 1`.
    const PROBE: usize = 0;
}

/// The membership walk's slot roster.
///
/// Priority `[PROBE, bound 0, bound 1, …]`: the probe steps first on every
/// tie — it is every pair's first operand, and the binary law's equal-depth
/// arm steps its first operand first — which is what keeps each pair's
/// accumulator write sequence identical to its pair sweep's (the placement
/// identity rows in `tests/meter.rs` pin the single-bound identity). The
/// bounds' order among themselves moves no committed reading: no two bounds
/// share an accumulator.
impl CursorSet for MemberCursors<'_> {
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static {
        0..self.sides.len() + 1
    }

    /// A dropped (satisfied-hole) bound reads zero and never steps.
    fn depth(&self, slot: usize) -> usize {
        match slot {
            Self::PROBE => self.probe.depth(),
            _ => self.sides[slot - 1]
                .as_ref()
                .map_or(0, |side| side.cursor.depth()),
        }
    }

    /// The probe's step folds its crossing into every live pair as the `A`
    /// operand; a bound's step folds into its own pair as the `B` operand
    /// (skipped for a settled pair, whose stream advances unread).
    fn step(&mut self, slot: usize) -> usize {
        match slot {
            Self::PROBE => {
                let (flip, step) = self.probe.step();
                for side in self.sides.iter_mut().flatten() {
                    if side.pair.live {
                        fold(&mut side.pair.diff, Side::A, step.sign, &step.magnitude);
                    }
                }
                flip
            }
            _ => {
                let side = self.sides[slot - 1]
                    .as_mut()
                    .expect("an absent side reads depth zero and never steps");
                let (flip, step) = side.cursor.step();
                if side.pair.live {
                    fold(&mut side.pair.diff, Side::B, step.sign, &step.magnitude);
                }
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
    let (lo, lo_first) = LeafCursor::open(lo);
    let (hi, hi_first) = LeafCursor::open(hi);
    let sides: Vec<Option<SpanSide<'a>>> = bounds
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

    let mut walk = SpanCursors {
        lo,
        lo_live: true,
        hi,
        hi_live: true,
        sides,
    };
    loop {
        for slot in &mut walk.sides {
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
                    if !side.hi.directions.ge {
                        return Coverage::Empty;
                    }
                    // Admitting everything needs `floor <= lo`; its refutation
                    // settles the lo pair (not Full, not this bound's emptiness
                    // — that reads the hi pair).
                    if side.lo.live && !side.lo.directions.ge {
                        full_possible = false;
                        side.lo.live = false;
                    }
                }
                // The ceiling dually: admitting nothing is a refutation on the
                // segment's minimum.
                Demand::Before => {
                    if !side.lo.directions.le {
                        return Coverage::Empty;
                    }
                    if side.hi.live && !side.hi.directions.le {
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
                Demand::NotBefore | Demand::NotStrictlyBefore => {
                    if side.hi.live && !side.hi.directions.le {
                        side.hi.live = false;
                    }
                    if side.lo.live && !side.lo.directions.le {
                        side.lo.live = false;
                    }
                }
                Demand::NotAfter | Demand::NotStrictlyAfter => {
                    if side.lo.live && !side.lo.directions.ge {
                        side.lo.live = false;
                    }
                    if side.hi.live && !side.hi.directions.ge {
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
/// Division of labor with the walk: a settled pair's refutation already lives
/// where the verdict needs it — a required demand's in `full_possible`, a
/// hole's as the favorable answer itself — so the `!live` guards below keep
/// `finish` from consulting a settled pair's stale `directions`. The stale
/// directions would happen to agree (a settle-direction refutation is
/// permanent), but that agreement is not part of the contract: only a pair
/// alive at exhaustion answers by its decided relation.
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
        // Fullness: the bound must admit everything the segment covers
        // (required demands inclusively: `After` admits equality). A settled
        // pair already recorded its refutation in `full_possible`; a pair alive
        // at exhaustion answers by its decided relation.
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

/// The coverage walk's owned cursor set: both probe endpoints' cursors, their
/// live flags, and every bound side — the walk state itself, as in the
/// placement walk's `Cursors`.
///
/// The read loop reaches the sides through the fields between advances, and the
/// whole set steps by the overlay-advance law ([`advance_set`]).
struct SpanCursors<'a> {
    lo: LeafCursor<'a>,
    /// Whether any pair still reads the `lo` endpoint; a settled endpoint's
    /// stream is never scanned further.
    lo_live: bool,
    hi: LeafCursor<'a>,
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
/// every tie (each is its pairs' first operand, and the binary law's
/// equal-depth arm steps its first operand first), which is what keeps each
/// pair's accumulator write sequence identical to its pair sweep's. `hi` before
/// `lo` and the bounds' order among themselves move no committed reading: no
/// two of those cursors share an accumulator.
impl CursorSet for SpanCursors<'_> {
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static {
        0..self.sides.len() + 2
    }

    /// A settled probe endpoint or dropped bound reads zero and never steps.
    fn depth(&self, slot: usize) -> usize {
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
    fn step(&mut self, slot: usize) -> usize {
        match slot {
            Self::HI => {
                let (flip, step) = self.hi.step();
                for side in self.sides.iter_mut().flatten() {
                    if side.hi.live {
                        fold(&mut side.hi.diff, Side::A, step.sign, &step.magnitude);
                    }
                }
                flip
            }
            Self::LO => {
                let (flip, step) = self.lo.step();
                for side in self.sides.iter_mut().flatten() {
                    if side.lo.live {
                        fold(&mut side.lo.diff, Side::A, step.sign, &step.magnitude);
                    }
                }
                flip
            }
            _ => {
                let side = self.sides[slot - 2]
                    .as_mut()
                    .expect("an absent side reads depth zero and never steps");
                let (flip, step) = side.cursor.step();
                for pair in [&mut side.lo, &mut side.hi] {
                    if pair.live {
                        fold(&mut pair.diff, Side::B, step.sign, &step.magnitude);
                    }
                }
                flip
            }
        }
    }
}
