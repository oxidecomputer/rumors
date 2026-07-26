//! The region difference `self \ other`, as a boolean-skyline sweep.
//!
//! Under the packed coding an id *is* a boolean skyline: a dyadic tiling
//! of the unit interval into owned (`1`) and unowned (`0`) plateaus,
//! listed left to right in preorder, with an absent child standing for an
//! unowned plateau that occupies no bits. Region difference is pointwise
//! `a ∧ ¬b` over that interval, so it rides the event side's sweep
//! discipline (the skyline sweep module carries the boundary
//! bookkeeping): two leaf cursors walk the operands' overlay partition,
//! one output plateau — owned exactly where `self` is and `other` is
//! not — is appended per elementary interval at the deeper cursor's
//! depth, and the collapsing output builder
//! ([`IdSkylineBuilder`](super::build::IdSkylineBuilder)) re-derives the
//! canonical id from the depth sequence. What the event sweep's
//! accumulator does for integer heights, a single owned bit per cursor
//! does here: the boolean semiring needs no running difference, only the
//! two current values.
//!
//! Nothing recurses and nothing is skipped: every topology tag of either
//! operand is read exactly once, the transient state is bits per open
//! ancestor (two cursor paths and the builder's stacks), and identical
//! deep operands — the shape on which a structural walk's recursion
//! depth tracks the full tree depth — cost bits, not stack frames or
//! grown segments.

use core::cmp::Ordering;

use crate::codec::{Bits, BitsSlice};
use crate::idbits::IdReader;
use crate::step;

use super::build::IdSkylineBuilder;

impl IdReader<'_> {
    /// The region *difference* `self \ other` (normal-form ids): the part of
    /// `self`'s region that `other` does not own, as a normalized id.
    ///
    /// Unlike [`sum`](IdReader::sum), `diff` is *total* — overlap is the whole
    /// point, not an error — and its result may be the **empty** `0` id (the
    /// empty bit stream), exactly when `other` covers `self`. The caller
    /// ([`Party::without`](crate::Party::without))
    /// maps that empty result to `None`, since a `Party` is a nonzero share.
    ///
    /// The result is always a subregion of `self` (`self \ other ⊆ self`), so it
    /// introduces no region `self` did not already own. That is what keeps it
    /// linearity-safe where a general id *meet* is not (see the note on the
    /// absent `BitAnd for Clock` in [`oracle`](crate::oracle)): carving a
    /// sub-share out of a region you already hold, and consuming the original,
    /// can never synthesize a region shared with a third live party.
    ///
    /// `O(n + m)`: the sweep form of `oracle::Party::without` (the module
    /// doc), reading each operand's tags exactly once and emitting one
    /// output plateau per elementary interval of the overlay.
    pub(crate) fn diff(self, other: IdReader) -> Bits {
        // `self \ other ⊆ self`, but over a full `self` plateau the output
        // is `other`'s complement, which can be as large as `other`. Both
        // inputs combined is a safe bound; normalization only shrinks it.
        let mut out = IdSkylineBuilder::with_capacity(self.bits().len() + other.bits().len());
        let mut a = IdLeafCursor::open(self);
        let mut b = IdLeafCursor::open(other);
        loop {
            // One plateau per elementary interval — the deeper cursor's,
            // since overlapping dyadic intervals nest — owned where `self`
            // survives `other`.
            out.leaf(a.depth().max(b.depth()), a.owned() && !b.owned());
            if a.done() && b.done() {
                return out.finish();
            }
            advance(&mut a, &mut b);
        }
    }
}

/// Advance the overlay walk one boundary: step the deeper cursor, and the
/// other in the same step on a tie.
///
/// The event sweep's `advance` on boolean cursors — overlapping dyadic
/// intervals nest, so the deeper plateau ends first or ties, and a tie at
/// unequal depths is visible as the deeper side's flip level rising to or
/// above the shallower side's depth (the skyline sweep module derives the
/// bookkeeping). The two sides of a tie then close to the same flip
/// level, debug-asserted here exactly as there.
fn advance(a: &mut IdLeafCursor, b: &mut IdLeafCursor) {
    match a.depth().cmp(&b.depth()) {
        Ordering::Greater => {
            let fa = a.step();
            if fa <= b.depth() {
                let fb = b.step();
                debug_assert_eq!(fa, fb, "tied boundaries close to one shared flip level");
            }
        }
        Ordering::Less => {
            let fb = b.step();
            if fb <= a.depth() {
                let fa = a.step();
                debug_assert_eq!(fb, fa, "tied boundaries close to one shared flip level");
            }
        }
        Ordering::Equal => {
            let fa = a.step();
            let fb = b.step();
            debug_assert_eq!(
                fa, fb,
                "equal-depth plateaus share their whole path, so their flip levels agree"
            );
        }
    }
}

/// A cursor at the current plateau of one packed id, read as a boolean
/// skyline.
///
/// The id-side sibling of the event sweep's leaf cursor: the tag stream
/// is consumed forward exactly once, the root-to-plateau path is the only
/// per-depth state, and the same three dyadic facts (the deeper plateau
/// ends first; ties close to one shared flip level; the all-right path is
/// the exhausted tiling) drive [`advance`]. An absent child — a stored
/// `0` occupies no bits — is presented as a synthetic unowned plateau at
/// the child's own depth, so the cursor always tiles the whole interval.
struct IdLeafCursor<'a> {
    bits: &'a BitsSlice,
    /// The next unread tag's bit offset. Preorder consumption keeps it at
    /// the subtree of the next *present* child slot the walk flips into;
    /// synthetic plateaus consume nothing.
    pos: usize,
    /// Root-to-plateau branch directions: `false` inside a left child
    /// slot, `true` inside a right.
    path: Bits,
    /// One bit per open left-branch level, innermost last: whether that
    /// ancestor's right child is present in the stream (`false` = the
    /// right slot is a synthetic unowned plateau).
    pending_right: Bits,
    /// Count of left-branch levels in `path`: zero exactly at the final
    /// plateau (the all-right path), so [`done`](Self::done) is `O(1)`.
    open_lefts: usize,
    /// Whether the current plateau is owned.
    owned: bool,
}

impl<'a> IdLeafCursor<'a> {
    /// Open an id at its first plateau. A synthetic
    /// [`Empty`](IdReader::Empty) reader is the anonymous `0` id: one
    /// unowned plateau covering the whole interval.
    fn open(src: IdReader<'a>) -> Self {
        let mut this = IdLeafCursor {
            bits: BitsSlice::empty(),
            pos: 0,
            path: Bits::new(),
            pending_right: Bits::new(),
            open_lefts: 0,
            owned: false,
        };
        if let IdReader::At { bits, pos } = src {
            this.bits = bits;
            this.pos = pos;
            this.descend();
        }
        this
    }

    /// The current plateau's depth: its interval has width `2^-depth`.
    fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether the current plateau is the tiling's last (it ends at the
    /// unit interval's right edge).
    fn done(&self) -> bool {
        self.open_lefts == 0
    }

    /// Whether the current plateau is owned.
    fn owned(&self) -> bool {
        self.owned
    }

    /// Advance past the current plateau to the next, returning the flip
    /// level's depth for the caller's tie test.
    ///
    /// Pops the trailing right-branch levels (each an ancestor whose
    /// subtree the consumed plateau completed), then steps the deepest
    /// left-branch level to its right child slot: a present subtree is
    /// descended, an absent one is the synthetic unowned plateau at the
    /// flip level itself.
    ///
    /// Never called on a final plateau: a sweep stops when both cursors
    /// are done, and the skyline sweep module's bookkeeping (which this
    /// cursor inherits) shows a final plateau is never the advanced side
    /// before then.
    fn step(&mut self) -> usize {
        loop {
            match self.path.pop() {
                Some(true) => continue, // this ancestor closed with the plateau
                Some(false) => break,   // the flip level: its right slot is next
                None => unreachable!(
                    "the advanced cursor is never at its final plateau: an all-right path means the tiling is consumed"
                ),
            }
        }
        self.open_lefts -= 1;
        self.path.push(true);
        let flip = self.path.len();
        let right_present = self
            .pending_right
            .pop()
            .expect("every open left branch queues its right slot");
        if right_present {
            self.descend();
        } else {
            self.owned = false;
        }
        flip
    }

    /// Descend from `pos` into the present subtree there, to its first
    /// plateau: read each internal tag and enter its left slot, stopping
    /// at a terminal (an owned plateau) or an absent left child (a
    /// synthetic unowned plateau).
    fn descend(&mut self) {
        loop {
            step!();
            crate::codec::scan::record_bits(2); // one 2-bit tag read
            let (left, right) = (self.bits[self.pos], self.bits[self.pos + 1]);
            self.pos += 2;
            if !left && !right {
                // The terminal `1` leaf.
                self.owned = true;
                return;
            }
            self.path.push(false);
            self.pending_right.push(right);
            self.open_lefts += 1;
            if !left {
                // An absent left child: an unowned plateau, no bits.
                self.owned = false;
                return;
            }
        }
    }
}
