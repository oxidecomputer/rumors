//! Adapts Party and Version streams to the public shape iterators.
//!
//! A version iterator reports the rise that enters each constant-height region.
//! A party iterator reports whether each constant-ownership region is owned.
//! The cursors already expose those regions; this module translates their
//! decoded values into the public vocabulary.
//!
//! Several shapes can also be refined into one shared partition. Their current
//! regions all contain the same point and therefore nest. The smallest region
//! supplies the next boundary; every other region ending there advances with
//! it. A version's pending rise is consumed only by the first refined fragment
//! of its region.

use num_bigint::{BigInt, Sign};

use crate::party::io::PartyRegionReader;
use crate::shape::Rise;
use crate::{Count, Party};

use crate::version::io::regions::{RegionReader, VersionRegionReader};
use crate::Version;

/// A shape walk over one Version stream: [`VersionRegionReader`] plus the pending
/// rise entering its current leaf.
pub struct VersionWalk<'a> {
    cursor: VersionRegionReader<'a>,
    /// The rise entering the current plateau, until consumed; the
    /// stream's first payload is an absolute height, which is exactly
    /// the first rise (the walk enters at height 0).
    pending: Option<Rise>,
}

impl<'a> VersionWalk<'a> {
    /// Convert a signed height change to a public [`Rise`].
    fn rise(delta: BigInt) -> Option<Rise> {
        let (sign, magnitude) = delta.into_parts();
        if sign == Sign::NoSign {
            return None;
        }
        let ticks = Count(magnitude);
        Some(match sign {
            Sign::Plus => Rise::Up(ticks),
            Sign::Minus => Rise::Down(ticks),
            Sign::NoSign => unreachable!("zero returned above"),
        })
    }

    /// Open a canonical Version stream at its first constant-height region.
    pub fn open(bits: &'a Version) -> Self {
        let (cursor, first) = VersionRegionReader::open(bits);
        VersionWalk {
            pending: Self::rise(BigInt::from(first)),
            cursor,
        }
    }

    /// The rise entering the current plateau, consumed.
    ///
    /// The first take after a plateau entry yields it, every later take
    /// yields `None` — which is what makes a refinement cell carry an
    /// input's rise only on its plateau's first fragment.
    pub fn take_rise(&mut self) -> Option<Rise> {
        self.pending.take()
    }
}

/// A shape walk over one party stream: a thin visibility shim over
/// [`PartyRegionReader`] (whose per-region ownership state is already the
/// public item's payload).
pub struct PartyWalk<'a> {
    cursor: PartyRegionReader<'a>,
}

impl<'a> PartyWalk<'a> {
    /// Open a canonical party stream at its first constant region.
    pub fn open(party: &'a Party) -> Self {
        PartyWalk {
            cursor: PartyRegionReader::new(party),
        }
    }

    /// Whether the current region is owned by the party.
    pub fn owned(&self) -> bool {
        self.cursor.owned()
    }
}

/// Operations required to refine several shape walks together.
///
/// The `&mut` blanket impl lets a heterogeneous pair enter
/// [`advance_refinement`] as a slice of `&mut dyn Refine`, so the law
/// has one implementation for the homogeneous combiner and the
/// version × party overlay alike.
pub trait Refine {
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
        self.pending = Self::rise(step);
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

/// Advance every walk ending at the next shared boundary.
///
/// The deepest unfinished region is the smallest and therefore ends first.
/// After it advances, every other region ending at the same boundary advances
/// in the same round. Returns `true` when all walks are on their final region.
///
/// An exhausted walk's plateau runs to the unit interval's right edge,
/// so it is never the deepest side and never reaches a flip level (both
/// would put its end strictly inside the interval); it simply spans
/// every remaining cell. The empty slice reports done immediately: with
/// no boundaries to cross, the single all-interval cell is final.
pub fn advance_refinement<W: Refine>(walks: &mut [W]) -> bool {
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
