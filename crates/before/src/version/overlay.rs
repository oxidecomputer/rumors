//! Forward cursors over dyadic interval partitions.
//!
//! A Version partitions the unit interval into constant-height regions; a
//! party partitions it into constant-ownership regions. A leaf at depth `d`
//! spans `2^-d`. Overlay operations walk these partitions together without
//! materializing interval endpoints.
//!
//! At every step, all current leaves contain the current position. Overlapping
//! dyadic intervals nest, so the deepest leaf ends first. If advancing it flips
//! an ancestor at or above another leaf's depth, that leaf ends at the same
//! boundary and advances too. Equal-depth leaves coincide and always advance
//! together. A final leaf extends to the end of the unit interval, so it is
//! never advanced alone.
//!
//! Each cursor moves only forward. Every topology bit is read once, every path
//! bit is pushed and popped once, and every Version payload is decoded once.
//! State is proportional to the open paths plus the operation's accumulators;
//! traversal uses no recursion or endpoint-sized integers.

use core::cmp::Ordering;

use suanpan::Accumulator;

use num_bigint::{BigInt, BigUint};

use crate::accumulator::BigIntAccumulator as _;
use crate::party::io::PartyRegionReader;
use crate::version::io::regions::{HeightChange, RegionReader, VersionRegionReader};
use crate::Version;

/// One crossing the overlay law consumed, tagged with the cursor that crossed
/// it: the argument [`advance`] feeds the caller's fold.
pub enum Crossed<A, B> {
    /// The `a` cursor's crossing.
    A(A),
    /// The `b` cursor's crossing.
    B(B),
}

/// Advance to the next shared boundary.
///
/// The deeper cursor steps first. The other also steps when the first cursor's
/// flip reaches its depth. `fold` receives crossings in that order; the return
/// value also identifies them by side for later use. Call only while at least
/// one cursor is not at its final leaf.
pub fn advance<A: RegionReader, B: RegionReader>(
    a: &mut A,
    b: &mut B,
    mut fold: impl FnMut(Crossed<&A::Crossing, &B::Crossing>),
) -> (Option<A::Crossing>, Option<B::Crossing>) {
    match a.depth().cmp(&b.depth()) {
        Ordering::Greater => {
            let (flip_a, crossing_a) = a.step();
            fold(Crossed::A(&crossing_a));
            let crossing_b = (flip_a <= b.depth()).then(|| {
                let (flip_b, crossing_b) = b.step();
                debug_assert_eq!(
                    flip_a, flip_b,
                    "tied boundaries close to one shared flip level"
                );
                fold(Crossed::B(&crossing_b));
                crossing_b
            });
            (Some(crossing_a), crossing_b)
        }
        Ordering::Less => {
            let (flip_b, crossing_b) = b.step();
            fold(Crossed::B(&crossing_b));
            let crossing_a = (flip_b <= a.depth()).then(|| {
                let (flip_a, crossing_a) = a.step();
                debug_assert_eq!(
                    flip_b, flip_a,
                    "tied boundaries close to one shared flip level"
                );
                fold(Crossed::A(&crossing_a));
                crossing_a
            });
            (crossing_a, Some(crossing_b))
        }
        Ordering::Equal => {
            let (flip_a, crossing_a) = a.step();
            fold(Crossed::A(&crossing_a));
            let (flip_b, crossing_b) = b.step();
            debug_assert_eq!(
                flip_a, flip_b,
                "equal-depth leaves share their whole path, so their flip levels agree"
            );
            fold(Crossed::B(&crossing_b));
            (Some(crossing_a), Some(crossing_b))
        }
    }
}

/// A fixed set of cursors advanced by one overlay.
///
/// Each cursor occupies a numbered *slot*; the set names its slots and answers
/// for them. Where [`RegionReader`] carries one cursor and yields its
/// crossings for the caller to fold, a `CursorSet` keeps the folding inside:
/// [`step`](Self::step) both moves the slot's cursor and applies its crossing
/// to the walk's own accumulators, so the driver never sees a crossing type.
///
/// An absent or dropped slot reads depth zero and is never stepped: a depth-0
/// plateau tiles the whole interval and is final, so a live, unexhausted cursor
/// sits strictly deeper — a done slot is always strictly shallower than any
/// not-done one. The pick therefore always lands on a live slot, and a tied
/// step requires depth at or above a flip level, which is at least one.
pub trait CursorSet {
    /// Every slot, in priority order — the one sequence serving both of the
    /// law's tie-breaks: the pick takes the *first* slot in priority order
    /// achieving the maximum depth, and tied slots step in priority order.
    ///
    /// Semantically the choice is free only because every client algebra folds
    /// commutative sums — any order yields the same fold values; a client with
    /// a non-commutative fold would make the tie-break part of its answer,
    /// which no current client does. The order is contract, not convenience: a
    /// walk whose slots share an accumulator commits its digit writes in step
    /// order, so the committed touch-meter readings pin each walk's sequence
    /// (each impl documents which identities pin its own). The iterator is
    /// owned (`'static`) — a const-shaped array or index range, never allocated
    /// per round — so the driver can hold it across the mutable steps.
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static;

    /// The slot's current plateau depth: its interval has width `2^-depth`.
    /// An absent or dropped slot reads zero.
    fn depth(&self, slot: usize) -> u64;

    /// Step the slot past its plateau, folding its crossing into the walk's
    /// own algebra; returns the flip level.
    fn step(&mut self, slot: usize) -> u64;
}

/// Advance an overlay walk of N cursors one boundary — the overlay-advance law
/// at arity N: the deepest slot steps, and every other slot whose depth reaches
/// the flip level steps in the same round.
///
/// The module doc's boundary bookkeeping is the correctness argument, unchanged
/// at higher arity: every current leaf or region contains the sweep point, so
/// all the intervals nest by depth — the deepest slot's plateau ends first, and
/// a shallower slot's end ties exactly when the flip level rises to or above
/// its depth. Tied sides close to one shared flip level, debug-asserted here at
/// every tie. The set's single [`priority`](CursorSet::priority) sequence fixes
/// both tie-breaks: which of several equally-deep slots is picked, and the
/// order tied slots step in.
///
/// This is the law's arity-N, fold-internal face: each crossing is folded
/// inside [`CursorSet::step`], and nothing is returned. [`advance`] beside it
/// is the same law's arity-2, crossing-explicit face — it hands each crossing
/// to the caller's fold and returns the pair, which emission and the pair
/// integrals need.
pub fn advance_set(set: &mut impl CursorSet) {
    let priority = set.priority();
    let mut deepest: Option<(usize, u64)> = None;
    for slot in priority.clone() {
        let depth = set.depth(slot);
        // Strict: the first slot in priority order achieving the maximum.
        if deepest.is_none_or(|(_, max)| depth > max) {
            deepest = Some((slot, depth));
        }
    }
    let (deepest, _) = deepest.expect("a cursor set has at least one slot");
    let flip = set.step(deepest);
    for slot in priority {
        if slot != deepest && set.depth(slot) >= flip {
            let tied = set.step(slot);
            debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
        }
    }
}

impl RegionReader for PartyRegionReader<'_> {
    /// A Party boundary carries no value. Ownership describes the current
    /// region and is read between boundaries.
    type Crossing = ();

    fn depth(&self) -> u64 {
        PartyRegionReader::depth(self)
    }

    fn done(&self) -> bool {
        PartyRegionReader::done(self)
    }

    fn step(&mut self) -> (u64, ()) {
        (self.advance(), ())
    }
}

/// Which side of the difference `D = height_a − height_b` a stream feeds.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The left operand: its heights enter `D` positively.
    A,
    /// The right operand: its heights enter `D` negatively.
    B,
}

impl Side {
    /// The opposite side: folding a delta on it negates the delta's effect on
    /// the difference, which is how the pair co-sweep applies an orientation
    /// coefficient of −1 without touching the magnitude.
    pub fn other(self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }

    /// Fold one delta into `D = height_a - height_b` from this side.
    pub fn fold(self, diff: &mut Accumulator, delta: &BigInt) {
        match self {
            Side::A => diff.add_bigint(delta),
            Side::B => diff.sub_bigint(delta),
        }
    }
}

/// Advance the two-Version overlay one boundary, folding each consumed delta
/// into the running difference `D = height_a − height_b` as it is consumed.
///
/// The one shared algebra over [`advance`]: the comparison sweep, the join/meet
/// emission, and the pair integrals all maintain exactly this difference, so
/// its application lives here once. Returns each side's consumed delta (`None`
/// for a side that did not step), which the emission sweep re-codes, the pair
/// integrals re-fold into their integrand, and the comparison sweep discards.
pub fn advance_diff(
    a: &mut VersionRegionReader<'_>,
    b: &mut VersionRegionReader<'_>,
    diff: &mut Accumulator,
) -> (Option<HeightChange>, Option<HeightChange>) {
    advance(a, b, |crossing| {
        let (side, step) = match crossing {
            Crossed::A(step) => (Side::A, step),
            Crossed::B(step) => (Side::B, step),
        };
        side.fold(diff, step);
    })
}

/// A two-Version overlay, opened with both cursors at their first leaves and the
/// running difference `D = height_a − height_b` seeded with the two absolute
/// opening heights.
///
/// The shared opening move of every two-Version walk, stated once so the
/// seeding's orientation — `a` positive, `b` negative, the orientation [`Side::fold`]
/// applies to every later crossing — has one home. The opening heights ride
/// along for the clients that consume an absolute opening (the emission sweep's
/// first output leaf, the masked walk's height integrators); the seeded
/// difference already carries their values, so the comparison sweep and the
/// pair integrals drop them unread.
pub struct OpenedPair<'a> {
    /// The left operand's cursor, at its first leaf.
    pub a: VersionRegionReader<'a>,
    /// The right operand's cursor, at its first leaf.
    pub b: VersionRegionReader<'a>,
    /// The running difference, seeded `a_first − b_first`.
    pub diff: Accumulator,
    /// The left operand's absolute first height.
    pub a_first: BigUint,
    /// The right operand's absolute first height.
    pub b_first: BigUint,
}

impl<'a> OpenedPair<'a> {
    /// Open both streams at their first leaves and seed the difference.
    ///
    /// # Panics
    ///
    /// Panics if either stream is not a canonical Version encoding.
    pub fn open(a: &'a Version, b: &'a Version) -> OpenedPair<'a> {
        let (a, a_first) = VersionRegionReader::open(a);
        let (b, b_first) = VersionRegionReader::open(b);
        let mut diff = Accumulator::new();
        diff.add_shifted_limbs(0, a_first.iter_u64_digits());
        diff.sub_shifted_limbs(0, b_first.iter_u64_digits());
        OpenedPair {
            a,
            b,
            diff,
            a_first,
            b_first,
        }
    }
}
