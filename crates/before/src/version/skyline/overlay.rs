//! The overlay cursors and the advance law: the walk machinery every merge
//! over aligned dyadic streams is built on.
//!
//! A skyline stream lists its version's plateaus left to right — a leaf at
//! depth `d` is a constant run of width `2^-d` — and a packed id stream lists
//! its constant-ownership regions the same way: each stream is a dyadic tiling
//! of the unit id interval, read in preorder. Every multi-stream walk in this
//! crate overlays such tilings, consuming their boundaries in position order.
//!
//! The machinery is crate-private, in three layers. [`PlateauCursor`] is the
//! cursor vocabulary: one dyadic tiling, yielded plateau by plateau, each
//! boundary carrying the cursor's own crossing payload. The overlay-advance
//! law is stated (and debug-asserted) in exactly two generic faces — the
//! binary [`advance`], which hands each crossing to the caller's fold, and the
//! N-ary [`advance_set`] over a walk's whole [`CursorSet`], which folds
//! crossings inside each slot's step; the boundary bookkeeping below is their
//! shared correctness argument. Above them sit the two cursor instances —
//! [`LeafCursor`] walks a
//! skyline stream, its crossings the signed height deltas ([`Step`]);
//! [`IdLeafCursor`] walks a packed id stream, whose ownership is per-region
//! state read between boundaries — and the pair-difference algebra every
//! two-skyline walk shares: [`OpenedPair`] seeds `D = height_a − height_b`
//! from the two absolute opening heights, and [`Side`], [`fold`], and
//! [`advance_diff`] orient every later crossing into it. The traversal folds
//! nothing itself; each client module names what it consumes and the algebra
//! it folds.
//!
//! # The boundary bookkeeping: which cursor advances
//!
//! The two cursors are asymmetric — their current leaves generally sit at
//! different depths, with different interval ends — and the walk never
//! materializes an interval end as a number: an end is `depth` path bits wide,
//! so comparing two of them arithmetically at every boundary would be quadratic
//! on deep streams. Three facts about dyadic intervals replace the arithmetic:
//!
//! - **Overlapping dyadic intervals nest.** The walk's invariant is that
//!   both current leaves contain the *sweep point*, the walk's position
//!   (the latest boundary crossed; the unit interval's left edge before
//!   any), so the two leaf intervals overlap — hence the deeper is
//!   contained in the shallower, and the deeper one's end comes first or
//!   ties. The deeper cursor advances. At equal depths the two intervals
//!   coincide outright (equal-width dyadic intervals sharing a point are
//!   identical), so their ends tie and both cursors advance in the same
//!   step.
//! - **A tie at unequal depths is visible on the deeper cursor's path.**
//!   Advancing pops the path's trailing *right*-branch levels (each is an
//!   ancestor whose subtree the consumed leaf just completed), then steps
//!   the deepest *left*-branch level — the *flip level* — to its right
//!   child. Nesting makes the deeper path extend the shallower one, so
//!   the deeper leaf's end telescopes up to the shallower leaf's end
//!   exactly when its path is all right-branches strictly below the
//!   shallower leaf's depth — that is, when its flip level rises to or
//!   above that depth. So the whole rule is: advance the deeper cursor,
//!   and when its flip level is at or above the other side's depth,
//!   advance the other side in the same step. The two sides then close to
//!   the *same* flip level (their paths agree there), which [`advance`]
//!   debug-asserts at every tie.
//! - **The all-right path is the exhausted stream.** A leaf whose path is
//!   all right-branches is the last leaf in preorder — its plateau ends
//!   at the unit interval's right edge — and it is the current leaf
//!   exactly when the cursor has consumed its whole stream. Canonical
//!   streams therefore exhaust *together*, and a walk stops when both
//!   cursors are done. An advanced cursor always finds a left-branch
//!   level to flip: only a final leaf has none, a cursor at its final
//!   leaf is never the deeper side (the other side's end would have to
//!   reach the right edge too), and a tie against a final leaf means both
//!   are final — the case that already stopped the walk.
//!
//! # Cost
//!
//! Derived: a cursor only moves forward, so every topology bit of a stream is
//! read at most once, every path bit is pushed and popped at most once, and
//! every leaf payload is decoded exactly once — scan, decode, and stack work
//! are linear in the streams' bits. Transient state is one path bit per open
//! ancestor per cursor plus the client's accumulators: a deep operand costs
//! its *bits*, never stack frames. The pair algebra's arithmetic rides the
//! cliff-immune [`Accumulator`]: a machine-word delta costs amortized O(1)
//! digit touches, a wide delta O(its own limbs) — paid by the code the input
//! spent to express it ([`suanpan`]'s crate docs carry the argument). The
//! comparison sweeps pin these constants for the pair walk (the Cost section
//! of [`sweep`](super::sweep)); each other client's meter rows pin its own.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{BitCursor, BitStack, BitsSlice, DsiCursor, Int, SliceCursor};

/// A cursor over one dyadic tiling of the unit interval, yielding its plateaus
/// in preorder.
///
/// What every overlay walk rests on: a *plateau* is one maximal constant run of
/// the cursor's stream — an interval of width `2^-depth` — and stepping past it
/// crosses a boundary that carries the cursor's own payload. The
/// overlay-advance law is stated once over this trait ([`advance`]); what a
/// crossing means — a skyline's signed height delta ([`Step`]), or nothing at
/// all for a cursor whose payload is per-region state read between boundaries
/// (the id cursors) — stays with the cursor, and the traversal folds nothing
/// itself: the crossing is yielded for the caller's algebra.
pub(crate) trait PlateauCursor {
    /// What crossing a boundary carries, for the caller's algebra to
    /// fold.
    type Crossing;

    /// The current plateau's depth: its interval has width `2^-depth`.
    fn depth(&self) -> usize;

    /// Whether the current plateau is the tiling's last (its interval ends at
    /// the unit interval's right edge).
    fn done(&self) -> bool;

    /// Advance past the current plateau: the flip level's depth and the
    /// boundary's crossing.
    ///
    /// The flip level is the path's length *after* the flip, so the flipped
    /// ancestor itself is counted; the boundary just crossed is a multiple of
    /// `2^-flip`, which is what the law's tie test reads — the deeper side's
    /// plateau end reaches the shallower side's exactly when `flip <=
    /// other.depth()` (the module doc's bookkeeping).
    fn step(&mut self) -> (usize, Self::Crossing);
}

/// One crossing the overlay law consumed, tagged with the cursor that crossed
/// it: the argument [`advance`] feeds the caller's fold.
pub(crate) enum Crossed<A, B> {
    /// The `a` cursor's crossing.
    A(A),
    /// The `b` cursor's crossing.
    B(B),
}

/// Advance the overlay walk one boundary — the law, stated once: the deeper
/// cursor steps, and the other steps in the same round exactly when the flip
/// level rises to or above its depth.
///
/// The module doc's boundary bookkeeping is the correctness argument. Tied
/// sides close to one shared flip level, which is debug-asserted at every tie.
///
/// Traversal and algebra are separate: the cursors yield their crossings, and
/// `fold` — the caller's algebra — receives each one *as it is consumed*, in
/// step order (the deeper side's first, `a`'s at equal depths). The order is
/// contract, not convenience: an algebra folding both sides into one shared
/// accumulator commits digit writes whose amortized carry work — and with it
/// the committed touch-meter readings — depends on the write order. The same
/// crossings come back positionally (`None` for a side that did not step) for
/// clients that re-code or re-fold them after the boundary.
pub(crate) fn advance<A: PlateauCursor, B: PlateauCursor>(
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

/// A fixed roster of cursors advancing under one overlay — the state a walk at
/// arity N hands to [`advance_set`].
///
/// Each cursor occupies a numbered *slot*; the set names its slots and answers
/// for them. Where [`PlateauCursor`] carries one cursor and yields its
/// crossings for the caller to fold, a `CursorSet` keeps the folding inside:
/// [`step`](Self::step) both moves the slot's cursor and applies its crossing
/// to the walk's own accumulators, so the driver never sees a crossing type.
///
/// An absent or dropped slot reads depth zero and is never stepped: a walk
/// that is not exhausted has a live cursor strictly below the root, so the
/// pick always lands on a live slot, and a tied step requires depth at or
/// above a flip level, which is at least one.
pub(crate) trait CursorSet {
    /// Every slot, in priority order — the one sequence serving both of the
    /// law's tie-breaks: the pick takes the *first* slot in priority order
    /// achieving the maximum depth, and tied slots step in priority order.
    ///
    /// The order is contract, not convenience: a walk whose slots share an
    /// accumulator commits its digit writes in step order, so the committed
    /// touch-meter readings pin each walk's sequence (each impl documents
    /// which identities pin its own). The iterator is owned (`'static`) — a
    /// const-shaped array or index range, never allocated per round — so the
    /// driver can hold it across the mutable steps.
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static;

    /// The slot's current plateau depth: its interval has width `2^-depth`.
    /// An absent or dropped slot reads zero.
    fn depth(&self, slot: usize) -> usize;

    /// Step the slot past its plateau, folding its crossing into the walk's
    /// own algebra; returns the flip level.
    fn step(&mut self, slot: usize) -> usize;
}

/// Advance an overlay walk of N cursors one boundary — the overlay-advance law
/// at arity N: the deepest slot steps, and every other slot whose depth
/// reaches the flip level steps in the same round.
///
/// The module doc's boundary bookkeeping is the correctness argument,
/// unchanged at higher arity: every current leaf or region contains the sweep
/// point, so all the intervals nest by depth — the deepest slot's plateau ends
/// first, and a shallower slot's end ties exactly when the flip level rises to
/// or above its depth. Tied sides close to one shared flip level,
/// debug-asserted here at every tie. The set's single
/// [`priority`](CursorSet::priority) sequence fixes both tie-breaks: which of
/// several equally-deep slots is picked, and the order tied slots step in.
///
/// This is the law's arity-N, fold-internal face: each crossing is folded
/// inside [`CursorSet::step`], and nothing is returned. [`advance`] beside it
/// is the same law's arity-2, crossing-explicit face — it hands each crossing
/// to the caller's fold and returns the pair, which emission and the pair
/// integrals need.
pub(crate) fn advance_set(set: &mut impl CursorSet) {
    let priority = set.priority();
    let mut deepest: Option<(usize, usize)> = None;
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

/// One boundary crossing on a skyline stream: the leaf-to-leaf delta the step
/// consumed ([`LeafCursor`]'s [`Crossing`](PlateauCursor::Crossing)).
pub(super) struct Step {
    /// Whether the delta lowers this stream's height.
    pub(super) negative: bool,
    /// The delta's absolute value.
    pub(super) magnitude: Int,
}

/// A cursor at the current leaf of one skyline stream.
///
/// Holds the sequential bit cursor and the root-to-leaf path — one bit per open
/// ancestor: `false` inside its left child (the right subtree is still pending
/// in the stream), `true` inside its right. The path is the only per-depth
/// state; no height, base, or node is retained, which is what keeps a sweep's
/// transient linear in depth *bits*. Every skyline walk shares the cursor
/// through [`PlateauCursor`]: the [`Step`]s it yields are folded by each
/// client's own algebra (the pair clients through [`fold`]), and emission
/// additionally re-codes them.
pub(super) struct LeafCursor<'a> {
    cursor: DsiCursor<'a>,
    /// Root-to-leaf branch directions, root first.
    path: BitStack,
    /// The stream's live bit length; the cursor reaching it is
    /// exhaustion (the current leaf is the stream's last).
    len: usize,
}

impl<'a> LeafCursor<'a> {
    /// Open a stream at its first leaf, returning the cursor and that leaf's
    /// absolute height (later leaves carry zigzag deltas, which
    /// [`step`](PlateauCursor::step) decodes instead).
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    pub(super) fn open(bits: &'a BitsSlice) -> (Self, Int) {
        let mut this = LeafCursor {
            cursor: DsiCursor::new(bits),
            path: BitStack::new(),
            len: bits.len(),
        };
        let first = this.descend();
        (this, first)
    }

    /// The flip level the next step would close to, read without moving.
    ///
    /// The path's trailing right-branch run popped and the deepest left branch
    /// flipped. Zero on a final leaf (the all-right path), where no step
    /// remains — every real flip level is at least one.
    pub(super) fn peek_flip(&self) -> usize {
        self.path.len() - self.path.trailing_ones()
    }

    /// Consume plateaus while the next boundary's flip level stays strictly
    /// deeper than `bound`, folding every crossed delta into `net` (positively
    /// oriented: the caller applies its own side).
    ///
    /// The ownership-gated walks' block consume: a boundary whose flip level
    /// exceeds every other cursor's depth is crossed by this cursor alone, so a
    /// caller that has established the crossed intervals carry no verdict or
    /// output of their own needs only the net height movement to re-enter.
    /// Stops with the cursor at the first plateau whose end reaches level
    /// `bound` or shallower — a final leaf stops the loop unconditionally (its
    /// peek is zero), so exhaustion needs no separate guard.
    ///
    /// Every skipped bit is still read and recorded: the scan meter's reading
    /// is identical to the plateau-by-plateau walk this batches.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    pub(super) fn skip_deeper(&mut self, bound: usize, net: &mut Accumulator) {
        while self.peek_flip() > bound {
            let (_, step) = self.step();
            super::signed::fold_signed_int(net, step.negative, &step.magnitude);
        }
    }

    /// Descend from the cursor to the next leaf in preorder, extending the path
    /// with a left branch per internal node passed.
    ///
    /// Returns the leaf's payload code undecoded: absolute for the stream's
    /// first leaf, zigzag for every later one.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    fn descend(&mut self) -> Int {
        // One word-parallel unary read takes the whole descent: the run of
        // internal flags ends at the leaf's `1`. The scan meter records the
        // same run width the per-flag reads would.
        let internal_nodes = self.cursor.read_unary().expect("canonical skyline bits");
        for _ in 0..internal_nodes {
            self.path.push(false);
        }
        // The cursor's own `read_int`, so the payload decode takes the
        // word-parallel fast path; the scan meter records the same `2k + 1`
        // bits either way.
        self.cursor.read_int().expect("canonical skyline bits")
    }
}

impl PlateauCursor for LeafCursor<'_> {
    type Crossing = Step;

    /// The current leaf's depth: its plateau has width `2^-depth`.
    fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether the current leaf is the stream's last (its plateau ends at the
    /// unit interval's right edge).
    fn done(&self) -> bool {
        self.cursor.position() == self.len
    }

    /// Advance past the current leaf to the next: the flip level's depth for
    /// the caller's tie test, and the leaf-to-leaf zigzag delta for the
    /// caller's fold.
    ///
    /// Pops the trailing right-branch levels (each ancestor's subtree the
    /// consumed leaf completed), steps the deepest left-branch level to its
    /// right child, and descends to the next leaf.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding. Never called
    /// on a final leaf: a sweep stops when both cursors are done, and the
    /// module doc's bookkeeping shows a final leaf is never the advanced side
    /// before then.
    fn step(&mut self) -> (usize, Step) {
        loop {
            match self.path.pop() {
                Some(true) => continue, // this ancestor closed with the leaf
                Some(false) => break,   // the flip level: its right subtree is next
                None => unreachable!(
                    "the advanced cursor is never at its final leaf: an all-right path means the stream is consumed"
                ),
            }
        }
        self.path.push(true);
        let flip = self.path.len();
        let code = self.descend();
        let (negative, magnitude) = super::signed::unzigzag(code);
        (
            flip,
            Step {
                negative,
                magnitude,
            },
        )
    }
}

/// A cursor at the current constant-ownership region of a packed id stream.
///
/// The id-side mirror of the skyline [`LeafCursor`]: the same root-to-leaf path
/// bits and the same flip bookkeeping, entering every overlay through
/// [`PlateauCursor`] with a state payload (owned or not, read between
/// boundaries) instead of a height delta. Absent children in the packed form
/// are unowned regions, so the cursor synthesizes an empty leaf wherever a
/// present-child flag is clear without consuming stream bits; exhaustion is
/// therefore tracked by the path's left-branch count (zero means the current
/// leaf is the preorder last), not by stream position.
pub(super) struct IdLeafCursor<'a> {
    cursor: SliceCursor<'a>,
    /// Root-to-leaf branch directions, root first.
    path: BitStack,
    /// Parallel to `path`: whether each level's right child is present in the
    /// stream (a clear flag is a synthetic unowned leaf).
    right_present: BitStack,
    /// Left-branch levels still open; zero exactly at the final leaf.
    lefts: usize,
    /// Whether the current leaf's region is owned.
    owned: bool,
}

impl<'a> IdLeafCursor<'a> {
    /// Open a packed id stream at its first constant region.
    ///
    /// The empty stream is the empty id — one unowned region over the whole
    /// interval — mirroring the packed coding, where absence *is* the empty
    /// region.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical packed id.
    pub(super) fn open(bits: &'a BitsSlice) -> Self {
        let mut this = IdLeafCursor {
            cursor: SliceCursor::new(bits, 0),
            path: BitStack::new(),
            right_present: BitStack::new(),
            lefts: 0,
            owned: false,
        };
        if !bits.is_empty() {
            this.descend();
        }
        this
    }

    /// Whether the current region is owned by the id.
    pub(super) fn owned(&self) -> bool {
        self.owned
    }

    /// Descend from the cursor to the next stored region in preorder, extending
    /// the path with a left branch per internal node passed.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical packed id.
    fn descend(&mut self) {
        loop {
            // The two tag-bit reads below record themselves through the
            // cursor's recording `read_bit`: no separate tag record, or every
            // 2-bit tag would count twice.
            let left = self.cursor.read_bit().expect("canonical id bits");
            let right = self.cursor.read_bit().expect("canonical id bits");
            if !left && !right {
                // The full leaf: an owned terminal region.
                self.owned = true;
                return;
            }
            self.path.push(false);
            self.lefts += 1;
            self.right_present.push(right);
            if !left {
                // The absent left child: a synthetic unowned region.
                self.owned = false;
                return;
            }
        }
    }
}

impl PlateauCursor for IdLeafCursor<'_> {
    /// An id boundary carries no payload: ownership is the *current* region's
    /// state, read between boundaries ([`owned`](IdLeafCursor::owned)), never a
    /// crossing delta.
    type Crossing = ();

    /// The current region's depth: its interval has width `2^-depth`.
    fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether the current region is the stream's last (its interval ends at
    /// the unit interval's right edge).
    fn done(&self) -> bool {
        self.lefts == 0
    }

    /// Advance past the current region to the next: the flip level's depth for
    /// the law's tie test, and the empty crossing.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical packed id. Never called on a
    /// final region (the overlay stops when both cursors are done).
    fn step(&mut self) -> (usize, ()) {
        loop {
            match self.path.pop() {
                Some(true) => {
                    self.right_present.pop();
                    continue;
                }
                Some(false) => break,
                None => unreachable!(
                    "the advanced cursor is never at its final region: an all-right path means the stream is consumed"
                ),
            }
        }
        self.lefts -= 1;
        self.path.push(true);
        let flip = self.path.len();
        if self
            .right_present
            .last()
            .expect("a flipped level recorded its right-child flag")
        {
            self.descend();
        } else {
            // The absent right child: one synthetic unowned region at the
            // flip level itself.
            self.owned = false;
        }
        (flip, ())
    }
}

/// Which side of the difference `D = height_a − height_b` a stream feeds.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    /// The left operand: its heights enter `D` positively.
    A,
    /// The right operand: its heights enter `D` negatively.
    B,
}

impl Side {
    /// The opposite side: folding a delta on it negates the delta's effect on
    /// the difference, which is how the pair co-sweep applies an orientation
    /// coefficient of −1 without touching the magnitude.
    pub(super) fn other(self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}

/// Fold one decoded leaf delta into the running difference, oriented by the
/// side its stream feeds: `a`'s height rising raises `D`, `b`'s lowers it.
pub(super) fn fold(diff: &mut Accumulator, side: Side, negative: bool, magnitude: &Int) {
    let lowers_diff = match side {
        Side::A => negative,
        Side::B => !negative,
    };
    super::signed::fold_signed_int(diff, lowers_diff, magnitude);
}

/// Advance the skyline pair overlay one boundary, folding each consumed delta
/// into the running difference `D = height_a − height_b` as it is consumed.
///
/// The one shared algebra over [`advance`]: the comparison sweep, the join/meet
/// emission, and the pair integrals all maintain exactly this difference, so
/// its application lives here once. Returns each side's consumed delta (`None`
/// for a side that did not step), which the emission sweep re-codes, the pair
/// integrals re-fold into their integrand, and the comparison sweep discards.
pub(super) fn advance_diff(
    a: &mut LeafCursor<'_>,
    b: &mut LeafCursor<'_>,
    diff: &mut Accumulator,
) -> (Option<Step>, Option<Step>) {
    advance(a, b, |crossing| {
        let (side, step) = match crossing {
            Crossed::A(step) => (Side::A, step),
            Crossed::B(step) => (Side::B, step),
        };
        fold(diff, side, step.negative, &step.magnitude);
    })
}

/// A two-skyline overlay, opened: both cursors at their first leaves and the
/// running difference `D = height_a − height_b` seeded with the two absolute
/// opening heights.
///
/// The shared opening move of every two-skyline walk, stated once so the
/// seeding's orientation — `a` positive, `b` negative, the orientation [`fold`]
/// applies to every later crossing — has one home. The opening heights ride
/// along for the clients that consume an absolute opening (the emission sweep's
/// first output leaf, the masked walk's height integrators); the seeded
/// difference already carries their values, so the comparison sweep and the
/// pair integrals drop them unread.
pub(super) struct OpenedPair<'a> {
    /// The left operand's cursor, at its first leaf.
    pub(super) a: LeafCursor<'a>,
    /// The right operand's cursor, at its first leaf.
    pub(super) b: LeafCursor<'a>,
    /// The running difference, seeded `a_first − b_first`.
    pub(super) diff: Accumulator,
    /// The left operand's absolute first height.
    pub(super) a_first: Int,
    /// The right operand's absolute first height.
    pub(super) b_first: Int,
}

impl<'a> OpenedPair<'a> {
    /// Open both streams at their first leaves and seed the difference.
    ///
    /// # Panics
    ///
    /// Panics if either stream is not a canonical skyline encoding.
    pub(super) fn open(a_bits: &'a BitsSlice, b_bits: &'a BitsSlice) -> OpenedPair<'a> {
        let (a, a_first) = LeafCursor::open(a_bits);
        let (b, b_first) = LeafCursor::open(b_bits);
        let mut diff = Accumulator::new();
        super::signed::fold_signed_int(&mut diff, false, &a_first);
        super::signed::fold_signed_int(&mut diff, true, &b_first);
        OpenedPair {
            a,
            b,
            diff,
            a_first,
            b_first,
        }
    }
}
