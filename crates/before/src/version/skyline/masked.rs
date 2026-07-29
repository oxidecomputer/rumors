//! The masked comparison co-walk: causal order between *projected*
//! skylines, decided without materializing any projection.
//!
//! A projection `v / p` is the version's step function gated by the
//! party's 0-or-1 ownership landscape: the height where `p` owns, zero
//! elsewhere. Comparing a projection therefore never needs the projected
//! stream as an object — it is a pointwise question over the overlay of
//! up to four packed streams (each side's event stream plus its optional
//! id mask), and this module answers it in one fused merge, the
//! [`sweep`](super::sweep) machinery generalized from two cursors to the
//! full cursor set.
//!
//! # The walk
//!
//! One `LeafCursor` per event stream and one `IdLeafCursor` per mask
//! (an unmasked side simply has no id cursor — its ownership is
//! everywhere). All current leaves and regions contain the sweep point,
//! so they nest by depth, and the comparison sweep's boundary bookkeeping
//! generalizes verbatim: the deepest cursor advances, and every other
//! cursor whose depth reaches the advanced cursor's flip level advances
//! in the same step (tied boundaries close to one shared flip level; the
//! walk debug-asserts it at every tie). Nothing recurses, and the
//! transient state is the cursors' path bits plus three accumulators.
//!
//! # The integrators
//!
//! Per elementary interval the verdict needs the sign of
//! `h′_a − h′_b`, where `h′` is the projected (gated) height. The walk
//! never materializes a projected height; it maintains, on the
//! cliff-immune [`Accumulator`], exactly the running quantities the four
//! ownership cases read:
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
//! The single-owner reads are the trichotomy's zero-check: distinguishing
//! `=` from `<` requires knowing whether the *other* operand has positive
//! height outside the region, and the running `h` accumulator answers it
//! without walking any skipped subtree twice. Each event delta folds into
//! at most two accumulators (a constant factor over the pair sweep), and
//! each per-interval sign read is amortized O(1) digit touches against
//! the writes that preceded it ([`suanpan`]'s argument, unchanged) — a
//! mask toggle adds a read, never a re-walk.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the `masked_cmp_*` rows of the
//! resource-envelope suite (`tests/meter.rs`): every topology bit of
//! every stream is read at most once, every path bit pushed and popped at
//! most once, every event payload decoded once and folded into at most
//! two accumulators, and every id region visited once — so scan, decode,
//! stack, and fold work are all linear in the operand streams' bits,
//! `O(|v| + |p| + |w|)` for one mask and
//! `O(|v₁| + |p₁| + |v₂| + |p₂|)` for two. The per-interval sign reads
//! ride the accumulator's amortized-O(1) collapse; the correlated
//! families (mask boundaries riding the other operand's carry-boundary
//! drift) are the committed adversaries holding that claim to its
//! envelope.
//!
//! # Early exit
//!
//! Each entry point stops at the first interval that decides its
//! question, exactly as the pair sweep does: [`causal_cmp`] when both
//! directions are excluded, [`eq`] when either is.
//!
//! # Testing
//!
//! The materialized form is the behavioral oracle: `view ⋚ w` must equal
//! `view.to_version() ⋚ w` on every input, which the differential laws in
//! [`crate::laws`] pin over generated, organic, and fuzzed populations
//! (three- and four-stream forms, plus the seed-mask coherence law), and
//! the public-surface proptests beside [`OwnVersion`](crate::OwnVersion)
//! drive against the recursive oracle's composed projection-and-compare.
//! The resource envelopes are the meter rows named above.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::BitsSlice;

use super::query::IdLeafCursor;
use super::sweep::{fold, LeafCursor, PlateauCursor, Side};

/// The causal order of two projected skylines, `None` for concurrent.
///
/// `a_mask`/`b_mask` are the sides' packed id streams; a side without a
/// mask compares its whole skyline. One fused merge over every operand
/// stream; no projection is materialized.
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
    match Walk::open(a, a_mask, b, b_mask).run(Mode::Order) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

/// Whether two projected skylines denote the same version.
///
/// Semantic equality of the projections (the gated step functions agree
/// pointwise), decided by the same fused merge as [`causal_cmp`] with the
/// equality mode's earlier exit: any nonzero projected difference refutes
/// it. No byte shortcut exists — the projections are never materialized,
/// so there are no canonical bytes to compare.
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
    let (le, ge) = Walk::open(a, a_mask, b, b_mask).run(Mode::Equality);
    le && ge
}

/// The question a walk answers, hence the earliest point it may stop.
#[derive(Clone, Copy)]
enum Mode {
    /// The full order: stop only when both directions are excluded.
    Order,
    /// Equality: stop when either direction is excluded.
    Equality,
}

impl Mode {
    /// Whether the surviving-direction flags already decide this mode's
    /// question.
    fn decided(self, le: bool, ge: bool) -> bool {
        match self {
            Mode::Order => !le && !ge,
            Mode::Equality => !le || !ge,
        }
    }
}

/// The cursor set and integrators of one masked comparison.
struct Walk<'a> {
    /// The left event stream's leaf cursor.
    a: LeafCursor<'a>,
    /// The left side's id cursor; `None` means the side is unmasked
    /// (owned everywhere).
    am: Option<IdLeafCursor<'a>>,
    /// The right event stream's leaf cursor.
    b: LeafCursor<'a>,
    /// The right side's id cursor, as `am`.
    bm: Option<IdLeafCursor<'a>>,
    /// `D = h_a − h_b`, the both-owned intervals' sign source.
    diff: Accumulator,
    /// `h_a`, maintained only when `b` is masked (the only case that
    /// reads it: `a` owned alone compares `h_a` against zero).
    ha: Option<Accumulator>,
    /// `h_b`, maintained only when `a` is masked, dually.
    hb: Option<Accumulator>,
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
        let (a, a_first) = LeafCursor::open(a_bits);
        let (b, b_first) = LeafCursor::open(b_bits);
        let mut diff = Accumulator::new();
        diff.add_magnitude(&a_first);
        diff.sub_magnitude(&b_first);
        // Each height integrator exists only if the *other* side is
        // masked: no ownership case reads it otherwise, so feeding it
        // would be pure waste.
        let ha = b_mask.map(|_| {
            let mut ha = Accumulator::new();
            ha.add_magnitude(&a_first);
            ha
        });
        let hb = a_mask.map(|_| {
            let mut hb = Accumulator::new();
            hb.add_magnitude(&b_first);
            hb
        });
        Walk {
            a,
            am: a_mask.map(IdLeafCursor::open),
            b,
            bm: b_mask.map(IdLeafCursor::open),
            diff,
            ha,
            hb,
        }
    }

    /// Run the merge, returning the surviving directions
    /// `(a′ <= b′, b′ <= a′)` over the projected skylines.
    ///
    /// The pair is truthful only for the question `mode` asks: an early
    /// exit leaves the direction the mode does not need wherever the
    /// folded prefix left it.
    fn run(mut self, mode: Mode) -> (bool, bool) {
        let (mut le, mut ge) = (true, true);
        loop {
            // One fold per elementary interval: the ownership case picks
            // which integrator's sign is the projected difference's.
            let owned_a = self.am.as_ref().is_none_or(IdLeafCursor::owned);
            let owned_b = self.bm.as_ref().is_none_or(IdLeafCursor::owned);
            let sign = match (owned_a, owned_b) {
                (true, true) => self.diff.sign(),
                (true, false) => {
                    // `h′_b = 0`: the interval's sign is `sign(h_a)`,
                    // the trichotomy's zero-check on the unmasked side.
                    let s = self.ha.as_mut().expect("a masked `b` maintains h_a").sign();
                    debug_assert_ne!(s, Ordering::Less, "heights are nonnegative");
                    s
                }
                (false, true) => {
                    let s = self.hb.as_mut().expect("a masked `a` maintains h_b").sign();
                    debug_assert_ne!(s, Ordering::Less, "heights are nonnegative");
                    s.reverse()
                }
                (false, false) => Ordering::Equal, // 0 vs 0
            };
            match sign {
                Ordering::Greater => le = false,
                Ordering::Less => ge = false,
                Ordering::Equal => {}
            }
            if mode.decided(le, ge) || self.done() {
                return (le, ge);
            }
            self.advance();
        }
    }

    /// Whether every operand stream is at its final leaf or region.
    ///
    /// Canonical streams all tile the unit interval, so they exhaust
    /// together, exactly as in the pair sweep.
    fn done(&self) -> bool {
        self.a.done()
            && self.b.done()
            && self.am.as_ref().is_none_or(IdLeafCursor::done)
            && self.bm.as_ref().is_none_or(IdLeafCursor::done)
    }

    /// Advance the overlay one boundary: the deepest cursor steps, and
    /// every other cursor whose depth reaches the flip level steps in the
    /// same round (its boundary tied).
    ///
    /// The pair sweep's tie rule at full arity: overlapping dyadic
    /// intervals nest, so the deepest cursor's plateau ends first, and a
    /// shallower cursor's end ties exactly when the flip level rises to
    /// or above its depth. A cursor at depth zero (a single-leaf stream,
    /// or no mask at all) never steps: a flip level is at least one.
    fn advance(&mut self) {
        let depths = self.depths();
        let deepest = (0..4)
            .max_by_key(|&i| depths[i])
            .expect("four cursor slots");
        let flip = self.step(deepest);
        for (i, &depth) in depths.iter().enumerate() {
            if i != deepest && depth >= flip {
                let tied = self.step(i);
                debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
            }
        }
    }

    /// Every cursor slot's current depth; an absent mask reads zero (one
    /// all-owned region over the whole interval, which never steps).
    fn depths(&self) -> [usize; 4] {
        [
            self.a.depth(),
            self.am.as_ref().map_or(0, IdLeafCursor::depth),
            self.b.depth(),
            self.bm.as_ref().map_or(0, IdLeafCursor::depth),
        ]
    }

    /// Step one cursor slot past its current leaf or region, folding an
    /// event delta into the integrators that watch it, and return the
    /// flip level for the caller's tie test.
    fn step(&mut self, slot: usize) -> usize {
        match slot {
            0 => {
                let (flip, step) = self.a.step();
                fold(&mut self.diff, Side::A, step.negative, &step.magnitude);
                if let Some(ha) = &mut self.ha {
                    fold(ha, Side::A, step.negative, &step.magnitude);
                }
                flip
            }
            1 => self
                .am
                .as_mut()
                .expect("slot 1 is the present a-mask")
                .step(),
            2 => {
                let (flip, step) = self.b.step();
                fold(&mut self.diff, Side::B, step.negative, &step.magnitude);
                if let Some(hb) = &mut self.hb {
                    // `h_b` accumulates positively: the side orientation
                    // belongs to `D` alone.
                    fold(hb, Side::A, step.negative, &step.magnitude);
                }
                flip
            }
            3 => self
                .bm
                .as_mut()
                .expect("slot 3 is the present b-mask")
                .step(),
            _ => unreachable!("four cursor slots"),
        }
    }
}
