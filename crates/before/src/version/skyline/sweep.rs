//! The comparison sweeps over two skyline streams: order, equality, domination,
//! concurrency.
//!
//! The walk machinery — the plateau cursors and the overlay-advance law, with
//! its boundary bookkeeping — lives in [`overlay`](super::overlay). This module
//! owns the comparison algebra folded over that walk and the entry points
//! asking each question: [`causal_cmp`], [`eq`], [`le`], [`concurrent`].
//!
//! Two versions overlay into a common refinement — the *elementary intervals*,
//! the maximal spans crossing no leaf boundary of either partition — and every
//! comparison the crate asks is pointwise over that refinement: `a <= b` iff no
//! elementary interval has `a`'s height above `b`'s, `b <= a` iff none has the
//! reverse, equal iff both hold, concurrent iff neither. The sweep maintains
//! **one** running signed difference `D = height_a − height_b` on the
//! cliff-free [`Accumulator`](suanpan::Accumulator), folds `sign(D)` once per
//! elementary interval into the two surviving [`Directions`], and advances
//! whichever cursor's plateau ends first (the law's rule). Nothing recurses,
//! and no synthetic zero subtree is ever walked when one side bottoms out
//! early: a leaf is one long plateau, and the other side's boundaries are
//! consumed against it iteratively. No per-level value is saved anywhere — the
//! transient state is two path-bit stacks (one bit per open ancestor per side)
//! and the one accumulator.
//!
//! # Cost
//!
//! The cursor and fold work is [`overlay`](super::overlay)'s cost argument:
//! scan, decode, and stack work linear in the two streams' bits, arithmetic
//! priced by the codes that express the deltas. The comparison adds one
//! per-interval `sign(D)` read, amortized O(1) against the writes that preceded
//! it ([`suanpan`]'s crate docs carry both arguments). The constants are pinned
//! by the `skyline_cmp_*` rows of the resource-envelope suite
//! (`tests/meter.rs`): the flatness rows pin the per-delta cost flat across a
//! boundary-comb size doubling, and every scenario pins at zero grown stack
//! segments — comparing a deep operand against a shallow one costs the deep
//! side's *bits*, not its frames.
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
//! reached, and a decided sweep reads no more of either stream. That last
//! clause is a measured number, not just a claim: the `eq_early_exit` row of
//! the resource-envelope suite (`tests/meter.rs`) pins the sweep's touch and
//! scan readings on a first-interval-refuted pair as tail-independent
//! absolutes at two operand scales, so an exit discipline that keeps sweeping
//! a decided question fails that row while every verdict-level suite stays
//! green.
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

use crate::codec::BitsView;

use super::overlay::{advance_diff, OpenedPair, PlateauCursor};

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
pub fn causal_cmp(a: BitsView<'_>, b: BitsView<'_>) -> Option<Ordering> {
    // Clone identity decides reflexivity without a walk: one shared stored
    // buffer read through two views is bit-for-bit one stream (`Version::clone`
    // is a refcount bump), and a version compares `Equal` to itself — the
    // `order_reflexive` law in `crate::laws`. Equal streams in distinct buffers
    // still take the sweep below.
    if a.ptr_eq(&b) {
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
pub fn eq(a: BitsView<'_>, b: BitsView<'_>) -> bool {
    // Surviving to exhaustion is equality: `eq_exit` breaks on any refutation
    // before the exhaustion check runs, so reaching the finish arm IS the
    // verdict. The assertion keeps that control-flow argument loud: an exit or
    // loop change that ever admits a refuted sweep here fails debug builds at
    // the seam instead of silently re-deriving (or corrupting) the verdict.
    sweep(a, b, eq_exit, |directions| {
        debug_assert!(
            directions.le && directions.ge,
            "eq_exit breaks on refutation, so exhaustion is equality"
        );
        true
    })
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
pub fn concurrent(a: BitsView<'_>, b: BitsView<'_>) -> bool {
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
pub fn le(a: BitsView<'_>, b: BitsView<'_>) -> bool {
    sweep(
        a,
        b,
        // Domination `a <= b` stops the moment that one direction is refuted;
        // the other direction never matters.
        |directions| {
            if directions.le {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(false)
            }
        },
        // Surviving to exhaustion confirms the domination.
        |directions| directions.le,
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
pub(super) fn order_exit(directions: Directions) -> ControlFlow<Option<Ordering>> {
    if !directions.le && !directions.ge {
        ControlFlow::Break(None)
    } else {
        ControlFlow::Continue(())
    }
}

/// Equality's exit question: stop the moment either direction is refuted — any
/// nonzero difference refutes equality.
pub(super) fn eq_exit(directions: Directions) -> ControlFlow<bool> {
    if !directions.le || !directions.ge {
        ControlFlow::Break(false)
    } else {
        ControlFlow::Continue(())
    }
}

/// Run the merge, generic over the question asked of the surviving
/// [`Directions`].
///
/// After each interval's sign fold, `exit` sees the surviving directions and
/// may declare the question decided — the `Break` payload carries the verdict,
/// so the earliest stop and its answer are one value. A break carries `V`
/// rather than the directions because the direction the question ignores may
/// be stale at an early exit: only the fully-swept directions `finish` maps
/// at exhaustion are all decided.
fn sweep<V>(
    a_bits: BitsView<'_>,
    b_bits: BitsView<'_>,
    exit: impl Fn(Directions) -> ControlFlow<V>,
    finish: impl FnOnce(Directions) -> V,
) -> V {
    let OpenedPair {
        mut a,
        mut b,
        mut diff,
        ..
    } = OpenedPair::open(a_bits, b_bits);
    let mut directions = Directions::new();
    loop {
        // One fold per elementary interval: the interval starting at the sweep
        // point ends at the earlier plateau end, and `D` is constant across it.
        directions.fold(diff.sign());
        if let ControlFlow::Break(verdict) = exit(directions) {
            return verdict;
        }
        if a.done() && b.done() {
            return finish(directions);
        }
        advance_diff(&mut a, &mut b, &mut diff);
    }
}

#[cfg(test)]
mod tests;
