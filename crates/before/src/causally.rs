//! Named, composable constructors for causal [`Version`] ranges.
//!
//! On totally ordered values, a range is an interval. On causal [`Version`]s,
//! which are only *partially* ordered, the useful generalization is a
//! **difference of down-sets**: all versions contained in the end bound,
//! excepting the versions also contained in the start bound. The constructors
//! here name each bound's meaning so a filter reads as a sentence, and every
//! start kind composes with every end kind:
//!
//! | | end unbounded | [`known_at(e)`](known_at): `v <= e` | [`before(e)`](before): `v < e` |
//! |---|---|---|---|
//! | **start unbounded** | [`all()`](all) | `known_at(&e)` | `before(&e)` |
//! | **[`not_before(s)`](not_before): subtract `v < s`** | `not_before(&s)` | `not_before(&s).known_at(&e)` | `not_before(&s).before(&e)` |
//! | **[`since(s)`](since): subtract `v <= s`** | `since(&s)` | `since(&s).known_at(&e)`, a.k.a. [`delta`] | `since(&s).before(&e)`, a.k.a. [`delta_before`] |
//!
//! The asymmetry inherent to the partial order: a start bound of either kind
//! keeps versions *concurrent* to it: "everything since `start`" must not drop
//! other parties' concurrent versions; conversely, "everything before `end`"
//! must instead drop everything concurrent to it.
//!
//! Pairing a start with an end validates the composition: the start version
//! must lie *within* the end bound (`start <= end` under [`known_at`], `start <
//! end` under [`before`]), and a pair that crosses is rejected with
//! [`Crossed`]. The gate is what makes [`placement_of`](Range::placement_of)'s
//! trichotomy total: a range that exists subtracts only versions its end bound
//! keeps, so no version can fail both bounds at once.
//!
//! Every constructor returns a [`Range`], which implements
//! [`RangeBounds<Version>`] so it can be handed to any version-ranged API, and
//! offers [`contains`](Range::contains) as the authoritative membership
//! predicate.
//!
//! # Placement
//!
//! Every membership question a range answers is a coarsening of one
//! *placement*: where a version sits relative to the range. [`Range::bounded`]
//! answers it at full resolution: the six [`Bounded`] verdicts form an ordered
//! line `Before` < `AtStart` < `Between` < `AtEnd` < `After`, with `Concurrent`
//! off the axis beside the end, and [`placement_of`](Range::placement_of) folds
//! those six down to by each bound's inclusivity, with
//! [`contains`](Range::contains) as the `Equal` arm.
//!
//! # [`Span`] placement
//!
//! Two concrete versions `lo <= hi` form a [`Span`], which is a genuinely
//! different object from a [`Range`]: while a range's bounds are down-set
//! cut-points, a span is the ordered pair itself and the segment between its
//! versions. [`Span::place`] answers the placement question at the finest
//! resolution the partial order admits, and [`Span::dominance`] coarsens it to
//! the three-way [`Dominance`] verdict a filter over version-bounded regions
//! consumes. Spans, their operator algebra, their party-quotient view
//! ([`OwnSpan`]), and their wire form live in the crate's span module and are
//! re-exported here.
//!
//! # Complexity
//!
//! A [`Range`] or [`Span`] stores two borrows. Pairing a start with an end, or
//! validating a span through [`Span::new`], costs one comparison in the bounds'
//! packed sizes, `|s| + |e|`. Determining the placement of a [`Version`] `v` in
//! a range `s..e` is one fused pass `O(|v| + |s| + |e|)`; likewise for all
//! other range forms, and for [`Span`]s.
//!
//! ```
//! use before::{Clock, causally};
//!
//! let mut alice = Clock::seed();
//! let mut bob = alice.fork();
//! let a1 = alice.tick().clone();
//! let b1 = bob.tick().clone(); // concurrent to a1
//! let a2 = alice.tick().clone(); // a1 < a2
//!
//! // A start bound subtracts only its causal past: versions concurrent to
//! // it pass.
//! assert!(causally::since(&a1).contains(&a2));
//! assert!(causally::since(&a1).contains(&b1));
//! assert!(!causally::since(&a1).contains(&a1));
//! // `not_before` differs only at the bound itself.
//! assert!(causally::not_before(&a1).contains(&a1));
//!
//! // An end bound demands containment: concurrent versions are dropped.
//! assert!(causally::known_at(&a2).contains(&a1));
//! assert!(!causally::known_at(&a2).contains(&b1));
//!
//! // Every start kind composes with every end kind, in either order —
//! // provided the start lies within the end.
//! let range = causally::since(&a1).known_at(&a2).unwrap();
//! assert!(range.contains(&a2));
//! assert!(!range.contains(&b1));
//! assert_eq!(causally::delta(&a1, &a2).unwrap(), range);
//! // A crossed pair is rejected at composition: b1 is not within a1.
//! assert!(causally::delta(&b1, &a1).is_err());
//! ```

use std::cmp::Ordering;
use std::ops::{Bound, RangeBounds};

pub use crate::error::Crossed;
pub use crate::span::{Dominance, Endpoint, OwnSpan, Placement, Span};

use crate::version::skyline::place;
use crate::Version;

/// A causal version range comprising a pair of [`Bound`]s.
///
/// Build one with the module's constructors and refine it with the same-named
/// methods, in either order; setting a bound that is already set keeps the
/// latest value. Refinement validates the pair, so every `Range` that exists is
/// well-formed. The struct implements [`RangeBounds<Version>`] for use with
/// version-ranged APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range<'a> {
    start: Bound<&'a Version>,
    end: Bound<&'a Version>,
}

/// The unbounded range: every version.
///
/// The identity for composition; refine it with any of [`Range`]'s methods.
pub fn all<'a>() -> Range<'a> {
    Range {
        start: Bound::Unbounded,
        end: Bound::Unbounded,
    }
}

/// Everything *strictly since* `start`: its causal future and everything
/// concurrent to it.
///
/// `start` itself is excluded; this is the resume/subscription shape, where
/// the boundary version has already been seen.
pub fn since(start: &Version) -> Range<'_> {
    Range {
        start: Bound::Excluded(start),
        end: Bound::Unbounded,
    }
}

/// Everything *not strictly before* `start`: like [`since`], but `start` itself
/// is included.
///
/// On a partial order, "not before" is unambiguous where "at or after" would
/// not be, since concurrent versions are neither.
pub fn not_before(start: &Version) -> Range<'_> {
    Range {
        start: Bound::Included(start),
        end: Bound::Unbounded,
    }
}

/// Everything *known at* `end`: its causal past, inclusive.
pub fn known_at(end: &Version) -> Range<'_> {
    Range {
        start: Bound::Unbounded,
        end: Bound::Included(end),
    }
}

/// Everything *strictly before* `end`: versions contained in `end`,
/// exclusive of `end` itself.
pub fn before(end: &Version) -> Range<'_> {
    Range {
        start: Bound::Unbounded,
        end: Bound::Excluded(end),
    }
}

/// The causal delta from `start` to `end`: everything known at `end` but not at
/// `start`.
///
/// Shorthand for [`since(start)`](since)[`.known_at(end)`](Range::known_at).
///
/// # Errors
///
/// [`Crossed`] unless `start <= end`.
pub fn delta<'a>(start: &'a Version, end: &'a Version) -> Result<Range<'a>, Crossed> {
    since(start).known_at(end)
}

/// The half-open causal delta: everything strictly since `start` and
/// strictly before `end`.
///
/// Shorthand for [`since(start)`](since)[`.before(end)`](Range::before).
///
/// # Errors
///
/// [`Crossed`] unless `start < end`.
pub fn delta_before<'a>(start: &'a Version, end: &'a Version) -> Result<Range<'a>, Crossed> {
    since(start).before(end)
}

impl<'a> Range<'a> {
    /// Refines the start bound to *strictly since* `start` (see [`since`]).
    ///
    /// # Errors
    ///
    /// [`Crossed`] if `start` is not within the end bound: unless `start <=
    /// end` under [`known_at`](Self::known_at), unless `start < end` under
    /// [`before`](Self::before). An unbounded end accepts every start.
    pub fn since(self, start: &'a Version) -> Result<Self, Crossed> {
        Self {
            start: Bound::Excluded(start),
            ..self
        }
        .validated()
    }

    /// Refines the start bound to *not strictly before* `start` (see
    /// [`not_before`]).
    ///
    /// # Errors
    ///
    /// [`Crossed`] if `start` is not within the end bound: unless `start <=
    /// end` under [`known_at`](Self::known_at), unless `start < end` under
    /// [`before`](Self::before). An unbounded end accepts every start.
    pub fn not_before(self, start: &'a Version) -> Result<Self, Crossed> {
        Self {
            start: Bound::Included(start),
            ..self
        }
        .validated()
    }

    /// Refines the end bound to *known at* `end` (see [`known_at`]).
    ///
    /// # Errors
    ///
    /// [`Crossed`] unless the start version, if any, satisfies `start <= end`.
    /// An unbounded start accepts every end.
    pub fn known_at(self, end: &'a Version) -> Result<Self, Crossed> {
        Self {
            end: Bound::Included(end),
            ..self
        }
        .validated()
    }

    /// Refines the end bound to *strictly before* `end` (see [`before`]).
    ///
    /// # Errors
    ///
    /// [`Crossed`] unless the start version, if any, satisfies `start < end`.
    /// An unbounded start accepts every end.
    pub fn before(self, end: &'a Version) -> Result<Self, Crossed> {
        Self {
            end: Bound::Excluded(end),
            ..self
        }
        .validated()
    }

    /// The well-formedness gate every refinement passes through: the start
    /// version, if any, must lie within the end bound, if any.
    ///
    /// The gate makes [`placement_of`](Self::placement_of) a coherent
    /// trichotomy. A subtracted version sits at or below the start, and a start
    /// within the end bound pulls everything at or below it within too (the
    /// strictness required of `start` vs `end` matches the end bound's own
    /// strictness), so everything the start subtracts the end keeps: no version
    /// is both below the range and beyond it.
    fn validated(self) -> Result<Self, Crossed> {
        let start = match self.start {
            Bound::Unbounded => return Ok(self),
            Bound::Included(start) | Bound::Excluded(start) => start,
        };
        let within_end = match self.end {
            Bound::Unbounded => true,
            Bound::Included(end) => start <= end,
            Bound::Excluded(end) => start < end,
        };
        if within_end {
            Ok(self)
        } else {
            Err(Crossed)
        }
    }

    /// The causal membership predicate: whether `version` is contained in the
    /// end bound and *not* contained in the start bound.
    ///
    /// Equivalent to [`placement_of`](Self::placement_of) returning
    /// [`Equal`](Ordering::Equal).
    ///
    /// Per bound kind, for a version `v`:
    ///
    /// - start unbounded: nothing subtracted; [`since(s)`](since): `v <= s`
    ///   subtracted; [`not_before(s)`](not_before): `v < s` subtracted.
    /// - end unbounded: everything kept; [`known_at(e)`](known_at): `v <= e`
    ///   kept; [`before(e)`](before): `v < e` kept.
    ///
    /// The [`RangeBounds`] impl overrides the trait's provided `contains` to
    /// answer identically, so generic [`RangeBounds`] consumers reach the same
    /// verdict as this method.
    pub fn contains(&self, version: &Version) -> bool {
        self.placement_of(version) == Ordering::Equal
    }

    /// Totally orders `version` against this range.
    ///
    /// Where the causal order on [`Version`]s alone is partial, a version's
    /// placement relative to a range is always one of exactly three cases:
    ///
    /// - [`Less`](Ordering::Less): the start bound subtracts it; it is in
    ///   the range's past.
    /// - [`Equal`](Ordering::Equal): the range [`contains`](Self::contains)
    ///   it.
    /// - [`Greater`](Ordering::Greater): the end bound does not contain it
    ///   (its causal future *or* something concurrent to it); "beyond the
    ///   range", not necessarily after every version in it.
    pub fn placement_of(&self, version: &Version) -> Ordering {
        match self.bounded(version) {
            Bounded::Before => Ordering::Less,
            // Raw equality to the start: the start kind decides whether
            // the bound itself is subtracted.
            Bounded::AtStart => match self.start {
                Bound::Excluded(_) => Ordering::Less,
                Bound::Included(_) => Ordering::Equal,
                Bound::Unbounded => unreachable!("an at-start verdict requires a start bound"),
            },
            Bounded::Between => Ordering::Equal,
            // Raw equality to the end: the end kind decides whether the
            // bound itself is kept.
            Bounded::AtEnd => match self.end {
                Bound::Included(_) => Ordering::Equal,
                Bound::Excluded(_) => Ordering::Greater,
                Bound::Unbounded => unreachable!("an at-end verdict requires an end bound"),
            },
            Bounded::After | Bounded::Concurrent => Ordering::Greater,
        }
    }

    /// Places `version` against this range at full resolution: the
    /// six-way [`Bounded`] verdict.
    ///
    /// ```
    /// use before::{Clock, causally::{self, Bounded}};
    ///
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let b1 = bob.tick().clone(); // concurrent to everything of alice's
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// let range = causally::delta(&a1, &a3).unwrap();
    /// assert_eq!(range.bounded(&a1), Bounded::AtStart);
    /// assert_eq!(range.bounded(&a2), Bounded::Between);
    /// assert_eq!(range.bounded(&a3), Bounded::AtEnd);
    /// // Concurrent to the start but within the end: contained, so
    /// // `Between` — never `Concurrent`, which is an end-bound verdict.
    /// let top = &a3 | &b1;
    /// let wide = causally::delta(&a1, &top).unwrap();
    /// assert_eq!(wide.bounded(&b1), Bounded::Between);
    /// ```
    pub fn bounded(&self, version: &Version) -> Bounded {
        let stream = |bound: Bound<&'a Version>| match bound {
            Bound::Included(v) | Bound::Excluded(v) => Some(&**v.view()),
            Bound::Unbounded => None,
        };
        match place::range(version.view(), stream(self.start), stream(self.end)) {
            place::Ranged::BelowStart => Bounded::Before,
            place::Ranged::AtStart => Bounded::AtStart,
            place::Ranged::Inside => Bounded::Between,
            place::Ranged::AtEnd => Bounded::AtEnd,
            place::Ranged::AboveEnd => Bounded::After,
            place::Ranged::ConcurrentToEnd => Bounded::Concurrent,
        }
    }
}

/// Where a version sits relative to a [`Range`].
///
/// The variants read as an ordered line: `Before` < `AtStart` < `Between` <
/// `AtEnd` < `After`, with [`Concurrent`](Self::Concurrent) off the axis beside
/// the end. The region variants (`Before`, `Between`, `After`, `Concurrent`)
/// follow the range semantics (a difference of causal down-sets; see the
/// [module docs](self)); the at-bound variants (`AtStart`, `AtEnd`) report *raw
/// equality* to a bound's version, deliberately independent of the bound's
/// kind: whether the range keeps or subtracts a version sitting exactly at a
/// bound is the bound's inclusivity question, answered in the coarsening to
/// [`placement_of`](Range::placement_of).
///
/// Note that `Concurrent` is an **end-bound** verdict. A version *concurrent to
/// the start bound but within the end bound* is [`Between`](Self::Between) —
/// start bounds subtract only their causal past, so versions concurrent to a
/// start are kept, the module's deliberate keep-concurrent-versions behavior.
///
/// **Vocabulary kinship, divergent semantics**: span placement's [`Placement`]
/// reuses `Before`, `Between`, and `After` — the same question against a
/// different object. There the words are raw strict-order facts against two
/// concrete versions (a version concurrent to the span's start is
/// `Concurrent(Start)`, never `Between`); here the region variants fold the
/// range semantics above. [`Dominance`] reuses the same three words a third
/// way, coarser than either: each of its verdicts is a *bucket* of `Placement`s
/// keyed to how much of the span the probe dominates, folding concurrencies and
/// endpoint hits into the buckets (its variant docs carry the exact tables). On
/// a two-bounded range this verdict is exactly a coarsening of the nine-state
/// placement, pinned by the `bounded_coarsens_span_place` law in
/// [`laws`](crate::laws).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bounded {
    /// Strictly inside the start bound's causal past: `v < start`.
    ///
    /// Subtracted under either start kind.
    Before,
    /// Exactly the start bound's version: `v == start`.
    ///
    /// Subtracted by [`since`], kept by [`not_before`] — the coarsening
    /// to [`placement_of`](Range::placement_of) decides which.
    ///
    /// When the two bounds coincide (a validated range permits
    /// `start == end`), a version equal to both is at the start *and* the
    /// end; it reports `AtStart`, never [`AtEnd`](Self::AtEnd). The start
    /// bound speaks first — subtraction precedes containment, mirroring
    /// the order [`placement_of`](Range::placement_of) checks its bounds —
    /// and the precedence is load-bearing: under an excluded start with an
    /// included end (`since(x).known_at(x)`, a validated composition), the
    /// shared bound is subtracted, which only `AtStart`'s coarsening
    /// (`Less` under an excluded start) reports; `AtEnd` would coarsen to
    /// `Equal` there and misplace it.
    AtStart,
    /// Contained, at neither bound: past the start (dominating it or
    /// concurrent to it — or no start bound at all) and within the end.
    Between,
    /// Exactly the end bound's version: `v == end`.
    ///
    /// Kept by [`known_at`], dropped by [`before`] — the coarsening
    /// decides which.
    AtEnd,
    /// Beyond the end bound in its causal future: `end < v`.
    After,
    /// Beyond the end bound by incomparability: `v` and the end bound are
    /// concurrent, so the end cannot contain `v`.
    ///
    /// Specifically an end-bound verdict: a version concurrent to the
    /// *start* bound but within the end is [`Between`](Self::Between),
    /// not `Concurrent`.
    Concurrent,
}

impl RangeBounds<Version> for Range<'_> {
    fn start_bound(&self) -> Bound<&Version> {
        self.start
    }

    fn end_bound(&self) -> Bound<&Version> {
        self.end
    }

    /// The causal membership predicate, overriding the trait's provided
    /// body: keep the item unless the start bound *subtracts* it, and
    /// the end bound must contain it.
    ///
    /// Subtraction is `item <= start` under an excluded start and
    /// `item < start` under an included one. The provided body instead
    /// requires the item to *dominate* the start bound
    /// (`start <= item` / `start < item`), which on a partial order
    /// silently drops versions concurrent to the start — versions
    /// [`Range::contains`] keeps. For `Version` probes this override and
    /// the inherent method agree exactly (pinned in the module's tests).
    fn contains<U>(&self, item: &U) -> bool
    where
        Version: PartialOrd<U>,
        U: ?Sized + PartialOrd<Version>,
    {
        (match self.start {
            Bound::Included(start) => !(item < start),
            Bound::Excluded(start) => !(item <= start),
            Bound::Unbounded => true,
        }) && (match self.end {
            Bound::Included(end) => item <= end,
            Bound::Excluded(end) => item < end,
            Bound::Unbounded => true,
        })
    }
}

#[cfg(test)]
mod tests;
