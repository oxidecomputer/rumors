//! The shape of a [`Version`], [`Party`], or [`Clock`]: its step function
//! over the unit id interval, walked as a sequence of constant runs.
//!
//! A [`Version`] *is* a step function from the unit interval `[0, 1)` of
//! ids to event counts, and a [`Party`] is a 0/1-valued function over the
//! same interval (which ids it owns). This module is the vocabulary for
//! walking those functions directly — for renderers, analysis tooling,
//! and debuggers that want to draw or inspect a value rather than compare
//! it — without needing to parse the encoded form:
//!
//! - [`Version::shape`] yields one [`Plateau`] per maximal constant run
//!   of the version: the height change entering the run ([`Rise`]), and
//!   the dyadic interval it spans.
//! - [`Party::shape`] yields one [`Region`] per maximal constant run of
//!   the party: whether the party owns it, and the interval it spans.
//! - [`Clock::shape`] yields the clock's version plateaus overlaid with
//!   its party's ownership — the pair most renderers actually draw.
//! - [`combine`] walks any number of versions as one iterator over the
//!   coarsest common refinement of their shapes' intervals, for
//!   consumers that compare or aggregate several versions pointwise.
//!
//! Every walk borrows its value and streams in place: nothing is
//! materialized up front, draining is linear in the value's encoded
//! size, and an item allocates only when its rise magnitude exceeds two
//! machine words.
//!
//! # Intervals and heights
//!
//! Every interval in this vocabulary is *dyadic*: it has width
//! `2^-depth`, and each item carries its own `depth`. Positions are never
//! materialized as numbers — an item stream lists its intervals left to
//! right, and consecutive widths tile the unit interval exactly, so a
//! consumer that wants coordinates accumulates them (widths sum to
//! exactly 1 over any complete walk).
//!
//! Version heights travel as *rises* — the signed change entering each
//! plateau — rather than absolute values: heights are event counts with
//! no ceiling, so a delta stream is what keeps the walk linear in the
//! value's encoded size rather than in the magnitudes it reaches. The
//! walk starts at height 0 on the interval's left edge, so the first
//! plateau's rise is its absolute height and a running sum reconstructs
//! every later one; the running height is never negative.
//!
//! ```
//! use before::shape::Rise;
//! use before::Version;
//!
//! let version: Version = "(1, 1, (0, 0, 2))".parse().unwrap();
//! // Reconstruct absolute heights from the rises (u64 is enough here;
//! // `Ticks` itself has no ceiling, and converts out fallibly).
//! let mut height = 0u64;
//! let mut heights = Vec::new();
//! for plateau in version.shape() {
//!     match &plateau.rise {
//!         Some(Rise::Up(count)) => height += u64::try_from(count).unwrap(),
//!         Some(Rise::Down(count)) => height -= u64::try_from(count).unwrap(),
//!         None => {}
//!     }
//!     heights.push((height, plateau.depth));
//! }
//! // Left half at height 2, then quarters at heights 1 and 3.
//! assert_eq!(heights, vec![(2, 1), (1, 2), (3, 2)]);
//! ```
//!
//! # The shape is an exact rendering of the value
//!
//! A value and its item sequence determine each other exactly: two
//! versions are equal iff their plateau sequences are equal, two parties
//! iff their region sequences are. These iterators are therefore also an
//! independently-checked witness of the crate's own semantics.

use core::iter::FusedIterator;

use crate::version::skyline::shape::{advance_refinement, PartyWalk, Refine, VersionWalk};
use crate::{Clock, Party, Ticks, Version};

#[cfg(test)]
mod tests;

/// One plateau of a version's shape: the height change entering it, and
/// the dyadic interval it spans.
///
/// A *plateau* is one maximal constant run of the version's step
/// function. A shape walk yields plateaus left to right; see the
/// [module docs](self) for how rises and widths reconstruct the
/// function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plateau {
    /// The height change entering this plateau; `None` continues level.
    ///
    /// The first plateau's rise is its absolute height (`None` if the
    /// shape starts at 0): the walk begins at height 0 on the interval's
    /// left edge. `None` occurs mid-stream too: two equal-height
    /// plateaus separated by a subtree boundary are a real shape.
    pub rise: Option<Rise>,
    /// The plateau spans a dyadic interval of width `2^-depth`.
    pub depth: u64,
}

/// A nonzero vertical move of a shape.
///
/// The sign is notated by the variant; the magnitude is in the payload.
///
/// Magnitudes are always nonzero: the level step is spelled once, as
/// `None` in [`Plateau::rise`], not as a zero rise, so every `Rise` a
/// walk yields moves the height.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rise {
    /// The height increases by the contained (nonzero) count.
    Up(Ticks),
    /// The height decreases by the contained (nonzero) count.
    Down(Ticks),
}

/// One constant-ownership region of a party's shape: whether the [`Party`]
/// owns it, and the dyadic interval it spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    /// Whether the party owns this region's identity space.
    pub owned: bool,
    /// The region spans a dyadic interval of width `2^-depth`.
    pub depth: u64,
}

/// One cell of a [`combine`]d walk: the refinement interval, and the rise
/// entering it from each input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell<const N: usize> {
    /// The cell spans a dyadic interval of width `2^-depth`.
    pub depth: u64,
    /// One entry per input, in [`combine`] argument order: the rise
    /// entering this cell from that input.
    ///
    /// `None` continues level — including on every fragment after the
    /// first when a cell subdivides an input's plateau: an input's rise
    /// appears exactly once, on the first cell of the plateau it enters.
    pub rises: [Option<Rise>; N],
}

/// An iterator over the plateaus of a version's shape, yielded left to
/// right; see [`Version::shape`].
///
/// The walk borrows the version and streams its stored form in place.
/// It is [fused](FusedIterator) but not exact-size: the plateau count is
/// not known without a full scan.
pub struct Plateaus<'a> {
    walk: VersionWalk<'a>,
    finished: bool,
}

impl<'a> Plateaus<'a> {
    /// Open a version's shape at its first plateau.
    pub(crate) fn of_version(version: &'a Version) -> Self {
        Plateaus {
            walk: VersionWalk::open(version.view().live()),
            finished: false,
        }
    }
}

impl Iterator for Plateaus<'_> {
    type Item = Plateau;

    fn next(&mut self) -> Option<Plateau> {
        if self.finished {
            return None;
        }
        let plateau = Plateau {
            rise: self.walk.take_rise(),
            depth: self.walk.depth(),
        };
        if self.walk.done() {
            self.finished = true;
        } else {
            self.walk.advance();
        }
        Some(plateau)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {
            (0, Some(0))
        } else {
            (1, None)
        }
    }
}

impl FusedIterator for Plateaus<'_> {}

/// An iterator over the regions of a party's shape, yielded left to
/// right; see [`Party::shape`].
///
/// The walk borrows the party and streams its stored form in place. It
/// is [fused](FusedIterator) but not exact-size: the region count is not
/// known without a full scan.
pub struct Regions<'a> {
    walk: PartyWalk<'a>,
    finished: bool,
}

impl<'a> Regions<'a> {
    /// Open a party's shape at its first region.
    pub(crate) fn of_party(party: &'a Party) -> Self {
        Regions {
            walk: PartyWalk::open(party.as_bits()),
            finished: false,
        }
    }
}

impl Iterator for Regions<'_> {
    type Item = Region;

    fn next(&mut self) -> Option<Region> {
        if self.finished {
            return None;
        }
        let region = Region {
            owned: self.walk.owned(),
            depth: self.walk.depth(),
        };
        if self.walk.done() {
            self.finished = true;
        } else {
            self.walk.advance();
        }
        Some(region)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {
            (0, Some(0))
        } else {
            (1, None)
        }
    }
}

impl FusedIterator for Regions<'_> {}

/// An iterator over a clock's version plateaus overlaid with its party's
/// ownership; see [`Clock::shape`].
///
/// Items are `(Plateau, bool)`: one fragment of the version's shape, and
/// whether the clock's party owns that fragment's interval. Where the
/// party subdivides a version plateau the plateau is split: the first
/// fragment carries the plateau's rise, later fragments continue level
/// (`rise: None`). Therefore, this stream is a *refinement* of the version's
/// shape, not a transliteration of it; the exact walk-is-the-value
/// correspondence lives on [`Version::shape`] and [`Party::shape`].
///
/// The walk borrows the clock and streams both stored forms in place. It
/// is [fused](FusedIterator) but not exact-size.
pub struct Overlay<'a> {
    version: VersionWalk<'a>,
    party: PartyWalk<'a>,
    finished: bool,
}

impl<'a> Overlay<'a> {
    /// Open a clock's overlay walk at its first fragment.
    pub(crate) fn of_clock(clock: &'a Clock) -> Self {
        Overlay {
            version: VersionWalk::open(clock.version().view().live()),
            party: PartyWalk::open(clock.party().as_bits()),
            finished: false,
        }
    }
}

impl Iterator for Overlay<'_> {
    type Item = (Plateau, bool);

    fn next(&mut self) -> Option<(Plateau, bool)> {
        if self.finished {
            return None;
        }
        let plateau = Plateau {
            rise: self.version.take_rise(),
            depth: self.version.depth().max(self.party.depth()),
        };
        let owned = self.party.owned();
        self.finished = advance_refinement(&mut [
            &mut self.version as &mut dyn Refine,
            &mut self.party as &mut dyn Refine,
        ]);
        Some((plateau, owned))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {
            (0, Some(0))
        } else {
            (1, None)
        }
    }
}

impl FusedIterator for Overlay<'_> {}

/// Walk `N` versions as one iterator over the coarsest common
/// refinement of their shapes' plateau intervals.
///
/// Each yielded [`Cell`] is the largest dyadic interval every input's
/// current plateau spans (dyadic intervals nest or are disjoint, so the
/// common refinement's cells are themselves dyadic and each is some
/// input's own plateau interval), with the rise entering it from each
/// input.
///
/// `N = 0` yields the trivial refinement: one all-interval cell of depth
/// 0 with no entries.
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/shape_combine.html"))]
///
/// # Example
///
/// ```
/// use before::{shape::combine, Version};
///
/// let a: Version = "(0, 1, (0, 0, 2))".parse().unwrap();
/// let b: Version = "2".parse().unwrap();
/// let cells: Vec<_> = combine([&a, &b]).collect();
/// // Three cells: `a` subdivides the right half, `b`'s single plateau
/// // spans everything (its rise enters at the first cell only).
/// assert_eq!(cells.len(), 3);
/// assert_eq!(cells[0].depth, 1);
/// assert!(cells[0].rises[1].is_some()); // b's absolute height, once
/// assert!(cells[1].rises[1].is_none()); // b continues level
/// ```
pub fn combine<'a, const N: usize>(versions: [&'a Version; N]) -> Cells<'a, N> {
    Cells {
        walks: versions.map(|version| VersionWalk::open(version.view().live())),
        finished: false,
    }
}

/// An iterator over the cells of [`combine`]d version shapes, yielded
/// left to right.
///
/// The walk borrows the versions and streams every stored form in place.
/// It is [fused](FusedIterator) but not exact-size.
pub struct Cells<'a, const N: usize> {
    walks: [VersionWalk<'a>; N],
    finished: bool,
}

impl<const N: usize> Iterator for Cells<'_, N> {
    type Item = Cell<N>;

    fn next(&mut self) -> Option<Cell<N>> {
        if self.finished {
            return None;
        }
        let depth = self.walks.iter().map(Refine::depth).max().unwrap_or(0);
        let rises = self.walks.each_mut().map(VersionWalk::take_rise);
        self.finished = advance_refinement(&mut self.walks);
        Some(Cell { depth, rises })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {
            (0, Some(0))
        } else {
            (1, None)
        }
    }
}

impl<const N: usize> FusedIterator for Cells<'_, N> {}
