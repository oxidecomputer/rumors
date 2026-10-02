//! Validates the upper version of a span while comparing it with the lower.
//!
//! A span contains canonical versions `lo` and `hi` and requires `lo <= hi`
//! everywhere. Validating `hi` and then comparing the pair would decode `hi`
//! twice. This walk instead advances both trees over the same regions while
//! strictly parsing `hi`.
//!
//! The comparison accumulator holds `height_lo - height_hi`. A positive value
//! disproves dominance; a negative value also records that the versions differ.
//! Because a validated `lo` is nonnegative, an accepted `hi` is necessarily
//! nonnegative as well. The comparison therefore replaces the separate height
//! accumulator used by the standalone validator. [`CheckedVersionReader`] still
//! rejects truncation and collapsible sibling leaves as it parses `hi`.
//!
//! Once dominance fails, further comparison cannot change the verdict. The
//! walk stops advancing `lo` and the difference, but continues parsing `hi` so
//! a later structural error is still returned. All traversal is iterative;
//! transient state is two compact path stacks and one accumulator.

use core::cmp::Ordering;

use suanpan::Accumulator;

use num_bigint::BigUint;

use crate::bits::stack::BitStack;
use crate::bits::BitRead;
use crate::error::Decode;
use crate::version::io::regions::PayloadKind;

use crate::version::io::regions::{HeightChange, RegionReader, VersionRegionReader};
use crate::version::overlay::Side;
use crate::Version;

/// A validating leaf cursor over one untrusted Version stream.
///
/// [`VersionRegionReader`]'s region vocabulary — depth, done, step — with the strict
/// validator's obligations folded into the same reads: truncation surfaces as
/// the cursor's own errors, and minimal topology (no collapsible sibling pair)
/// is checked as each internal node closes. It need not track `hi`'s height
/// separately: dominance over the validated, nonnegative `lo` establishes
/// nonnegativity on every accepted region.
///
/// Two parallel per-ancestor bit stacks ride the walk: the branch path (the
/// advance law's tie test, as [`VersionRegionReader`]'s), and the validator's
/// left-was-leaf bits (what the sibling-collapse check reads at each close).
/// `open_lefts` counts the path's left branches, so exhaustion — the tree's
/// root completing — is an O(1) question where the path itself would need a
/// full scan.
struct CheckedVersionReader<'a, C> {
    cursor: &'a mut C,
    /// Root-to-leaf branch directions, root first (`false`: inside the left
    /// child, its right sibling still pending in the stream).
    path: BitStack,
    /// Per open ancestor: whether its completed left child was a leaf (a
    /// placeholder `false` until that child completes).
    left_was_leaf: BitStack,
    /// The count of `false` bits in `path`: zero exactly when the current
    /// leaf's plateau ends at the unit interval's right edge — the tree is
    /// whole and the stream's bits end here.
    ///
    /// `u64`, as the path height it counts within: each open left branch
    /// is one stored path bit.
    open_lefts: u64,
    /// Whether the current leaf's payload code was zero — the collapsible-pair
    /// check's right-child half. Never read for the first leaf (preorder puts
    /// it leftmost, so it is no ancestor's right child).
    last_delta_zero: bool,
}

impl<'a, C: BitRead> CheckedVersionReader<'a, C>
where
    Decode: From<C::Error>,
{
    /// Open the stream at its first leaf: the descent to it, and the leaf's
    /// absolute height code.
    fn open(cursor: &'a mut C) -> Result<(Self, BigUint), Decode> {
        let mut this = CheckedVersionReader {
            cursor,
            path: BitStack::new(),
            left_was_leaf: BitStack::new(),
            open_lefts: 0,
            last_delta_zero: false,
        };
        let first = this.descend()?;
        Ok((this, first))
    }

    /// Descend to the next leaf in preorder, opening the internal nodes on the
    /// way: [`VersionRegionReader`]'s descent with the reads fallible and the
    /// validator's placeholder bits pushed alongside the path.
    fn descend(&mut self) -> Result<BigUint, Decode> {
        let internal_nodes = self.cursor.read_unary()?;
        for _ in 0..internal_nodes {
            self.path.push(false);
            self.left_was_leaf.push(false); // placeholder until the left child completes
        }
        self.open_lefts += internal_nodes;
        self.cursor.read_gamma()
    }

    /// The current leaf's depth: its plateau has width `2^-depth`.
    ///
    /// Depths are `u64` across the walk surface, as every stream position
    /// is: each open ancestor costs at least one bit of the walked stream,
    /// whose live length outgrows a 32-bit `usize` from 512 MiB.
    fn depth(&self) -> u64 {
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
    fn step(&mut self) -> Result<(u64, HeightChange), Decode> {
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
        let flip = self.depth();
        let code = self.descend()?;
        self.last_delta_zero = code == BigUint::ZERO;
        Ok((flip, PayloadKind::Delta.decode(code)))
    }

    /// Close out the whole tree at exhaustion.
    ///
    /// The final leaf's trailing ancestors close only here because the walk
    /// never advances beyond it. Closing them here is what rejects a stream
    /// whose final two leaves form a collapsible pair.
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
/// canonical `lo` it was compared with.
///
/// Three-valued rather than the bare dominance bool because the same single
/// sign read per elementary interval that proves `lo <= hi` also distinguishes
/// equality (no interval read a strict `Less`), and the coincident span's
/// storage dedup dispatches on exactly that: on [`Equal`](Admission::Equal) the
/// caller materializes one buffer and clones it into both endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
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
    /// This also covers a parsed stream whose running height becomes negative:
    /// such a stream cannot dominate the nonnegative `lo`. Callers therefore
    /// treat `Refuted` as rejection, not as a relation between valid versions.
    Refuted,
}

/// Strictly parse one Version tree from `cursor`, deciding in the same pass
/// whether its version dominates — or equals — the canonical stream `lo` (`lo
/// <= hi` pointwise over the party space).
///
/// Returns with the cursor just past the tree, carrying the [`Admission`]
/// verdict. The walk never pronounces the pair rejection itself — the caller
/// first checks the required marker and zero padding, then converts a
/// [`Refuted`](Admission::Refuted) verdict to [`Decode::NotCanonical`]. This
/// preserves structural errors found while parsing the complete stream.
///
/// # Errors
///
/// - [`Decode::Truncated`]: the cursor's bits end mid-tree or
///   mid-integer.
/// - [`Decode::NotCanonical`]: a collapsible sibling pair. A negative running
///   height produces [`Admission::Refuted`] instead.
/// - [`Decode::Io`]: the cursor's own reads fail. A slice cursor reports
///   exhaustion as truncation instead.
///
/// The walk parses the whole tree after dominance fails, so these errors take
/// precedence over the verdict.
///
/// # Panics
///
/// `lo` must be a canonical Version stream — its cursor is the pair sweep's and
/// shares [`Version::partial_cmp`]'s contract. The parsed
/// stream needs no such trust; that is the point.
pub fn dominating_from<C: BitRead>(lo: &Version, cursor: &mut C) -> Result<Admission, Decode>
where
    Decode: From<C::Error>,
{
    let (mut lo_cur, lo_first) = VersionRegionReader::open(lo);
    let (mut hi_cur, hi_first) = CheckedVersionReader::open(cursor)?;
    // The difference is `height_lo - height_hi`. Dominance holds only while it
    // is nonpositive over every region produced by the combined walk.
    let mut diff = Accumulator::new();
    diff.add_shifted_limbs(0, lo_first.iter_u64_digits());
    diff.sub_shifted_limbs(0, hi_first.iter_u64_digits());
    // Equality rides the same sign reads: the pair is equal exactly when no
    // elementary interval reads a strict `Less` (and none reads `Greater`,
    // which refutes outright) — canonical uniqueness then makes the verdict
    // byte equality of the two streams.
    let mut equal = true;
    loop {
        // One sign read per elementary interval, exactly as the sweep folds it;
        // the three-way match keeps that single read while deciding both the
        // dominance and the equality questions.
        match diff.cmp_zero() {
            Ordering::Greater => {
                // Dominance is permanently refuted. Stop using the lower
                // cursor and difference, but finish parsing the upper stream
                // so a later structural defect is still returned.
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
/// This is the ordinary overlay rule specialized to a checked cursor whose
/// reads can fail. The deeper region ends first. When its boundary rises to the
/// other cursor's depth, both cursors cross the same boundary; ties advance
/// `lo` first to preserve the comparison fold's ordering.
///
/// A completed cursor is never stepped. Its final region reaches the right edge
/// of the unit interval, while every unfinished cursor reaches an earlier
/// boundary.
fn advance<C: BitRead>(
    lo: &mut VersionRegionReader<'_>,
    hi: &mut CheckedVersionReader<'_, C>,
    diff: &mut Accumulator,
) -> Result<(), Decode>
where
    Decode: From<C::Error>,
{
    match lo.depth().cmp(&hi.depth()) {
        Ordering::Greater => {
            let (flip_lo, step) = lo.step();
            Side::A.fold(diff, &step);
            if flip_lo <= hi.depth() {
                let (flip_hi, step) = hi.step()?;
                debug_assert_eq!(
                    flip_lo, flip_hi,
                    "tied boundaries close to one shared flip level"
                );
                Side::B.fold(diff, &step);
            }
        }
        Ordering::Less => {
            let (flip_hi, step) = hi.step()?;
            Side::B.fold(diff, &step);
            if flip_hi <= lo.depth() {
                let (flip_lo, step) = lo.step();
                debug_assert_eq!(
                    flip_hi, flip_lo,
                    "tied boundaries close to one shared flip level"
                );
                Side::A.fold(diff, &step);
            }
        }
        Ordering::Equal => {
            let (flip_lo, step) = lo.step();
            Side::A.fold(diff, &step);
            let (flip_hi, step) = hi.step()?;
            debug_assert_eq!(
                flip_lo, flip_hi,
                "equal-depth leaves share their whole path, so their flip levels agree"
            );
            Side::B.fold(diff, &step);
        }
    }
    Ok(())
}
