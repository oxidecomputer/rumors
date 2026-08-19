//! The shape transliteration walks: the crate-private engines under the
//! public step-function iterators in [`crate::shape`].
//!
//! Each walk drives one overlay cursor — [`LeafCursor`] for a skyline
//! stream, [`IdLeafCursor`] for a packed id stream — and exposes exactly
//! what the public vocabulary needs. The event side converts crossings
//! into pending rises (the rise entering the current plateau, held until
//! the public iterator consumes it); the id side needs no conversion at
//! all, because its public item is the absolute per-region state the
//! cursor already carries.
//!
//! The refinement walks rest on the overlay module's boundary
//! bookkeeping (its module doc is the correctness argument): every
//! current plateau contains the sweep point, so the plateau intervals
//! nest by depth, the deepest is the common refinement's next cell, and
//! the cell's right boundary is crossed by the deepest walk plus every
//! walk whose depth the flip level reaches. [`advance_refinement`] states
//! that law once over [`Refine`], for any arity and either walk kind;
//! what this module adds beyond the law is only the rise bookkeeping — a
//! pending rise is created at each plateau entry and consumed exactly
//! once, so a refinement cell carries an input's rise on the first
//! fragment of that input's plateau and `None` on every later fragment.

use crate::codec::{BitsView, Int};
use crate::shape::Rise;
use crate::Ticks;

use super::overlay::{IdLeafCursor, LeafCursor, PlateauCursor};
use super::signed::Sign;

/// The rise a decoded signed delta denotes: `None` for the zero delta,
/// the sign and magnitude lifted into the public vocabulary otherwise.
fn rise(sign: Sign, magnitude: Int) -> Option<Rise> {
    if magnitude.is_zero() {
        return None;
    }
    let ticks = Ticks(magnitude.into_base());
    Some(match sign {
        Sign::Positive => Rise::Up(ticks),
        Sign::Negative => Rise::Down(ticks),
    })
}

/// A shape walk over one skyline stream: [`LeafCursor`] plus the pending
/// rise entering its current leaf.
pub(crate) struct VersionWalk<'a> {
    cursor: LeafCursor<'a>,
    /// The rise entering the current plateau, until consumed; the
    /// stream's first payload is an absolute height, which is exactly
    /// the first rise (the walk enters at height 0).
    pending: Option<Rise>,
}

impl<'a> VersionWalk<'a> {
    /// Open a canonical skyline stream at its first plateau.
    pub(crate) fn open(bits: BitsView<'a>) -> Self {
        let (cursor, first) = LeafCursor::open(bits);
        VersionWalk {
            pending: rise(Sign::Positive, first),
            cursor,
        }
    }

    /// The rise entering the current plateau, consumed.
    ///
    /// The first take after a plateau entry yields it, every later take
    /// yields `None` — which is what makes a refinement cell carry an
    /// input's rise only on its plateau's first fragment.
    pub(crate) fn take_rise(&mut self) -> Option<Rise> {
        self.pending.take()
    }
}

/// A shape walk over one packed id stream: a thin visibility shim over
/// [`IdLeafCursor`] (whose per-region ownership state is already the
/// public item's payload).
pub(crate) struct PartyWalk<'a> {
    cursor: IdLeafCursor<'a>,
}

impl<'a> PartyWalk<'a> {
    /// Open a canonical packed id stream at its first constant region.
    pub(crate) fn open(bits: BitsView<'a>) -> Self {
        PartyWalk {
            cursor: IdLeafCursor::open(bits),
        }
    }

    /// Whether the current region is owned by the walked id.
    pub(crate) fn owned(&self) -> bool {
        self.cursor.owned()
    }
}

/// The refinement walks' view of one shape walk: the overlay-advance
/// law's inputs (depth and exhaustion) and the step it drives.
///
/// The `&mut` blanket impl lets a heterogeneous pair enter
/// [`advance_refinement`] as a slice of `&mut dyn Refine`, so the law
/// has one implementation for the homogeneous combiner and the
/// version × party overlay alike.
pub(crate) trait Refine {
    /// The current plateau's depth: its interval has width `2^-depth`.
    fn depth(&self) -> u64;

    /// Whether the current plateau is the walk's last (its interval ends
    /// at the unit interval's right edge).
    fn done(&self) -> bool;

    /// Advance past the current plateau, arming any pending rise;
    /// returns the flip level for the law's tie test.
    ///
    /// Never called on a final plateau ([`advance_refinement`]'s guards
    /// hold it off; the cursor underneath panics if violated).
    fn advance(&mut self) -> u64;
}

impl Refine for VersionWalk<'_> {
    fn depth(&self) -> u64 {
        self.cursor.depth()
    }

    fn done(&self) -> bool {
        self.cursor.done()
    }

    fn advance(&mut self) -> u64 {
        let (flip, step) = self.cursor.step();
        self.pending = rise(step.sign, step.magnitude);
        flip
    }
}

impl Refine for PartyWalk<'_> {
    fn depth(&self) -> u64 {
        self.cursor.depth()
    }

    fn done(&self) -> bool {
        self.cursor.done()
    }

    fn advance(&mut self) -> u64 {
        let (flip, ()) = self.cursor.step();
        flip
    }
}

impl<T: Refine + ?Sized> Refine for &mut T {
    fn depth(&self) -> u64 {
        (**self).depth()
    }

    fn done(&self) -> bool {
        (**self).done()
    }

    fn advance(&mut self) -> u64 {
        (**self).advance()
    }
}

/// Advance a refinement walk one cell boundary, or report that every
/// walk is at its final plateau (the current cell is the refinement's
/// last).
///
/// The overlay-advance law at arity N over [`Refine`]s: the deepest
/// unexhausted walk steps, and every other unexhausted walk whose depth
/// reaches the flip level steps in the same round.
///
/// An exhausted walk's plateau runs to the unit interval's right edge,
/// so it is never the deepest side and never reaches a flip level (both
/// would put its end strictly inside the interval); it simply spans
/// every remaining cell. The empty slice reports done immediately: with
/// no boundaries to cross, the single all-interval cell is final.
pub(crate) fn advance_refinement<W: Refine>(walks: &mut [W]) -> bool {
    let mut deepest: Option<(usize, u64)> = None;
    for (slot, walk) in walks.iter().enumerate() {
        if walk.done() {
            continue;
        }
        let depth = walk.depth();
        // Strict: the first unexhausted slot achieving the maximum.
        if deepest.is_none_or(|(_, max)| depth > max) {
            deepest = Some((slot, depth));
        }
    }
    let Some((deepest, _)) = deepest else {
        return true;
    };
    let flip = walks[deepest].advance();
    for (slot, walk) in walks.iter_mut().enumerate() {
        if slot != deepest && !walk.done() && walk.depth() >= flip {
            let tied = walk.advance();
            debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
        }
    }
    false
}
