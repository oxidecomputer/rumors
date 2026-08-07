//! The overlay walk over two skyline streams, and the comparison sweeps —
//! order, equality, domination, concurrency — built on it.
//!
//! The walk machinery here is crate-private and shared, in three layers.
//! [`PlateauCursor`] is the cursor vocabulary: one dyadic tiling of the unit
//! interval, yielded plateau by plateau, each boundary carrying the cursor's
//! own crossing payload. The generic [`advance`] is the overlay-advance law
//! over two such cursors, stated (and debug-asserted) once. [`LeafCursor`] is
//! the skyline instance — its crossings are [`Step`]s — and the pair-difference
//! algebra every two-skyline walk shares lives beside it: [`OpenedPair`] seeds
//! `D = height_a − height_b` from the two absolute opening heights, and
//! [`Side`], [`fold`], and [`advance_diff`] orient every later crossing into
//! it. The clients: this module's own comparison entry points, which fold
//! heights and discard the steps; the join/meet emission
//! ([`emit`](super::emit)), which re-codes them into an output stream; and the
//! pair integrals ([`query`](super::query)'s distance and lag), which re-fold
//! them into a directed integrand. The single-cursor folds
//! ([`query`](super::query)'s rank and min_ticks) consume [`LeafCursor`] and
//! [`fold`] without the pair walk; the projection overlay
//! ([`query`](super::query)'s project) runs [`advance`] over the skyline × id
//! cursor mix; the [`masked`](super::masked) walk runs the law at full arity on
//! the trait; the placement walk ([`place`](mod@super::place)) restates it at
//! three cursors, seeding one pair difference per bound; and the id difference
//! ([`IdReader::diff`](crate::idbits::IdReader::diff)) runs [`advance`] over
//! its own boolean cursors, settling covered blocks between boundaries. The
//! boundary bookkeeping below is the shared correctness argument; the prose
//! reads it through comparison, the simplest client.
//!
//! A skyline stream lists its version's plateaus left to right: a leaf at depth
//! `d` is a constant run of width `2^-d` over the unit id interval. Two
//! versions overlay into a common refinement — the *elementary intervals*, the
//! maximal spans crossing no leaf boundary of either partition — and every
//! comparison the crate asks is pointwise over that refinement: `a <= b` iff no
//! elementary interval has `a`'s height above `b`'s, `b <= a` iff none has the
//! reverse, equal iff both hold, concurrent iff neither. The sweep maintains
//! **one** running signed difference `D = height_a − height_b` on the
//! cliff-immune [`Accumulator`], folds `sign(D)` once per elementary interval,
//! and advances whichever cursor's plateau ends first. Nothing recurses, and no
//! synthetic zero subtree is ever walked when one side bottoms out early: a
//! leaf is one long plateau, and the other side's boundaries are consumed
//! against it iteratively. No per-level value is saved anywhere — the transient
//! state is two path-bit stacks (one bit per open ancestor per side) and the
//! one accumulator.
//!
//! # The boundary bookkeeping: which cursor advances
//!
//! The two cursors are asymmetric — their current leaves generally sit at
//! different depths, with different interval ends — and the sweep never
//! materializes an interval end as a number: an end is `depth` path bits wide,
//! so comparing two of them arithmetically at every boundary would be quadratic
//! on deep streams. Three facts about dyadic intervals replace the arithmetic:
//!
//! - **Overlapping dyadic intervals nest.** The sweep's invariant is that
//!   both current leaves contain the sweep point (the left edge of the
//!   elementary interval being folded), so the two leaf intervals
//!   overlap — hence the deeper is contained in the shallower, and the
//!   deeper one's end comes first or ties. The deeper cursor advances. At
//!   equal depths the two intervals coincide outright (equal-width dyadic
//!   intervals sharing a point are identical), so their ends tie and both
//!   cursors advance in the same step.
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
//!   streams therefore exhaust *together*, and the sweep stops when both
//!   cursors are done. An advanced cursor always finds a left-branch
//!   level to flip: only a final leaf has none, a cursor at its final
//!   leaf is never the deeper side (the other side's end would have to
//!   reach the right edge too), and a tie against a final leaf means both
//!   are final — the case that stopped the loop instead.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the `skyline_cmp_*` rows of the
//! resource-envelope suite (`tests/meter.rs`): every topology bit of either
//! stream is read at most once (the cursors only move forward), every path bit
//! is pushed and popped at most once, and every leaf payload is decoded and
//! folded into `D` exactly once, so scan, decode, and stack work are all linear
//! in the two streams' bits. The accumulator prices the arithmetic: a
//! machine-word delta costs amortized O(1) digit touches, a wide delta O(its
//! own limbs) — paid by the code the input spent to express it — and each
//! per-interval `sign(D)` is amortized O(1) against the writes that preceded it
//! ([`suanpan`]'s crate docs carry both arguments; the envelope suite's
//! flatness rows pin the per-delta cost flat across a boundary-comb size
//! doubling). Transient space is one path bit per open ancestor per side plus
//! the accumulator, so comparing a deep operand against a shallow one costs the
//! deep side's *bits*, not its frames: the envelope rows pin every scenario at
//! zero grown stack segments.
//!
//! # Early exit
//!
//! The question asked decides the earliest stop, so each entry point passes its
//! question to the shared sweep as an exit predicate breaking with the decided
//! verdict: [`causal_cmp`] stops when both directions are excluded (the strict
//! mix reading concurrent — any other verdict needs exhaustion), [`eq`] when
//! either is (any nonzero `D` refutes equality), [`le`] when its one direction
//! is (any `D > 0`). A refuted direction stays refuted, so breaking at the
//! question's resolution never moves a verdict the completed sweep would have
//! reached, and a decided sweep reads no more of either stream.
//!
//! # Testing
//!
//! The stored-form comparison ([`Version`](crate::Version)'s `PartialOrd`) is
//! the verdict oracle: differential tests pin all four entry points against it
//! over the adversarial generator families, arbitrary normal-form trees,
//! organic op-trace histories, and the exhaustive small scope — every ordered
//! pair of normal-form event trees to the depth bound stated and argued in the
//! test-only `testing::exhaustive` module, which reaches every boundary genre
//! (aligned ties, flush-right ties at unequal depths, plateau consumption, zero
//! deltas across subtree boundaries) by brute force rather than sampling. The
//! resource envelopes are the meter rows named above.

// The module doc names its crate-private machinery by intra-doc link so a
// rename cannot rot the prose (the internal doc build resolves every link); on
// the public build those links render as plain code spans — the items are
// private — which this allow accepts.
#![allow(rustdoc::private_intra_doc_links)]

use core::cmp::Ordering;
use core::ops::ControlFlow;

use suanpan::Accumulator;

use crate::codec::{BitCursor, BitStack, BitsSlice, DsiCursor, Int};

/// The causal order of the versions two skyline streams denote; `None` is
/// concurrent.
///
/// One merge over the two streams, folding the sign of the running height
/// difference per elementary interval; stops at the first strict mix. Every
/// verdict matches the stored-form comparison exactly (the module doc's
/// differential suite pins all four outcomes).
///
/// # Panics
///
/// Operands must be canonical skyline streams — run
/// [`validate`](fn@super::validate) first on untrusted bytes. The violations
/// the walk structurally notices (truncation, malformation) panic; the rest (a
/// collapsible sibling pair, a delta driving the running height negative) sweep
/// silently, and the verdict is then unspecified.
pub fn causal_cmp(a: &BitsSlice, b: &BitsSlice) -> Option<Ordering> {
    // Clone identity decides reflexivity without a walk: one shared stored
    // buffer read through two views is bit-for-bit one stream (`Version::clone`
    // is a refcount bump), and a version compares `Equal` to itself — the
    // `order_reflexive` law in `crate::laws`. Equal streams in distinct buffers
    // still take the sweep below.
    if crate::codec::slice_ptr_eq(a, b) {
        return Some(Ordering::Equal);
    }
    // At exhaustion every surviving combination is a verdict.
    sweep(a, b, order_exit, Directions::relation)
}

/// Whether two skyline streams denote the same version.
///
/// The sweep form of equality: stops at the first elementary interval whose
/// height difference is nonzero. Canonical uniqueness makes plain byte equality
/// an equivalent test; the sweep form is the one whose verdicts the
/// differential suite pins, and it shares the other entry points' cost bounds.
///
/// # Panics
///
/// [`causal_cmp`]'s contract exactly: canonical operands required, structural
/// violations panic, the rest yield an unspecified verdict.
///
/// Test- and meter-only: production equality is the stored forms' byte equality
/// (canonical uniqueness makes them the same test).
#[cfg(any(test, feature = "meter"))]
pub fn eq(a: &BitsSlice, b: &BitsSlice) -> bool {
    // Surviving both directions to exhaustion is equality.
    sweep(a, b, eq_exit, |dirs| dirs.le && dirs.ge)
}

/// Whether the versions two skyline streams denote are concurrent: neither
/// dominates the other.
///
/// [`causal_cmp`]'s `None` outcome as a predicate. The sweep exits at the first
/// strict mix, so a concurrent verdict is typically decided well before either
/// stream ends.
///
/// # Panics
///
/// [`causal_cmp`]'s contract exactly: canonical operands required, structural
/// violations panic, the rest yield an unspecified verdict.
///
/// Test- and meter-only: production concurrency checks go through
/// [`Version::concurrent`](crate::Version::concurrent) over the same sweep.
#[cfg(any(test, feature = "meter"))]
pub fn concurrent(a: &BitsSlice, b: &BitsSlice) -> bool {
    causal_cmp(a, b).is_none()
}

/// Whether the first stream's version is dominated by the second's: `a <= b`
/// pointwise over the unit id interval.
///
/// The single-direction fold behind causal containment checks: stops at the
/// first elementary interval where `a`'s height exceeds `b`'s.
///
/// # Panics
///
/// [`causal_cmp`]'s contract exactly: canonical operands required, structural
/// violations panic, the rest yield an unspecified verdict.
///
/// Test- and meter-only: production ordering goes through the `PartialOrd`
/// surface over [`causal_cmp`].
#[cfg(any(test, feature = "meter"))]
pub fn le(a: &BitsSlice, b: &BitsSlice) -> bool {
    sweep(
        a,
        b,
        // Domination `a <= b` stops the moment that one direction is refuted;
        // the other direction never matters.
        |dirs| {
            if dirs.le {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(false)
            }
        },
        // Surviving to exhaustion confirms the domination.
        |dirs| dirs.le,
    )
}

/// The two surviving directions of a comparison walk: which of `a <= b` / `b <=
/// a` no folded elementary interval has refuted yet.
///
/// Every comparison walk — this module's sweep, the masked co-walk, the
/// placement walk's bound sides — folds one sign per elementary interval into
/// this pair and asks its question of the survivors. A refuted direction stays
/// refuted, which is what makes every early exit sound.
#[derive(Clone, Copy)]
pub(super) struct Directions {
    /// `a <= b` still possible (no interval put `a`'s height above).
    pub(super) le: bool,
    /// `b <= a` still possible (no interval put `b`'s height above).
    pub(super) ge: bool,
}

impl Directions {
    /// Both directions open: nothing folded yet.
    pub(super) fn new() -> Directions {
        Directions { le: true, ge: true }
    }

    /// Fold one elementary interval's sign of `D = height_a − height_b` into
    /// the surviving directions: a positive interval refutes `a <= b`, a
    /// negative one `b <= a`.
    pub(super) fn fold(&mut self, sign: Ordering) {
        match sign {
            Ordering::Greater => self.le = false,
            Ordering::Less => self.ge = false,
            Ordering::Equal => {}
        }
    }

    /// The relation the folded directions decide, as the causal order: both
    /// surviving is equality, one is the strict order, neither is concurrent
    /// (`None`).
    pub(super) fn relation(self) -> Option<Ordering> {
        match (self.le, self.ge) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

/// The full order's exit question: stop early only at the strict mix — both
/// directions refuted reads concurrent, and no other verdict is known before
/// exhaustion.
pub(super) fn order_exit(dirs: Directions) -> ControlFlow<Option<Ordering>> {
    if !dirs.le && !dirs.ge {
        ControlFlow::Break(None)
    } else {
        ControlFlow::Continue(())
    }
}

/// Equality's exit question: stop the moment either direction is refuted — any
/// nonzero difference refutes equality.
pub(super) fn eq_exit(dirs: Directions) -> ControlFlow<bool> {
    if !dirs.le || !dirs.ge {
        ControlFlow::Break(false)
    } else {
        ControlFlow::Continue(())
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
    super::fold_signed_int(diff, lowers_diff, magnitude);
}

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
            let (fa, ca) = a.step();
            fold(Crossed::A(&ca));
            let cb = (fa <= b.depth()).then(|| {
                let (fb, cb) = b.step();
                debug_assert_eq!(fa, fb, "tied boundaries close to one shared flip level");
                fold(Crossed::B(&cb));
                cb
            });
            (Some(ca), cb)
        }
        Ordering::Less => {
            let (fb, cb) = b.step();
            fold(Crossed::B(&cb));
            let ca = (fb <= a.depth()).then(|| {
                let (fa, ca) = a.step();
                debug_assert_eq!(fb, fa, "tied boundaries close to one shared flip level");
                fold(Crossed::A(&ca));
                ca
            });
            (ca, Some(cb))
        }
        Ordering::Equal => {
            let (fa, ca) = a.step();
            fold(Crossed::A(&ca));
            let (fb, cb) = b.step();
            debug_assert_eq!(
                fa, fb,
                "equal-depth leaves share their whole path, so their flip levels agree"
            );
            fold(Crossed::B(&cb));
            (Some(ca), Some(cb))
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
        super::fold_signed_int(&mut diff, false, &a_first);
        super::fold_signed_int(&mut diff, true, &b_first);
        OpenedPair {
            a,
            b,
            diff,
            a_first,
            b_first,
        }
    }
}

/// Run the merge, generic over the question asked of the surviving
/// [`Directions`].
///
/// After each interval's sign fold, `exit` sees the surviving directions and
/// may declare the question decided — the `Break` payload carries the verdict,
/// so the earliest stop and its answer are one value (an early exit leaves the
/// direction the question does not need wherever the folded prefix left it,
/// which is why directions are never handed back early). At exhaustion `finish`
/// maps the fully-swept directions.
fn sweep<V>(
    a_bits: &BitsSlice,
    b_bits: &BitsSlice,
    exit: impl Fn(Directions) -> ControlFlow<V>,
    finish: impl FnOnce(Directions) -> V,
) -> V {
    let OpenedPair {
        mut a,
        mut b,
        mut diff,
        ..
    } = OpenedPair::open(a_bits, b_bits);
    let mut dirs = Directions::new();
    loop {
        // One fold per elementary interval: the interval starting at the sweep
        // point ends at the earlier plateau end, and `D` is constant across it.
        dirs.fold(diff.sign());
        if let ControlFlow::Break(verdict) = exit(dirs) {
            return verdict;
        }
        if a.done() && b.done() {
            return finish(dirs);
        }
        advance_diff(&mut a, &mut b, &mut diff);
    }
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
            super::fold_signed_int(net, step.negative, &step.magnitude);
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
        let k = self.cursor.read_unary().expect("canonical skyline bits");
        for _ in 0..k {
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
        let (negative, magnitude) = super::unzigzag(code);
        (
            flip,
            Step {
                negative,
                magnitude,
            },
        )
    }
}

#[cfg(test)]
mod tests;
