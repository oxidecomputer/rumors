//! The query representation and its two verdicts.
//!
//! A [`Query`] is the interval-minus-holes normal form the rest of
//! the module constructs into: optional floor and ceiling, a hole
//! antichain, and a phantom polarity. Its two observations —
//! [`contains`](Query::contains) and [`coverage`](Query::coverage) —
//! compile the stored bounds into per-bound [`Demand`]s and hand them
//! to the fused overlay walks in the skyline `filter` kernel; the
//! clamp refinement behind coverage's `Partial` arm, where the
//! polarity exactness argument lands, is here too.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use super::polarity::{Hole, Neutral, Polarity};
use super::{le, Span, Version};
use crate::codec::BitsSlice;
use crate::version::skyline::place::filter::{self, Demand};

/// A causal query: an inclusive interval minus the holes its
/// [`Polarity`] admits.
///
/// Build one with `&` from atoms, negations, and queries of
/// compatible polarity, or with the named forms [`all`](super::all),
/// [`since`](super::since), [`until`](super::until),
/// [`delta`](super::delta), [`toward`](super::toward),
/// [`strictly_after`](super::strictly_after), and
/// [`strictly_before`](super::strictly_before). Ask it
/// [`contains`](Self::contains) for one version, or
/// [`coverage`](Self::coverage) for everything a [`Span`] covers.
///
/// A query borrows the versions it was built from (owned versions may
/// be moved in instead); [`into_owned`](Self::into_owned) settles the
/// borrows for a query held long-term.
pub struct Query<'a, P: Polarity = Neutral> {
    pub(super) floor: Option<Cow<'a, Version>>,
    pub(super) ceiling: Option<Cow<'a, Version>>,
    pub(super) holes: Vec<Hole<'a>>,
    pub(super) polarity: PhantomData<P>,
}

/// Clones by sharing every stored version's buffer, `O(1)` per bound.
///
/// Manual, not derived: the polarity parameter is a phantom marker,
/// which a derive would needlessly require to be `Clone` itself.
impl<'a, P: Polarity> Clone for Query<'a, P> {
    fn clone(&self) -> Self {
        Query {
            floor: self.floor.clone(),
            ceiling: self.ceiling.clone(),
            holes: self.holes.clone(),
            polarity: PhantomData,
        }
    }
}

/// How much of a [`Span`]'s segment a [`Query`] admits: the verdict a
/// filtered tree walk consumes per subtree.
///
/// The verdict is **exact** for every constructible query; exactness
/// is what the polarity boundary buys (see the [module docs](super)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Coverage {
    /// Every version the span covers is admitted by the query.
    Full,
    /// Some covered versions are admitted and some are not.
    Partial,
    /// No version the span covers is admitted by the query.
    Empty,
}

impl<'a, P: Polarity> Query<'a, P> {
    /// The empty conjunction at this polarity: no constraints.
    pub(super) fn unbounded() -> Self {
        Query {
            floor: None,
            ceiling: None,
            holes: Vec::new(),
            polarity: PhantomData,
        }
    }

    /// The stored bounds as (stream, demand) pairs, in the walks'
    /// deterministic read order: floor, holes in stored order,
    /// ceiling.
    fn demands(&self) -> impl Iterator<Item = (&BitsSlice, Demand)> {
        self.floor
            .as_deref()
            .map(|p| (&**p.view(), Demand::After))
            .into_iter()
            .chain(
                self.holes
                    .iter()
                    .map(|hole| (&**hole.at.view(), P::hole_demand(hole.strict))),
            )
            .chain(
                self.ceiling
                    .as_deref()
                    .map(|e| (&**e.view(), Demand::Before)),
            )
    }

    /// Whether the query admits `version`: at or above the floor, at
    /// or below the ceiling, and in none of the holes.
    pub fn contains(&self, version: &Version) -> bool {
        filter::admits(version.view(), self.demands())
    }

    /// How much of `span`'s segment this query admits, as an exact
    /// [`Coverage`] verdict.
    ///
    /// The probe is anything [`Into`] a [`Span`]: a span itself, or a
    /// [`Version`] (borrowed or owned), which probes the coincident
    /// span — there the verdict reduces to
    /// [`contains`](Self::contains), [`Full`](Coverage::Full) for a
    /// member and [`Empty`](Coverage::Empty) otherwise.
    pub fn coverage<'s>(&self, span: impl Into<Span<'s>>) -> Coverage {
        let span = span.into();
        let (lo, hi) = (span.meet(), span.join());
        if lo.view().ptr_eq(hi.view()) {
            return if self.contains(lo) {
                Coverage::Full
            } else {
                Coverage::Empty
            };
        }
        match filter::coverage(lo.view(), hi.view(), self.demands()) {
            Coverage::Full => Coverage::Full,
            Coverage::Empty => Coverage::Empty,
            Coverage::Partial => self.refine_partial(lo, hi),
        }
    }

    /// The clamp refinement behind [`coverage`](Self::coverage)'s
    /// `Partial` arm: the exact emptiness decision the fused
    /// endpoint fold cannot reach.
    ///
    /// The admitted portion of the segment is the *clamped* segment
    /// `[lo ∨ floor, hi ∧ ceiling]` minus the holes. A crossed clamp
    /// is empty outright. A down-set covering the clamped top covers
    /// the whole clamped segment — so per-hole checks against the
    /// clamped endpoint on the hole's own side decide every covering
    /// (dually for up-sets and the clamped bottom), *because* every
    /// hole shares one polarity: the joint-covering case that would
    /// escape per-hole checks needs both polarities at once, which
    /// the type refuses. This is where the exactness argument lands.
    fn refine_partial(&self, lo: &Version, hi: &Version) -> Coverage {
        let clamped_lo: Cow<'_, Version> = match self.floor.as_deref() {
            Some(floor) => Cow::Owned(lo | floor),
            None => Cow::Borrowed(lo),
        };
        let clamped_hi: Cow<'_, Version> = match self.ceiling.as_deref() {
            Some(ceiling) => Cow::Owned(hi & ceiling),
            None => Cow::Borrowed(hi),
        };
        if !le(&clamped_lo, &clamped_hi)
            || self
                .holes
                .iter()
                .any(|hole| P::hole_covers(hole, &clamped_lo, &clamped_hi))
        {
            Coverage::Empty
        } else {
            Coverage::Partial
        }
    }

    /// Settles every stored version owned, erasing the borrow
    /// lifetime, so the query outlives what it was built from — for a
    /// filter held long-term.
    ///
    /// # Complexity
    ///
    /// `O(1)` per stored bound: owned versions move, borrowed ones
    /// clone by sharing their stored buffers.
    pub fn into_owned(self) -> Query<'static, P> {
        Query {
            floor: self.floor.map(|p| Cow::Owned(p.into_owned())),
            ceiling: self.ceiling.map(|e| Cow::Owned(e.into_owned())),
            holes: self
                .holes
                .into_iter()
                .map(|hole| Hole {
                    at: Cow::Owned(hole.at.into_owned()),
                    strict: hole.strict,
                })
                .collect(),
            polarity: PhantomData,
        }
    }
}

impl<'a> Query<'a, Neutral> {
    /// Recasts the hole-free query at any polarity: a neutral query
    /// holds no holes structurally, so the reinterpretation moves no
    /// meaning.
    pub(super) fn adopt<P: Polarity>(self) -> Query<'a, P> {
        debug_assert!(self.holes.is_empty(), "a neutral query holds no holes");
        Query {
            floor: self.floor,
            ceiling: self.ceiling,
            holes: Vec::new(),
            polarity: PhantomData,
        }
    }
}

// Debug renders the module's own expression vocabulary — the only
// structural window into a query (there is deliberately no `Eq`; see
// the module docs), so failures and logs read as an expression
// denoting the same predicate. (Strict holes render as the negated
// strict atoms they equal, `!strictly_before(v)` — spellings reached
// through `or_concurrent` in the operator language.)
impl<P: Polarity> fmt::Debug for Query<'_, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut lead = |f: &mut fmt::Formatter<'_>| -> fmt::Result {
            if !first {
                write!(f, " & ")?;
            }
            first = false;
            Ok(())
        };
        if let Some(floor) = self.floor.as_deref() {
            lead(f)?;
            write!(f, "after({floor:?})")?;
        }
        for hole in &self.holes {
            lead(f)?;
            write!(f, "{}({:?})", P::hole_name(hole.strict), hole.at)?;
        }
        if let Some(ceiling) = self.ceiling.as_deref() {
            lead(f)?;
            write!(f, "before({ceiling:?})")?;
        }
        if first {
            write!(f, "all()")?;
        }
        Ok(())
    }
}
