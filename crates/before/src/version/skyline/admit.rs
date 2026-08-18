//! The admission walk: strictly parse one skyline stream while proving, in the
//! same pass, that its version dominates a given canonical stream.
//!
//! The span wire form is two concatenated version streams whose pair must
//! satisfy `lo <= hi`. Composed from the standalone pieces, loading one costs a
//! strict parse per component plus a comparison sweep — the second stream
//! scanned twice, its payloads decoded twice, and two accumulators run over it
//! (the validator's running height, then the comparison's running difference).
//! This module fuses the second component's parse with the pair validation: one
//! walk over the untrusted stream, co-swept against the already-validated `lo`,
//! maintaining only the comparison's difference.
//!
//! # Why the validator's height accumulator disappears
//!
//! The standalone validator ([`validate_from`](super::validate::validate_from))
//! carries a running leaf height for its nonnegativity condition. Here the
//! dominance verdict subsumes it: on every elementary interval an accepted walk
//! holds `height_hi >= height_lo >= 0` — the left inequality is the verdict
//! itself, the right is `lo`'s own canonicality, and the chain binds pointwise
//! because this walk reads a sign on *every* elementary interval (nothing here
//! block-skips) — so a stream whose height dips negative is always also
//! non-dominating, and both facts reject as the same [`Decode::NotCanonical`]
//! genre (a composite no encode produces). The fusion's saving is therefore
//! structural, not bookkeeping: the walk runs one accumulator where
//! parse-then-compare runs two, and the touch-meter pins in `tests/meter.rs`
//! hold the admission decode to exactly the
//! standalone-`lo`-parse-plus-comparison traffic, which a parse-then-compare
//! spelling cannot read (it pays the second validation's folds on top).
//!
//! # The walk
//!
//! [`LeafCursor`] walks `lo` (canonical by the caller's contract);
//! [`CheckedCursor`] walks the untrusted stream, enforcing the validator's
//! remaining obligations on the same reads — truncation as the cursor's own
//! read errors, minimal topology as each internal node closes. The
//! overlay-advance law is restated at this cursor pair — restated rather than
//! reused, because the generic law is infallible and the checked side's
//! crossings are `Result`s — with the same step and fold order as
//! [`advance_diff`](super::overlay::advance_diff), so the accumulator's write
//! sequence — and with it the committed touch-meter readings — is the pair
//! sweep's exactly. Once dominance is refuted the verdict is fixed, so `lo`'s
//! cursor and the difference are dropped and the walk completes the strict
//! parse alone: a structural defect later in the stream still reports its own
//! genre.
//!
//! Nothing recurses: the transient state is the two cursors' bit stacks and the
//! one accumulator, exactly the sweep's shape.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{BitCursor, BitsMut, BitsView, Int};
use crate::error::Decode;

use super::overlay::{fold, LeafCursor, PlateauCursor, Side, Step};
use super::signed::{unzigzag, Sign};

/// A validating leaf cursor over one untrusted skyline stream.
///
/// [`LeafCursor`]'s plateau vocabulary — depth, done, step — with the strict
/// validator's obligations folded into the same reads: truncation surfaces as
/// the cursor's own errors, and minimal topology (no collapsible sibling pair)
/// is checked as each internal node closes. Height nonnegativity is
/// deliberately *not* checked here; the admission verdict subsumes it (the
/// module doc's argument).
///
/// Two parallel per-ancestor bit stacks ride the walk: the branch path (the
/// advance law's tie test, as [`LeafCursor`]'s), and the validator's
/// left-was-leaf bits (what the sibling-collapse check reads at each close).
/// `open_lefts` counts the path's left branches, so exhaustion — the tree's
/// root completing — is an O(1) question where the path itself would need a
/// full scan.
struct CheckedCursor<'a, C> {
    cursor: &'a mut C,
    /// Root-to-leaf branch directions, root first (`false`: inside the left
    /// child, its right sibling still pending in the stream).
    path: BitsMut,
    /// Per open ancestor: whether its completed left child was a leaf (a
    /// placeholder `false` until that child completes).
    left_was_leaf: BitsMut,
    /// The count of `false` bits in `path`: zero exactly when the current
    /// leaf's plateau ends at the unit interval's right edge — the tree is
    /// whole and the stream's bits end here.
    open_lefts: usize,
    /// Whether the current leaf's payload code was zero — the collapsible-pair
    /// check's right-child half. Never read for the first leaf (preorder puts
    /// it leftmost, so it is no ancestor's right child).
    last_delta_zero: bool,
}

impl<'a, C: BitCursor> CheckedCursor<'a, C>
where
    Decode: From<C::Error>,
{
    /// Open the stream at its first leaf: the descent to it, and the leaf's
    /// absolute height code.
    fn open(cursor: &'a mut C) -> Result<(Self, Int), Decode> {
        let mut this = CheckedCursor {
            cursor,
            path: BitsMut::new(),
            left_was_leaf: BitsMut::new(),
            open_lefts: 0,
            last_delta_zero: false,
        };
        let first = this.descend()?;
        Ok((this, first))
    }

    /// Descend to the next leaf in preorder, opening the internal nodes on the
    /// way: [`LeafCursor`]'s descent with the reads fallible and the
    /// validator's placeholder bits pushed alongside the path.
    fn descend(&mut self) -> Result<Int, Decode> {
        let internal_nodes = self.cursor.read_unary()?;
        for _ in 0..internal_nodes {
            self.path.push(false);
            self.left_was_leaf.push(false); // placeholder until the left child completes
        }
        self.open_lefts += internal_nodes;
        self.cursor.read_int()
    }

    /// The current leaf's depth: its plateau has width `2^-depth`.
    fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether the current leaf completes the tree (see `open_lefts`).
    fn done(&self) -> bool {
        self.open_lefts == 0
    }

    /// Close one ancestor whose right child just completed: pop its left
    /// child's kind from the parallel stack and run the validator's
    /// collapsible-pair check.
    ///
    /// An internal node whose two children are leaves with a zero right delta
    /// is the shape minimal topology forbids. The closed pair then reads as an
    /// internal subtree for the next close up (`is_leaf`/`zero_delta`
    /// cleared).
    fn close_ancestor(&mut self, is_leaf: &mut bool, zero_delta: &mut bool) -> Result<(), Decode> {
        let left_was_leaf = self
            .left_was_leaf
            .pop()
            .expect("the parallel stacks hold one bit each per open ancestor");
        if left_was_leaf && *is_leaf && *zero_delta {
            return Err(Decode::NotCanonical); // a collapsible sibling pair
        }
        *is_leaf = false;
        *zero_delta = false;
        Ok(())
    }

    /// Advance past the current leaf: the flip level for the advance law's tie
    /// test, and the crossed boundary's delta.
    ///
    /// Every ancestor the consumed leaf completes closes here, through
    /// [`close_ancestor`](Self::close_ancestor)'s collapsible-pair check.
    ///
    /// Never called on a done cursor; the walk asks first.
    fn step(&mut self) -> Result<(usize, Step), Decode> {
        // The consumed leaf completes one subtree per popped right branch;
        // `is_leaf`/`zero_delta` describe the completed subtree (the leaf
        // itself on the first iteration).
        let mut is_leaf = true;
        let mut zero_delta = self.last_delta_zero;
        loop {
            match self.path.pop() {
                Some(true) => {
                    // This ancestor closes: the completed subtree was its right
                    // child.
                    self.close_ancestor(&mut is_leaf, &mut zero_delta)?;
                }
                Some(false) => {
                    // The flip level: the completed subtree was this ancestor's
                    // left child, and its right subtree is next in the stream.
                    self.left_was_leaf
                        .pop()
                        .expect("the parallel stacks hold one bit each per open ancestor");
                    self.path.push(true);
                    self.left_was_leaf.push(is_leaf);
                    self.open_lefts -= 1;
                    break;
                }
                None => unreachable!(
                    "the advanced cursor is never at its final leaf: the walk checks done() first"
                ),
            }
        }
        let flip = self.path.len();
        let code = self.descend()?;
        self.last_delta_zero = code.is_zero();
        let (sign, magnitude) = unzigzag(code);
        Ok((flip, Step { sign, magnitude }))
    }

    /// Close out the whole tree at exhaustion.
    ///
    /// The final leaf's trailing ancestors close only here — the walk never
    /// steps past it — so their collapsible-pair checks run at this seam or not
    /// at all: a stream whose *last* two leaves are a collapsible pair is
    /// rejected by this close-out.
    fn finish(mut self) -> Result<(), Decode> {
        debug_assert!(self.done(), "finish closes out a completed tree");
        let mut is_leaf = true;
        let mut zero_delta = self.last_delta_zero;
        while let Some(right) = self.path.pop() {
            debug_assert!(right, "a done cursor's path is all right branches");
            self.close_ancestor(&mut is_leaf, &mut zero_delta)?;
        }
        Ok(())
    }
}

/// The admission walk's pair verdict: how the parsed stream relates to the
/// canonical `lo` it was co-swept against.
///
/// Three-valued rather than the bare dominance bool because the same single
/// sign read per elementary interval that proves `lo <= hi` also distinguishes
/// equality (no interval read a strict `Less`), and the coincident span's
/// storage dedup dispatches on exactly that: on [`Equal`](Admission::Equal) the
/// caller materializes one buffer and clones it into both endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    /// The parsed stream equals `lo`: dominance with no strict interval.
    /// Canonical uniqueness makes this exactly byte equality of the two
    /// streams.
    Equal,
    /// The parsed stream strictly dominates `lo`: at least one elementary
    /// interval sits strictly above.
    Dominates,
    /// Dominance refuted: some elementary interval has `lo` strictly above the
    /// parsed stream. The pair is crossed or concurrent — no span encodes it.
    ///
    /// Also the surface for a structurally whole stream whose running height
    /// dips negative (the module doc's subsumption argument) — a stream that is
    /// not a valid version at all — so every caller must treat `Refuted` as
    /// rejection, never as a decided relation between two versions.
    Refuted,
}

/// Strictly parse one skyline tree from `cursor`, deciding in the same pass
/// whether its version dominates — or equals — the canonical stream `lo` (`lo
/// <= hi` pointwise over the unit id interval).
///
/// Returns with the cursor just past the tree, carrying the [`Admission`]
/// verdict. The walk never pronounces the pair rejection itself — the caller
/// checks its own tail obligations first (the byte-slice decode its zero
/// padding, the wire-side cursor its final byte's dead bits) and mints
/// [`Decode::NotCanonical`] from a [`Refuted`](Admission::Refuted) verdict only
/// after they pass, so every structural genre of the composite stays ahead of
/// the pair rejection.
///
/// # Errors
///
/// - [`Decode::Truncated`]: the cursor's bits end mid-tree or
///   mid-integer.
/// - [`Decode::NotCanonical`]: a collapsible sibling pair. A stream
///   whose running height dips negative surfaces as the
///   [`Refuted`](Admission::Refuted) verdict instead (the module doc's
///   subsumption argument), so on a
///   structurally whole stream the caller's rejection carries the same
///   genre the standalone validator would.
/// - [`Decode::Io`]: the cursor's own reads fail (the wire-side
///   cursor's genre; a slice cursor reports truncation instead).
///
/// On an input defective several ways at once, the structural genres win: the
/// walk always parses the whole tree — a refuted verdict never cuts the parse
/// short — and the errors above outrank the verdict at every caller.
///
/// # Panics
///
/// `lo` must be a canonical skyline stream — its cursor is the pair sweep's and
/// shares [`causal_cmp`](super::sweep::causal_cmp)'s contract. The parsed
/// stream needs no such trust; that is the point.
pub(crate) fn validate_dominating_from<C: BitCursor>(
    lo: BitsView<'_>,
    cursor: &mut C,
) -> Result<Admission, Decode>
where
    Decode: From<C::Error>,
{
    let (mut lo_cur, lo_first) = LeafCursor::open(lo);
    let (mut hi_cur, hi_first) = CheckedCursor::open(cursor)?;
    // D = height_lo − height_hi: dominance survives while no elementary
    // interval reads D > 0 (the pair sweep's `lo <= hi` direction, `lo` as the
    // `a` operand, seeded and folded in the sweep's own order so the
    // accumulator traffic is identical).
    let mut diff = Accumulator::new();
    super::signed::fold_signed_int(&mut diff, Sign::Positive, &lo_first);
    super::signed::fold_signed_int(&mut diff, Sign::Negative, &hi_first);
    // Equality rides the same sign reads: the pair is equal exactly when no
    // elementary interval reads a strict `Less` (and none reads `Greater`,
    // which refutes outright) — canonical uniqueness then makes the verdict
    // byte equality of the two streams.
    let mut equal = true;
    loop {
        // One sign read per elementary interval, exactly as the sweep folds it;
        // the three-way match keeps that single read while deciding both the
        // dominance and the equality questions.
        match diff.sign() {
            Ordering::Greater => {
                // Dominance refuted, permanently: the verdict is fixed. Drop
                // `lo`'s cursor and the difference, and complete the strict
                // parse alone so a structural defect later in the stream still
                // reports its own genre.
                while !hi_cur.done() {
                    hi_cur.step()?;
                }
                hi_cur.finish()?;
                return Ok(Admission::Refuted);
            }
            Ordering::Less => equal = false,
            Ordering::Equal => {}
        }
        if lo_cur.done() && hi_cur.done() {
            break;
        }
        advance(&mut lo_cur, &mut hi_cur, &mut diff)?;
    }
    hi_cur.finish()?;
    Ok(if equal {
        Admission::Equal
    } else {
        Admission::Dominates
    })
}

/// Advance the overlay one boundary: the deeper cursor steps, and the other
/// steps in the same round exactly when the flip level rises to or above its
/// depth.
///
/// The overlay-advance law ([`super::overlay::advance`]) restated at this
/// cursor pair — restated rather than reused, because the generic law is
/// infallible and the checked side's crossings are `Result`s. Step and fold
/// order are
/// [`advance_diff`](super::overlay::advance_diff)'s — the deeper side first, `lo`
/// (the `a` operand) first on ties — which keeps the difference's write
/// sequence, and with it the committed touch-meter readings, identical to the
/// pair sweep's.
///
/// The law never steps a done cursor: a not-done side is strictly deeper than a
/// done one (overlapping dyadic intervals nest, and only an all-right path ends
/// at the unit interval's right edge), and a crossed boundary short of the
/// right edge never ties a done side's depth. The sweep module doc carries the
/// bookkeeping argument.
fn advance<C: BitCursor>(
    lo: &mut LeafCursor<'_>,
    hi: &mut CheckedCursor<'_, C>,
    diff: &mut Accumulator,
) -> Result<(), Decode>
where
    Decode: From<C::Error>,
{
    match lo.depth().cmp(&hi.depth()) {
        Ordering::Greater => {
            let (flip_lo, step) = lo.step();
            fold(diff, Side::A, step.sign, &step.magnitude);
            if flip_lo <= hi.depth() {
                let (flip_hi, step) = hi.step()?;
                debug_assert_eq!(
                    flip_lo, flip_hi,
                    "tied boundaries close to one shared flip level"
                );
                fold(diff, Side::B, step.sign, &step.magnitude);
            }
        }
        Ordering::Less => {
            let (flip_hi, step) = hi.step()?;
            fold(diff, Side::B, step.sign, &step.magnitude);
            if flip_hi <= lo.depth() {
                let (flip_lo, step) = lo.step();
                debug_assert_eq!(
                    flip_hi, flip_lo,
                    "tied boundaries close to one shared flip level"
                );
                fold(diff, Side::A, step.sign, &step.magnitude);
            }
        }
        Ordering::Equal => {
            let (flip_lo, step) = lo.step();
            fold(diff, Side::A, step.sign, &step.magnitude);
            let (flip_hi, step) = hi.step()?;
            debug_assert_eq!(
                flip_lo, flip_hi,
                "equal-depth leaves share their whole path, so their flip levels agree"
            );
            fold(diff, Side::B, step.sign, &step.magnitude);
        }
    }
    Ok(())
}
