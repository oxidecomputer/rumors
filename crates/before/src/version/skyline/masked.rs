//! The masked comparison co-walk: causal order between *projected* skylines,
//! decided without materializing any projection.
//!
//! A projection `v / p` is the version's step function gated by the party's
//! 0-or-1 ownership landscape: the height where `p` owns, zero elsewhere.
//! Comparing a projection therefore never needs the projected stream as an
//! object — it is a pointwise question over the overlay of up to four packed
//! streams (each side's event stream plus its optional id mask), and this
//! module answers it in one fused merge, the [`overlay`](super::overlay)
//! machinery generalized from two cursors to the full cursor set.
//!
//! # The walk
//!
//! One `LeafCursor` per event stream and one `IdLeafCursor` per mask (an
//! unmasked side simply has no id cursor — its ownership is everywhere). All
//! current leaves and regions contain the sweep point, so they nest by depth,
//! and the walk advances by the overlay-advance law at arity four
//! ([`advance_set`]; this walk contributes only its slot roster and what each
//! slot's step folds). Nothing recurses, and the transient state is the
//! cursors' path bits plus three accumulators.
//!
//! # The integrators
//!
//! Per elementary interval the verdict needs the sign of `h′_a − h′_b`, where
//! `h′` is the projected (gated) height. The walk never materializes a
//! projected height; it maintains, on the cliff-immune [`Accumulator`], exactly
//! the running quantities the four ownership cases read:
//!
//! - `D = h_a − h_b`, fed by both event streams (the comparison sweep's
//!   own difference): read where **both** sides are owned.
//! - `h_a`, fed by `a`'s deltas alone, maintained only when `b` carries a
//!   mask: read where only `a` is owned (`h′_b = 0`, so the interval's
//!   sign is `sign(h_a)`).
//! - `h_b`, dually, maintained only when `a` carries a mask: read
//!   (reversed) where only `b` is owned.
//! - Neither side owned: the interval compares `0` with `0` — no read.
//!
//! The single-owner reads are the comparison trichotomy's (`<`/`=`/`>`)
//! zero-check: distinguishing `=`
//! from `<` requires knowing whether the *other* operand has positive height
//! outside the region, and the running `h` accumulator answers it without
//! walking any skipped subtree twice. Each event delta folds into at most two
//! accumulators (a constant factor over the pair sweep), and each per-interval
//! sign read is amortized O(1) digit touches against the writes that preceded
//! it ([`suanpan`]'s argument, unchanged) — a mask toggle adds a read, never a
//! re-walk.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the `masked_cmp_*` rows of the
//! resource-envelope suite (`tests/meter.rs`): every topology bit of every
//! stream is read at most once, every path bit pushed and popped at most once,
//! every event payload decoded once and folded into at most two accumulators,
//! and every id region visited once — so scan, decode, stack, and fold work are
//! all linear in the operand streams' bits, `O(|v| + |p| + |w|)` for one mask
//! and `O(|v₁| + |p₁| + |v₂| + |p₂|)` for two. The per-interval sign reads ride
//! the accumulator's amortized-O(1) collapse; the correlated families (mask
//! boundaries riding the other operand's carry-boundary drift) are the
//! committed adversaries holding that claim to its envelope.
//!
//! # Early exit
//!
//! Each entry point stops at the first interval that decides its question,
//! exactly as the pair sweep does: [`causal_cmp`] when both directions are
//! excluded, [`eq`] when either is.
//!
//! # Testing
//!
//! The materialized form is the behavioral oracle: `view ⋚ w` must equal
//! `view.to_version() ⋚ w` on every input, which the differential laws in
//! [`crate::laws`] pin over generated, organic, and fuzzed populations (three-
//! and four-stream forms, plus the seed-mask coherence law), and the
//! public-surface proptests beside [`OwnVersion`](crate::OwnVersion) drive
//! against the recursive oracle's composed projection-and-compare. The resource
//! envelopes are the meter rows named above.

use core::cmp::Ordering;
use core::ops::ControlFlow;

use suanpan::Accumulator;

use crate::codec::BitsSlice;

use super::overlay::{
    advance_set, fold, CursorSet, IdLeafCursor, LeafCursor, OpenedPair, PlateauCursor, Side,
};
use super::signed::Sign;
use super::sweep::{eq_exit, order_exit, Directions};

/// The causal order of two projected skylines, `None` for concurrent.
///
/// `a_mask`/`b_mask` are the sides' packed id streams; a side without a mask
/// compares its whole skyline. One fused merge over every operand stream; no
/// projection is materialized.
///
/// # Panics
///
/// Panics if an event operand is not a canonical skyline stream or a mask
/// operand is not a canonical packed id.
pub fn causal_cmp(
    a: &BitsSlice,
    a_mask: Option<&BitsSlice>,
    b: &BitsSlice,
    b_mask: Option<&BitsSlice>,
) -> Option<Ordering> {
    // At exhaustion every surviving combination is a verdict
    // ([`Directions::relation`]'s map).
    Walk::open(a, a_mask, b, b_mask).run(order_exit, Directions::relation)
}

/// Whether two projected skylines denote the same version.
///
/// Semantic equality of the projections (the gated step functions agree
/// pointwise), decided by the same fused merge as [`causal_cmp`] with
/// equality's earlier exit: any nonzero projected difference refutes it. No
/// byte shortcut exists — the projections are never materialized, so there are
/// no canonical bytes to compare.
///
/// # Panics
///
/// Panics on a non-canonical operand, exactly as [`causal_cmp`] does.
pub fn eq(
    a: &BitsSlice,
    a_mask: Option<&BitsSlice>,
    b: &BitsSlice,
    b_mask: Option<&BitsSlice>,
) -> bool {
    // Surviving both directions to exhaustion is equality.
    Walk::open(a, a_mask, b, b_mask).run(eq_exit, |directions| directions.le && directions.ge)
}

/// The cursor set and integrators of one masked comparison.
struct Walk<'a> {
    /// The left event stream's leaf cursor.
    a: LeafCursor<'a>,
    /// The left side's id cursor; `None` means the side is unmasked
    /// (owned everywhere).
    a_mask: Option<IdLeafCursor<'a>>,
    /// The right event stream's leaf cursor.
    b: LeafCursor<'a>,
    /// The right side's id cursor, as `a_mask`.
    b_mask: Option<IdLeafCursor<'a>>,
    /// `D = h_a − h_b`, the both-owned intervals' sign source.
    diff: Accumulator,
    /// `h_a`, maintained only when `b` is masked (the only case that reads it:
    /// `a` owned alone compares `h_a` against zero).
    height_a: Option<Accumulator>,
    /// `h_b`, maintained only when `a` is masked, dually.
    height_b: Option<Accumulator>,
}

/// The walk's cursor slots, named for [`CursorSet`]'s numbered vocabulary.
impl Walk<'_> {
    /// The left event stream's slot.
    const A: usize = 0;
    /// The left mask's slot.
    const A_MASK: usize = 1;
    /// The right event stream's slot.
    const B: usize = 2;
    /// The right mask's slot.
    const B_MASK: usize = 3;
}

impl<'a> Walk<'a> {
    /// Open every operand stream at its first leaf or region and seed the
    /// integrators with the two absolute first heights.
    fn open(
        a_bits: &'a BitsSlice,
        a_mask: Option<&'a BitsSlice>,
        b_bits: &'a BitsSlice,
        b_mask: Option<&'a BitsSlice>,
    ) -> Walk<'a> {
        let OpenedPair {
            a,
            b,
            diff,
            a_first,
            b_first,
        } = OpenedPair::open(a_bits, b_bits);
        // Each height integrator exists only if the *other* side is masked: no
        // ownership case reads it otherwise, so feeding it would be pure waste.
        let height_a = b_mask.map(|_| {
            let mut height_a = Accumulator::new();
            super::signed::fold_signed_int(&mut height_a, Sign::Positive, &a_first);
            height_a
        });
        let height_b = a_mask.map(|_| {
            let mut height_b = Accumulator::new();
            super::signed::fold_signed_int(&mut height_b, Sign::Positive, &b_first);
            height_b
        });
        Walk {
            a,
            a_mask: a_mask.map(IdLeafCursor::open),
            b,
            b_mask: b_mask.map(IdLeafCursor::open),
            diff,
            height_a,
            height_b,
        }
    }

    /// Run the merge over the projected skylines, generic over the question
    /// asked of the surviving [`Directions`] (here `a′ <= b′`, `b′ <= a′`: the
    /// projected heights).
    ///
    /// After each interval's sign fold, `exit` sees the surviving directions
    /// and may declare the question decided — the `Break` payload carries the
    /// verdict, so the earliest stop and its answer are one value (an early
    /// exit leaves the direction the question does not need wherever the folded
    /// prefix left it, which is why directions are never handed back early). At
    /// exhaustion `finish` maps the fully-swept directions.
    fn run<V>(
        mut self,
        exit: impl Fn(Directions) -> ControlFlow<V>,
        finish: impl FnOnce(Directions) -> V,
    ) -> V {
        let mut directions = Directions::new();
        loop {
            // One fold per elementary interval: the ownership case picks which
            // integrator's sign is the projected difference's.
            let owned_a = self.a_mask.as_ref().is_none_or(IdLeafCursor::owned);
            let owned_b = self.b_mask.as_ref().is_none_or(IdLeafCursor::owned);
            let sign = match (owned_a, owned_b) {
                (true, true) => self.diff.sign(),
                (true, false) => {
                    // `h′_b = 0`: the interval's sign is `sign(h_a)`, the
                    // trichotomy's zero-check on the unmasked side.
                    let height_sign = self
                        .height_a
                        .as_mut()
                        .expect("a masked `b` maintains h_a")
                        .sign();
                    debug_assert_ne!(height_sign, Ordering::Less, "heights are nonnegative");
                    height_sign
                }
                (false, true) => {
                    let height_sign = self
                        .height_b
                        .as_mut()
                        .expect("a masked `a` maintains h_b")
                        .sign();
                    debug_assert_ne!(height_sign, Ordering::Less, "heights are nonnegative");
                    height_sign.reverse()
                }
                (false, false) => Ordering::Equal, // 0 vs 0
            };
            directions.fold(sign);
            if let ControlFlow::Break(verdict) = exit(directions) {
                return verdict;
            }
            if self.done() {
                return finish(directions);
            }
            self.advance();
        }
    }

    /// Whether every operand stream is at its final leaf or region.
    ///
    /// Canonical streams all tile the unit interval, so they exhaust together,
    /// exactly as in the pair sweep.
    fn done(&self) -> bool {
        self.a.done()
            && self.b.done()
            && self.a_mask.as_ref().is_none_or(PlateauCursor::done)
            && self.b_mask.as_ref().is_none_or(PlateauCursor::done)
    }

    /// The deepest current depth among every cursor slot but `slot`: the block
    /// consume's bound — a boundary whose flip level exceeds it is crossed by
    /// `slot`'s cursor alone.
    fn others_deepest(&self, slot: usize) -> usize {
        self.priority()
            .filter(|&other| other != slot)
            .map(|other| self.depth(other))
            .max()
            .expect("the walk has more than one cursor slot")
    }

    /// Consume every boundary run the verdict cannot see, as blocks.
    ///
    /// While a masked side's current region is unowned and its event cursor's
    /// next flip level sits strictly below every other cursor's depth, the
    /// boundaries it would cross are its own alone (deeper than every plateau
    /// end, so no tie — the flip bound implies the cursor is strictly deepest)
    /// and they subdivide intervals whose sign every ownership case ignores:
    /// the side's projection is constantly zero there, and no other integrator
    /// source moves. The run is consumed in one block
    /// ([`LeafCursor::skip_deeper`]), its net movement folded once into the
    /// integrators that watch the side — value-identical to the per-boundary
    /// folds, with the duplicate interval signs never re-folded (folding an
    /// unchanged sign is the identity on the surviving directions).
    ///
    /// A block can consume a side to exhaustion, so the caller re-checks
    /// [`done`](Self::done) before applying the advance law.
    fn block_skip(&mut self) {
        loop {
            let a_bound = self.others_deepest(Self::A);
            if self.a_mask.as_ref().is_some_and(|mask| !mask.owned())
                && self.a.peek_flip() > a_bound
            {
                let mut net = Accumulator::new();
                self.a.skip_deeper(a_bound, &mut net);
                self.diff.add_accum(&net);
                if let Some(height_a) = &mut self.height_a {
                    height_a.add_accum(&net);
                }
                continue;
            }
            let b_bound = self.others_deepest(Self::B);
            if self.b_mask.as_ref().is_some_and(|mask| !mask.owned())
                && self.b.peek_flip() > b_bound
            {
                let mut net = Accumulator::new();
                self.b.skip_deeper(b_bound, &mut net);
                self.diff.sub_accum(&net);
                if let Some(height_b) = &mut self.height_b {
                    height_b.add_accum(&net);
                }
                continue;
            }
            return;
        }
    }

    /// Advance the overlay one boundary by the overlay-advance law
    /// ([`advance_set`]), after consuming any verdict-invisible runs as blocks.
    fn advance(&mut self) {
        self.block_skip();
        if self.done() {
            // A block consumed the last unowned run; the final interval's sign
            // folds in the caller's next round.
            return;
        }
        advance_set(self);
    }
}

/// The masked walk's slot roster for the overlay-advance law ([`advance_set`]).
///
/// Priority `[B_MASK, B, A_MASK, A]`: among equally-deep cursors the pick falls
/// on the later operand's streams first, masks before events — an arbitrary
/// order, pinned by the committed `masked_cmp_*` readings (`tests/meter.rs`).
/// What `step` shows is what a reorder may move: the mask slots fold nothing
/// and the height integrators are per-side, so the only accumulator two slots
/// share is `diff` (`A` and `B` both fold into it) — reordering `A` relative to
/// `B` changes `diff`'s write sequence and therefore moves committed readings
/// (a deliberate re-pin event), while any other reorder moves nothing. Every
/// fold is a commutative sum, so no order changes a verdict.
impl CursorSet for Walk<'_> {
    fn priority(&self) -> impl Iterator<Item = usize> + Clone + 'static {
        [Self::B_MASK, Self::B, Self::A_MASK, Self::A].into_iter()
    }

    /// An absent mask reads zero: one all-owned region over the whole
    /// interval, which never steps.
    fn depth(&self, slot: usize) -> usize {
        match slot {
            Self::A => self.a.depth(),
            Self::A_MASK => self.a_mask.as_ref().map_or(0, PlateauCursor::depth),
            Self::B => self.b.depth(),
            Self::B_MASK => self.b_mask.as_ref().map_or(0, PlateauCursor::depth),
            _ => unreachable!("four cursor slots"),
        }
    }

    /// An event slot's step folds its delta into the integrators that watch
    /// that side; a mask slot's step folds nothing.
    ///
    /// The watchers are `diff` always, plus the side's height integrator when
    /// present. A mask crossing carries no delta — ownership is per-region
    /// state read between boundaries.
    fn step(&mut self, slot: usize) -> usize {
        match slot {
            Self::A => {
                let (flip, step) = self.a.step();
                fold(&mut self.diff, Side::A, step.sign, &step.magnitude);
                if let Some(height_a) = &mut self.height_a {
                    // A height integrator accumulates its own side plainly:
                    // the side orientation belongs to `D` alone.
                    super::signed::fold_signed_int(height_a, step.sign, &step.magnitude);
                }
                flip
            }
            Self::A_MASK => {
                self.a_mask
                    .as_mut()
                    .expect("an absent mask reads depth zero and never steps")
                    .step()
                    .0
            }
            Self::B => {
                let (flip, step) = self.b.step();
                fold(&mut self.diff, Side::B, step.sign, &step.magnitude);
                if let Some(height_b) = &mut self.height_b {
                    // A height integrator accumulates its own side plainly:
                    // the side orientation belongs to `D` alone.
                    super::signed::fold_signed_int(height_b, step.sign, &step.magnitude);
                }
                flip
            }
            Self::B_MASK => {
                self.b_mask
                    .as_mut()
                    .expect("an absent mask reads depth zero and never steps")
                    .step()
                    .0
            }
            _ => unreachable!("four cursor slots"),
        }
    }
}
