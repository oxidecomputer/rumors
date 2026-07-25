//! The comparison sweep over skyline streams: order, equality, domination,
//! and concurrency from one iterative merge of the two leaf sequences.
//!
//! A skyline stream lists its version's plateaus left to right: a leaf at
//! depth `d` is a constant run of width `2^-d` over the unit id interval.
//! Two versions overlay into a common refinement — the *elementary
//! intervals*, the maximal spans crossing no leaf boundary of either
//! partition — and every comparison the crate asks is pointwise over that
//! refinement: `a <= b` iff no elementary interval has `a`'s height above
//! `b`'s, `b <= a` iff none has the reverse, equal iff both hold,
//! concurrent iff neither. The sweep maintains **one** running signed
//! difference `D = height_a − height_b` on the cliff-immune [`Accum`],
//! folds `sign(D)` once per elementary interval, and advances whichever
//! cursor's plateau ends first. Nothing recurses, and no synthetic zero
//! subtree is ever walked
//! when one side bottoms out early: a leaf is one long plateau, and the
//! other side's boundaries are consumed against it iteratively. No
//! per-level value is saved anywhere — the transient state is two path-bit
//! stacks (one bit per open ancestor per side) and the one accumulator.
//!
//! # The boundary bookkeeping: which cursor advances
//!
//! The two cursors are asymmetric — their current leaves generally sit at
//! different depths, with different interval ends — and the sweep never
//! materializes an interval end as a number: an end is `depth` path bits
//! wide, so comparing two of them arithmetically at every boundary would
//! be quadratic on deep streams. Three facts about dyadic intervals
//! replace the arithmetic:
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
//!   the *same* flip level (their paths agree there), which the sweep
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
//! resource-envelope suite (`tests/meter.rs`): every topology bit of
//! either stream is read at most once (the cursors only move forward),
//! every path bit is pushed and popped at most once, and every leaf
//! payload is decoded and folded into `D` exactly once, so scan, decode,
//! and stack work are all linear in the two streams' bits. The
//! accumulator prices the arithmetic: a machine-word delta costs
//! amortized O(1) digit touches, a wide delta O(its own limbs) — paid by
//! the code the input spent to express it — and each per-interval
//! `sign(D)` is amortized O(1) against the writes that preceded it
//! ([`Accum`]'s module doc carries both arguments; the envelope suite's
//! flatness rows pin the per-delta cost
//! flat across a boundary-comb size doubling). Transient space is one
//! path bit per open ancestor per side plus the accumulator, so comparing
//! a deep operand against a shallow one costs the deep side's *bits*, not
//! its frames: the envelope rows pin every scenario at zero grown stack
//! segments.
//!
//! # Early exit
//!
//! Each entry point stops the sweep at the first interval that decides
//! its own question: [`causal_cmp`] when both directions are excluded
//! (the strict mix reading concurrent), [`eq`] when either is (any
//! nonzero `D` refutes equality), [`le`] when its one direction is (any
//! `D > 0`). A decided sweep reads no more of either stream.
//!
//! # Testing
//!
//! The stored-form comparison ([`Version`](crate::Version)'s
//! `PartialOrd`) is the verdict oracle: differential tests pin all four
//! entry points against it over the adversarial generator families,
//! arbitrary normal-form trees, organic op-trace histories, and the
//! exhaustive small scope — every ordered pair of normal-form event trees
//! to the small-scope depth, which reaches every boundary genre (aligned
//! ties, flush-right ties at unequal depths, plateau consumption, zero
//! deltas across subtree boundaries) by brute force rather than sampling.
//! The resource envelopes are the meter rows named above.

use core::cmp::Ordering;

use crate::codec::accum::Accum;
use crate::codec::{Base, BitCursor, Bits, BitsSlice, SliceCursor};
use crate::step;

use super::Encoded;

/// The causal order of the versions two skyline streams denote; `None`
/// is concurrent.
///
/// One merge over the two streams, folding the sign of the running
/// height difference per elementary interval; stops at the first strict
/// mix. Every verdict matches the stored-form comparison exactly (the
/// module doc's differential suite pins all four outcomes).
///
/// # Panics
///
/// Panics if either operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes — or
/// declares more live bits than its bytes hold.
pub fn causal_cmp(a: &Encoded, b: &Encoded) -> Option<Ordering> {
    match sweep(live(a), live(b), Mode::Order) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

/// Whether two skyline streams denote the same version.
///
/// The sweep form of equality: stops at the first elementary interval
/// whose height difference is nonzero. Canonical uniqueness makes plain
/// byte equality an equivalent test; the sweep form is the one whose
/// verdicts the differential suite pins, and it shares the other entry
/// points' cost bounds.
///
/// # Panics
///
/// Panics on a non-canonical operand or an overrunning live-bit count,
/// exactly as [`causal_cmp`] does.
///
/// Test- and meter-only: production equality is the stored forms' byte
/// equality (canonical uniqueness makes them the same test).
#[cfg(any(test, feature = "meter"))]
pub fn eq(a: &Encoded, b: &Encoded) -> bool {
    let (le, ge) = sweep(live(a), live(b), Mode::Equality);
    le && ge
}

/// Whether the versions two skyline streams denote are concurrent:
/// neither dominates the other.
///
/// [`causal_cmp`]'s `None` outcome as a predicate. The sweep exits at
/// the first strict mix, so a concurrent verdict is typically decided
/// well before either stream ends.
///
/// # Panics
///
/// Panics on a non-canonical operand or an overrunning live-bit count,
/// exactly as [`causal_cmp`] does.
///
/// Test- and meter-only: production concurrency checks go through
/// [`Version::concurrent`](crate::Version::concurrent) over the same
/// sweep.
#[cfg(any(test, feature = "meter"))]
pub fn concurrent(a: &Encoded, b: &Encoded) -> bool {
    causal_cmp(a, b).is_none()
}

/// Whether the first stream's version is dominated by the second's:
/// `a <= b` pointwise over the unit id interval.
///
/// The single-direction fold behind causal containment checks: stops at
/// the first elementary interval where `a`'s height exceeds `b`'s.
///
/// # Panics
///
/// Panics on a non-canonical operand or an overrunning live-bit count,
/// exactly as [`causal_cmp`] does.
///
/// Test- and meter-only: production ordering goes through the
/// `PartialOrd` surface over [`causal_cmp`].
#[cfg(any(test, feature = "meter"))]
pub fn le(a: &Encoded, b: &Encoded) -> bool {
    sweep(live(a), live(b), Mode::Domination).0
}

/// Borrow one operand's live bits.
///
/// # Panics
///
/// Panics if the operand declares more live bits than its bytes hold.
fn live(enc: &Encoded) -> &BitsSlice {
    super::live_bits(&enc.bytes, enc.bits)
}

/// The question a sweep answers, hence the earliest point it may stop.
#[derive(Clone, Copy)]
enum Mode {
    /// The full order: stop only when both directions are excluded (the
    /// strict mix that reads concurrent).
    Order,
    /// Equality: stop when either direction is excluded — any nonzero
    /// difference refutes it. Only [`eq`] asks it.
    #[cfg(any(test, feature = "meter"))]
    Equality,
    /// Domination `a <= b`: stop when that one direction is excluded.
    /// Only [`le`] asks it.
    #[cfg(any(test, feature = "meter"))]
    Domination,
}

impl Mode {
    /// Whether the surviving-direction flags already decide this mode's
    /// question.
    fn decided(self, le: bool, ge: bool) -> bool {
        match self {
            Mode::Order => !le && !ge,
            #[cfg(any(test, feature = "meter"))]
            Mode::Equality => !le || !ge,
            #[cfg(any(test, feature = "meter"))]
            Mode::Domination => !le,
        }
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

/// Fold one decoded leaf delta into the running difference, oriented by
/// the side its stream feeds: `a`'s height rising raises `D`, `b`'s
/// lowers it.
pub(super) fn fold(diff: &mut Accum, side: Side, negative: bool, magnitude: &Base) {
    let raises_diff = match side {
        Side::A => !negative,
        Side::B => negative,
    };
    if raises_diff {
        diff.add_base(magnitude);
    } else {
        diff.sub_base(magnitude);
    }
}

/// One cursor advance: the flip level and the leaf-to-leaf delta folded.
pub(super) struct Step {
    /// The flip level's depth, for the boundary tie test.
    pub(super) flip: usize,
    /// Whether the delta lowers this stream's height.
    pub(super) negative: bool,
    /// The delta's absolute value.
    pub(super) magnitude: Base,
}

/// Advance the overlay walk one boundary: step the deeper cursor, and the
/// other in the same step on a tie, folding every consumed delta into
/// `diff`.
///
/// Returns each side's consumed delta (`None` for a side that did not
/// step), which the emission sweep re-codes and the comparison sweep
/// discards. The tie rule is the module doc's bookkeeping: the deeper
/// side's flip level rising to or above the shallower side's depth is the
/// tie, and the two sides then close to the same flip level.
pub(super) fn advance(
    a: &mut LeafCursor<'_>,
    b: &mut LeafCursor<'_>,
    diff: &mut Accum,
) -> (Option<Step>, Option<Step>) {
    match a.depth().cmp(&b.depth()) {
        Ordering::Greater => {
            let sa = a.step(diff, Side::A);
            let sb = (sa.flip <= b.depth()).then(|| {
                let sb = b.step(diff, Side::B);
                debug_assert_eq!(
                    sa.flip, sb.flip,
                    "tied boundaries close to one shared flip level"
                );
                sb
            });
            (Some(sa), sb)
        }
        Ordering::Less => {
            let sb = b.step(diff, Side::B);
            let sa = (sb.flip <= a.depth()).then(|| {
                let sa = a.step(diff, Side::A);
                debug_assert_eq!(
                    sb.flip, sa.flip,
                    "tied boundaries close to one shared flip level"
                );
                sa
            });
            (sa, Some(sb))
        }
        Ordering::Equal => {
            let sa = a.step(diff, Side::A);
            let sb = b.step(diff, Side::B);
            debug_assert_eq!(
                sa.flip, sb.flip,
                "equal-depth leaves share their whole path, so their flip levels agree"
            );
            (Some(sa), Some(sb))
        }
    }
}

/// Run the merge, returning the surviving directions `(a <= b, b <= a)`.
///
/// The pair is truthful only for the question `mode` asks: an early exit
/// leaves the direction the mode does not need wherever the folded
/// prefix left it.
fn sweep(a_bits: &BitsSlice, b_bits: &BitsSlice, mode: Mode) -> (bool, bool) {
    let mut diff = Accum::new();
    let (mut a, a_first) = LeafCursor::open(a_bits);
    let (mut b, b_first) = LeafCursor::open(b_bits);
    diff.add_base(&a_first);
    diff.sub_base(&b_first);
    let (mut le, mut ge) = (true, true);
    loop {
        // One fold per elementary interval: the interval starting at the
        // sweep point ends at the earlier plateau end, and `D` is
        // constant across it.
        match diff.sign() {
            Ordering::Greater => le = false,
            Ordering::Less => ge = false,
            Ordering::Equal => {}
        }
        if mode.decided(le, ge) || (a.done() && b.done()) {
            return (le, ge);
        }
        advance(&mut a, &mut b, &mut diff);
    }
}

/// A cursor at the current leaf of one skyline stream.
///
/// Holds the sequential bit cursor and the root-to-leaf path — one bit
/// per open ancestor: `false` inside its left child (the right subtree
/// is still pending in the stream), `true` inside its right. The path is
/// the only per-depth state; no height, base, or node is retained, which
/// is what keeps a sweep's transient linear in depth *bits*. The
/// comparison and emission sweeps share the cursor: both consume decoded
/// leaf deltas through [`fold`], and emission additionally re-codes
/// them.
pub(super) struct LeafCursor<'a> {
    cursor: SliceCursor<'a>,
    /// Root-to-leaf branch directions, root first.
    path: Bits,
    /// The stream's live bit length; the cursor reaching it is
    /// exhaustion (the current leaf is the stream's last).
    len: usize,
}

impl<'a> LeafCursor<'a> {
    /// Open a stream at its first leaf, returning the cursor and that
    /// leaf's absolute height (later leaves carry zigzag deltas, which
    /// [`step`](Self::step) decodes instead).
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    pub(super) fn open(bits: &'a BitsSlice) -> (Self, Base) {
        let mut this = LeafCursor {
            cursor: SliceCursor::new(bits, 0),
            path: Bits::new(),
            len: bits.len(),
        };
        let first = this.descend();
        (this, first)
    }

    /// The current leaf's depth: its plateau has width `2^-depth`.
    pub(super) fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether the current leaf is the stream's last (its plateau ends
    /// at the unit interval's right edge).
    pub(super) fn done(&self) -> bool {
        self.cursor.position() == self.len
    }

    /// Advance past the current leaf to the next, folding the leaf-to-
    /// leaf zigzag delta into `diff` on `side` and returning it with the
    /// flip level's depth for the caller's tie test.
    ///
    /// Pops the trailing right-branch levels (each ancestor's subtree
    /// the consumed leaf completed), steps the deepest left-branch
    /// level to its right child, and descends to the next leaf.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding. Never
    /// called on a final leaf: a sweep stops when both cursors are
    /// done, and the module doc's bookkeeping shows a final leaf is
    /// never the advanced side before then.
    pub(super) fn step(&mut self, diff: &mut Accum, side: Side) -> Step {
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
        fold(diff, side, negative, &magnitude);
        Step {
            flip,
            negative,
            magnitude,
        }
    }

    /// Descend from the cursor to the next leaf in preorder, extending
    /// the path with a left branch per internal node passed.
    ///
    /// Returns the leaf's payload code undecoded: absolute for the
    /// stream's first leaf, zigzag for every later one.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    fn descend(&mut self) -> Base {
        loop {
            step!();
            let internal = self.cursor.read_bit().expect("canonical skyline bits");
            if !internal {
                break;
            }
            self.path.push(false);
        }
        // The cursor's own `read_int`, so the payload decode takes the
        // word-wise window fast path; the scan meter records the same
        // `2k + 1` bits either way.
        self.cursor.read_int().expect("canonical skyline bits")
    }
}

#[cfg(test)]
mod tests;
