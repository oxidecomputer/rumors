//! The region difference `self \ other`, as a boolean-skyline sweep
//! with covered-block early exits.
//!
//! Under the packed coding an id *is* a boolean skyline: a dyadic tiling
//! of the unit interval into owned (`1`) and unowned (`0`) plateaus,
//! listed left to right in preorder, with an absent child standing for an
//! unowned plateau that occupies no bits. Region difference is pointwise
//! `a ∧ ¬b` over that interval, so it rides the event side's
//! overlay-advance law itself, through the plateau-cursor trait (the
//! skyline sweep module states the law once and carries its boundary
//! bookkeeping): two cursors walk the operands' overlay partition, the
//! output — owned exactly where `self` is and `other` is not — is
//! appended item by item at the deeper cursor's depth, and the
//! collapsing output builder ([`IdSkylineBuilder`]) re-derives the
//! canonical id from the depth sequence. What the event sweep's
//! accumulator does for integer heights, a single owned bit per cursor
//! does here: the boolean semiring needs no running difference, only the
//! two current values.
//!
//! # Covered blocks
//!
//! Because dyadic intervals nest, whenever one cursor is about to enter
//! a subtree, the other cursor's current plateau either covers that
//! subtree's whole interval or subdivides it. A covering plateau makes
//! the output over the interval a *block*, settled without walking the
//! subtree plateau by plateau:
//!
//! - `other` unowned over a `self` subtree: the subtree passes through
//!   unchanged — one iterative block scan past its tags and one verbatim
//!   splice into the output ([`IdSkylineBuilder::subtree`]).
//! - `other` owned over a `self` subtree: nothing of it survives — one
//!   block scan, and the interval stands as a single unowned plateau.
//! - `self` unowned over an `other` subtree: nothing there to carve
//!   from — one block scan, one unowned plateau.
//!
//! The fourth pairing (`self` owned over an `other` subtree) is not a
//! block: the output there is `other`'s complement, so the sweep walks
//! that subtree plateau by plateau. When neither cursor's plateau covers
//! the other's subtree — both enter subtrees over the same interval —
//! the two descend in lockstep until a plateau or a covered block
//! appears ([`descend_pair`]).
//!
//! Nothing recurses and nothing is read twice: every topology tag of
//! either operand is read at most once (block scans consume the tags
//! their subtree would have spent on descent), the transient state is
//! bits per open ancestor (two cursor paths and the builder's stacks),
//! and identical deep operands — the shape on which a structural walk's
//! recursion depth tracks the full tree depth — cost bits, not stack
//! frames or grown segments.

use crate::codec::{Bits, BitsSlice};
use crate::idbits::IdReader;
use crate::version::skyline::sweep::{self, PlateauCursor};

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
    /// doc), reading each operand's tags at most once and emitting one
    /// output plateau or covered block per item of the overlay.
    pub(crate) fn diff(self, other: IdReader) -> Bits {
        // `self \ other ⊆ self`, but over a full `self` plateau the output
        // is `other`'s complement, which can be as large as `other`. Both
        // inputs combined is a safe bound; normalization only shrinks it.
        let mut out = IdSkylineBuilder::with_capacity(self.bits().len() + other.bits().len());
        let (mut a, a_entering) = IdLeafCursor::open(self);
        let (mut b, b_entering) = IdLeafCursor::open(other);
        settle_pair(&mut a, &mut b, a_entering, b_entering);
        loop {
            // One item per boundary: a covered `self` subtree splices
            // whole, otherwise one plateau at the deeper cursor's depth —
            // overlapping dyadic intervals nest — owned exactly where
            // `self` survives `other`.
            match a.item {
                Item::Splice { start } => {
                    debug_assert!(
                        b.depth() <= a.depth() && !b.owned(),
                        "a splice is covered by an unowned `other` plateau"
                    );
                    out.subtree(a.depth(), &a.bits[start..a.pos]);
                }
                Item::Plateau { owned } => {
                    out.leaf(a.depth().max(b.depth()), owned && !b.owned());
                }
            }
            if a.done() && b.done() {
                return out.finish();
            }
            advance(&mut a, &mut b);
        }
    }
}

/// Advance the overlay walk one boundary — the overlay-advance law
/// ([`sweep::advance`]) over two boolean cursors — then settle any
/// subtree either cursor stepped into against the other side.
///
/// The law steps the deeper cursor, and the other in the same round
/// exactly on a tie (it debug-asserts the shared flip level); each
/// crossing it yields is that cursor's *entering* flag. A cursor
/// stepping *alone* lands strictly deeper than the other's current
/// plateau, so that plateau covers whatever subtree it entered (dyadic
/// intervals sharing a point nest): the solo arms settle against it
/// directly. A tie lands both cursors on slots of one shared depth,
/// settled jointly by [`settle_pair`].
fn advance(a: &mut IdLeafCursor, b: &mut IdLeafCursor) {
    // The crossings settle only after the whole boundary is crossed —
    // a settle reads and moves *both* cursors — so the fold callback
    // has nothing to do and the positional returns carry the flags.
    match sweep::advance(a, b, |_| {}) {
        (Some(a_entering), Some(b_entering)) => settle_pair(a, b, a_entering, b_entering),
        (Some(true), None) => settle_a(a, b),
        (None, Some(true)) => settle_b(a, b),
        (Some(false), None) | (None, Some(false)) => {}
        (None, None) => unreachable!("the law steps at least one cursor every boundary"),
    }
}

/// Settle a `self` subtree that `b`'s current plateau covers: nothing of
/// it survives an owned cover (a block scan, one unowned plateau), and
/// all of it survives an unowned cover (a block scan, one verbatim
/// splice).
///
/// The caller guarantees the cover: `b`'s plateau contains the subtree's
/// start and is at most as deep, so by dyadic nesting it contains the
/// whole subtree interval.
fn settle_a(a: &mut IdLeafCursor, b: &IdLeafCursor) {
    a.consume(!b.owned());
}

/// Settle an `other` subtree that `a`'s current plateau covers.
///
/// Under an unowned `a` there is nothing to carve from (a block scan,
/// one unowned plateau), while under an owned `a` the output is the
/// subtree's complement, so the sweep must walk it plateau by plateau
/// (an eager descent, no block).
///
/// The caller guarantees the cover, as in [`settle_a`].
fn settle_b(a: &IdLeafCursor, b: &mut IdLeafCursor) {
    if a.owned() {
        b.descend();
    } else {
        b.consume(false);
    }
}

/// Settle both cursors after a shared-depth boundary: whichever side
/// stepped into a subtree (`*_entering`) is settled against the other.
///
/// Both slots sit at one shared depth, so a side that settled at a
/// plateau (its slot was an absent child) covers the other side's
/// subtree; when both entered subtrees over the same interval, neither
/// covers, and the pair descends in lockstep.
fn settle_pair(a: &mut IdLeafCursor, b: &mut IdLeafCursor, a_entering: bool, b_entering: bool) {
    match (a_entering, b_entering) {
        (false, false) => {}
        (true, false) => settle_a(a, b),
        (false, true) => settle_b(a, b),
        (true, true) => descend_pair(a, b),
    }
}

/// Descend two subtrees over the same interval in lockstep until one
/// side reaches a plateau or an absent child, then settle the other side
/// against it.
///
/// Each round consumes both tops. While both are internal with present
/// left children the descent continues one level down (the intervals
/// stay equal); the moment one side resolves to a plateau — a terminal
/// (owned at the shared depth) or an absent left child (unowned one
/// level down) — that plateau covers whatever the other side still has
/// open at or below the same depth, and the covered-block rules of
/// [`settle_a`]/[`settle_b`] apply.
fn descend_pair(a: &mut IdLeafCursor, b: &mut IdLeafCursor) {
    loop {
        match (a.enter(), b.enter()) {
            // Both still inside: their left-child subtrees share the
            // next interval.
            (Enter::Left, Enter::Left) => continue,
            // One side's plateau covers the other's open subtree: a
            // block (or, under an owned `a`, the complement walk).
            (Enter::Left, Enter::Absent) => return a.consume(true),
            (Enter::Left, Enter::Full) => return a.consume(false),
            (Enter::Absent, Enter::Left) => return b.consume(false),
            (Enter::Full, Enter::Left) => return b.descend(),
            // Both sides resolved to plateaus: the sweep takes over.
            (Enter::Full, Enter::Full)
            | (Enter::Full, Enter::Absent)
            | (Enter::Absent, Enter::Full)
            | (Enter::Absent, Enter::Absent) => return,
        }
    }
}

/// The cursor's current item: one entry of its side of the overlay
/// tiling, occupying the dyadic interval at the cursor's depth.
#[derive(Clone, Copy)]
enum Item {
    /// A plateau: owned (a terminal) or unowned (an absent child, or a
    /// subtree consumed as a covered block that contributes nothing).
    Plateau { owned: bool },
    /// A whole `self` subtree consumed as one covered block, to be
    /// spliced verbatim: `bits[start..pos]`. Only ever formed on the
    /// `self` cursor, under an unowned `other` plateau.
    Splice { start: usize },
}

/// What one descent move resolved to (see [`IdLeafCursor::enter`]).
enum Enter {
    /// The top was the terminal: the cursor is at an owned plateau at
    /// the same depth.
    Full,
    /// The top was internal with a present left child: the cursor is
    /// atop that subtree, one level deeper, still unsettled.
    Left,
    /// The top was internal with an absent left child: the cursor is at
    /// an unowned plateau, one level deeper.
    Absent,
}

/// A cursor at the current item of one packed id, read as a boolean
/// skyline.
///
/// The id-side sibling of the event sweep's leaf cursor: the tag stream
/// is consumed forward at most once, the root-to-item path is the only
/// per-depth state, and the same three dyadic facts (the deeper item
/// ends first; ties close to one shared flip level; the all-right path is
/// the exhausted tiling) drive [`advance`]. An absent child — a stored
/// `0` occupies no bits — is presented as a synthetic unowned plateau at
/// the child's own depth, so the cursor always tiles the whole interval.
///
/// Stepping into a present subtree leaves the cursor *unsettled* (the
/// step reports it): the sweep decides — from the other operand's
/// current plateau — whether the subtree is consumed as one covered
/// block ([`consume`](Self::consume)) or walked plateau by plateau
/// ([`enter`](Self::enter)/[`descend`](Self::descend)).
struct IdLeafCursor<'a> {
    bits: &'a BitsSlice,
    /// The next unread tag's bit offset. Preorder consumption keeps it at
    /// the subtree of the next *present* child slot the walk flips into;
    /// synthetic plateaus consume nothing.
    pos: usize,
    /// Root-to-item branch directions: `false` inside a left child
    /// slot, `true` inside a right.
    path: Bits,
    /// One bit per open left-branch level, innermost last: whether that
    /// ancestor's right child is present in the stream (`false` = the
    /// right slot is a synthetic unowned plateau).
    pending_right: Bits,
    /// Count of left-branch levels in `path`: zero exactly at the final
    /// item (the all-right path), so [`done`](Self::done) is `O(1)`.
    open_lefts: usize,
    /// The current item (meaningless while the cursor is unsettled atop
    /// a just-entered subtree; every unsettled state is resolved before
    /// the sweep emits).
    item: Item,
}

impl<'a> IdLeafCursor<'a> {
    /// Open an id at its root, unsettled atop the whole tree (`true`)
    /// for the sweep to settle against the other operand.
    ///
    /// A synthetic [`Empty`](IdReader::Empty) reader is the anonymous
    /// `0` id — one unowned plateau covering the whole interval,
    /// already settled (`false`).
    fn open(src: IdReader<'a>) -> (Self, bool) {
        let mut this = IdLeafCursor {
            bits: BitsSlice::empty(),
            pos: 0,
            path: Bits::new(),
            pending_right: Bits::new(),
            open_lefts: 0,
            item: Item::Plateau { owned: false },
        };
        if let IdReader::At { bits, pos } = src {
            this.bits = bits;
            this.pos = pos;
            (this, true)
        } else {
            (this, false)
        }
    }

    /// Whether the current plateau is owned. Never queried on a spliced
    /// block: the splice's covering side is always a plateau.
    fn owned(&self) -> bool {
        match self.item {
            Item::Plateau { owned } => owned,
            Item::Splice { .. } => unreachable!("a spliced block is never the covering side"),
        }
    }

    /// One descent move at an unexplored subtree top: consume its tag.
    ///
    /// A terminal settles the cursor at the owned plateau there
    /// ([`Enter::Full`]); an internal node opens the level and lands on
    /// its left slot — settled at the synthetic unowned plateau if the
    /// left child is absent ([`Enter::Absent`]), unsettled atop the
    /// left subtree if present ([`Enter::Left`]).
    fn enter(&mut self) -> Enter {
        crate::codec::scan::record_bits(2); // one 2-bit tag read
        let (left, right) = (self.bits[self.pos], self.bits[self.pos + 1]);
        self.pos += 2;
        if !left && !right {
            // The terminal `1` leaf.
            self.item = Item::Plateau { owned: true };
            return Enter::Full;
        }
        self.path.push(false);
        self.pending_right.push(right);
        self.open_lefts += 1;
        if left {
            Enter::Left
        } else {
            // An absent left child: an unowned plateau, no bits.
            self.item = Item::Plateau { owned: false };
            Enter::Absent
        }
    }

    /// Descend from an unexplored subtree top to its first plateau:
    /// read each internal tag and enter its left slot, stopping at a
    /// terminal or an absent left child.
    fn descend(&mut self) {
        while matches!(self.enter(), Enter::Left) {}
    }

    /// Consume the whole unexplored subtree at the cursor as one
    /// covered block, without walking its plateaus.
    ///
    /// A terminal top settles as the owned plateau (the sweep's
    /// pointwise emission already yields the block's constant there);
    /// an internal top is scanned past in one iterative pass (the
    /// shared [`skip_subtree`](crate::idbits::skip_subtree) discipline:
    /// a pending-children counter, never the call stack) and stands as
    /// a verbatim [`Item::Splice`] when it survives into the output
    /// (`splice`, an unowned `other` cover) or as one unowned plateau
    /// when nothing of it does (an owned `other` or unowned `self`
    /// cover).
    fn consume(&mut self, splice: bool) {
        let start = self.pos;
        crate::codec::scan::record_bits(2); // the subtree top's 2-bit tag
        let (left, right) = (self.bits[self.pos], self.bits[self.pos + 1]);
        self.pos += 2;
        if !left && !right {
            self.item = Item::Plateau { owned: true };
            return;
        }
        let bits = self.bits;
        let scan = |at: usize| {
            // One 2-bit tag scanned per skipped node. Children present =
            // the two tag bits; the tag is 2 bits wide.
            crate::codec::scan::record_bits(2);
            let children = usize::from(bits[at]) + usize::from(bits[at + 1]);
            (children, at + 2)
        };
        if left {
            self.pos = crate::idbits::skip_subtree(self.pos, scan);
        }
        if right {
            self.pos = crate::idbits::skip_subtree(self.pos, scan);
        }
        self.item = if splice {
            Item::Splice { start }
        } else {
            Item::Plateau { owned: false }
        };
    }
}

impl PlateauCursor for IdLeafCursor<'_> {
    /// A step's crossing is its *entering* flag: whether the cursor is
    /// now unsettled atop a present subtree.
    ///
    /// An absent slot settles immediately as the synthetic unowned
    /// plateau. The sweep settles every entered subtree against the
    /// other operand before emitting.
    type Crossing = bool;

    /// The current item's depth: its interval has width `2^-depth`.
    fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether the current item is the tiling's last (it ends at the
    /// unit interval's right edge).
    fn done(&self) -> bool {
        self.open_lefts == 0
    }

    /// Advance past the current item to the next slot: the flip level's
    /// depth for the law's tie test, and the entering flag.
    ///
    /// Pops the trailing right-branch levels (each an ancestor whose
    /// subtree the consumed item completed), then steps the deepest
    /// left-branch level to its right child slot.
    ///
    /// Never called on a final item: a sweep stops when both cursors
    /// are done, and the skyline sweep module's bookkeeping (which this
    /// cursor inherits) shows a final item is never the advanced side
    /// before then.
    fn step(&mut self) -> (usize, bool) {
        loop {
            match self.path.pop() {
                Some(true) => continue, // this ancestor closed with the item
                Some(false) => break,   // the flip level: its right slot is next
                None => unreachable!(
                    "the advanced cursor is never at its final item: an all-right path means the tiling is consumed"
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
            (flip, true)
        } else {
            self.item = Item::Plateau { owned: false };
            (flip, false)
        }
    }
}
