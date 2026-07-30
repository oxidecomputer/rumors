//! Named, composable constructors for causal [`Version`] ranges.
//!
//! On totally ordered values a range is an interval. On causal [`Version`]s,
//! which are only *partially* ordered, the useful generalization is a
//! **difference of down-sets**: keep the versions contained in the end bound,
//! subtract the versions contained in the start bound. The constructors here
//! name each bound's meaning so a filter reads as a sentence, and every start
//! kind composes with every end kind:
//!
//! | | end unbounded | [`known_at(e)`](known_at): `v <= e` | [`before(e)`](before): `v < e` |
//! |---|---|---|---|
//! | **start unbounded** | [`all()`](all) | `known_at(&e)` | `before(&e)` |
//! | **[`not_before(s)`](not_before): subtract `v < s`** | `not_before(&s)` | `not_before(&s).known_at(&e)` | `not_before(&s).before(&e)` |
//! | **[`since(s)`](since): subtract `v <= s`** | `since(&s)` | `since(&s).known_at(&e)`, a.k.a. [`delta`] | `since(&s).before(&e)`, a.k.a. [`delta_before`] |
//!
//! The asymmetry inherent to the partial order: a start bound of either kind
//! keeps versions *concurrent* to it (subtraction removes only the bound's
//! causal past — "everything since `s`" must not drop other parties' concurrent
//! versions), while an end bound of either kind drops them (keeping demands
//! containment).
//!
//! Pairing a start with an end validates the composition: the start version
//! must lie *within* the end bound (`start <= end` under [`known_at`],
//! `start < end` under [`before`]), and a pair that crosses is rejected with
//! [`Crossed`]. The gate is what makes
//! [`placement_of`](Range::placement_of)'s trichotomy total: a range that
//! exists subtracts only versions its end bound keeps, so no version can
//! fail both bounds at once.
//!
//! Every constructor returns a [`Range`], which implements
//! [`RangeBounds<Version>`] so it can be handed to any version-ranged API, and
//! offers [`contains`](Range::contains) as the authoritative membership
//! predicate.
//!
//! # Placement
//!
//! Every membership question a range answers is a coarsening of one
//! *placement*: where a version sits relative to the range.
//! [`Range::bounded`] answers it at full resolution — the six [`Bounded`]
//! verdicts, an ordered line `Before, AtStart, Between, AtEnd, After` with
//! `Concurrent` off the axis beside the end — and
//! [`placement_of`](Range::placement_of) folds those six down to its
//! trichotomy by each bound's inclusivity, with
//! [`contains`](Range::contains) as the trichotomy's `Equal` arm. The
//! region verdicts read the range semantics above (in particular, a
//! version *concurrent to the start but within the end* is `Between`:
//! start bounds keep concurrent versions); the at-bound verdicts report
//! raw equality to a bound's version, leaving whether the range keeps
//! that version to the coarsening. [`Bounded`]'s variant docs carry the
//! exact case analysis, including the coincident-bounds corner.
//!
//! # Span placement
//!
//! Two concrete versions `lo <= hi` form a [`Span`] — a genuinely
//! different object from a [`Range`]: where a range's bounds are
//! down-set cut-points with inclusivity kinds, a span is the ordered
//! pair itself and the chain segment between its versions (on totally
//! ordered values it would be an interval), and its verdicts are raw
//! order facts with no inclusivity to fold.
//! [`Span::place`] answers the placement question at the finest
//! resolution the partial order admits — the nine [`Placement`] regions
//! — and [`Span::dominance_of`] coarsens it to the three-way
//! [`Dominance`] verdict a filter over version-bounded regions consumes.
//! Every nonempty collection of versions has a tightest containing
//! span — its lattice hull — derived by [`Version::span`] and
//! [`Version::span_all`], the total construction doors living beside
//! the join/meet family they compose.
//!
//! The module's placement questions have exactly two semantic roots:
//! pairwise comparison ([`partial_cmp`](PartialOrd::partial_cmp) on
//! [`Version`]s) and span placement. Every other verdict is a
//! lawful coarsening of one of the two, each pinned as a named law in
//! [`laws`](crate::laws): [`bounded`](Range::bounded) on a two-bounded
//! range coarsens `place` (`bounded_coarsens_span_place`), and on a
//! single-bounded range transcribes the one pairwise comparison
//! (`bounded_matches_bound_relations`);
//! [`placement_of`](Range::placement_of) and
//! [`contains`](Range::contains) coarsen `bounded` by bound kind
//! (`bounded_coarsens_to_placement`); [`dominance_of`](Span::dominance_of)
//! coarsens `place` to the dominance question
//! (`span_dominance_coarsens_place`); and `place` against the
//! degenerate span `[v, v]` is pairwise comparison itself
//! (`degenerate_span_place_is_partial_cmp`).
//!
//! # Complexity
//!
//! Every borrowing constructor in this module is `O(1)` time and space:
//! a [`Range`] or [`Span`] stores two borrows. Pairing a start with
//! an end — or validating a span through [`Span::new`] — costs
//! at most one causal comparison, `O(|s| + |e|)` in the bounds' packed
//! sizes ([`Span::ordered`] skips even that). The deriving
//! constructors live on [`Version`] ([`span`](Version::span) and
//! [`span_all`](Version::span_all), priced at the methods). The
//! placement family — [`bounded`](Range::bounded),
//! [`contains`](Range::contains),
//! [`placement_of`](Range::placement_of), [`Span::place`], and
//! [`Span::dominance_of`] — runs one fused comparison
//! pass over the version and the present bound versions, `O(|v| + |s| +
//! |e|)` in the operands' packed sizes (see [`Version`]), each stream
//! decoded once.
//!
//! **Complexity**: borrowing constructors `O(1)` (the deriving `span`/`span_all` priced on `Version`); validation at most one causal comparison; placement one fused pass `O(v + s + e)`.
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

use std::borrow::Cow;
use std::cmp::Ordering;
use std::ops::{Bound, RangeBounds};

pub use crate::error::Crossed;

use crate::version::skyline::place;
use crate::Version;

/// A causal version range: a pair of [`Bound`]s.
///
/// Build one with the module's constructors and refine it with the
/// same-named methods, in either order; setting a bound that is already set
/// keeps the latest value. Refinement validates the pair — a start that is
/// not within the end bound is rejected with [`Crossed`] — so every `Range`
/// that exists is well-formed. The struct implements
/// [`RangeBounds<Version>`] for use with version-ranged APIs.
///
/// Note that [`Range::contains`] — the causal membership predicate — is
/// deliberately *not* [`RangeBounds::contains`]: the trait's default method
/// requires the item to dominate the start bound, which on a partial order
/// silently drops versions concurrent to it. The inherent method shadows
/// the default so the natural call gets the causal semantics.
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
/// The name follows X.509's `notBefore`: on a partial order, "not before" is
/// unambiguous where "at or after" would not be, since concurrent versions are
/// neither.
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
    /// [`Crossed`] if `start` is not within the end bound: unless
    /// `start <= end` under [`known_at`](Self::known_at), unless
    /// `start < end` under [`before`](Self::before). An unbounded end
    /// accepts every start.
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
    /// [`Crossed`] if `start` is not within the end bound: unless
    /// `start <= end` under [`known_at`](Self::known_at), unless
    /// `start < end` under [`before`](Self::before). An unbounded end
    /// accepts every start.
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
    /// [`Crossed`] unless the start version, if any, satisfies
    /// `start <= end`. An unbounded start accepts every end.
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
    /// [`Crossed`] unless the start version, if any, satisfies
    /// `start < end`. An unbounded start accepts every end.
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
    /// trichotomy. A subtracted version sits at or below the start, and a
    /// start within the end bound pulls everything at or below it within
    /// too (the strictness required of `start` vs `end` matches the end
    /// bound's own strictness), so everything the start subtracts the end
    /// keeps: no version is both below the range and beyond it.
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
    /// This deliberately shadows the [`RangeBounds::contains`] default,
    /// whose start check would also drop versions concurrent to the start
    /// bound (see [`Range`]).
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
    ///   — its causal future *or* something concurrent to it; "beyond the
    ///   range", not necessarily after every version in it.
    ///
    /// The totality lives in the signature: a bare [`Ordering`], no
    /// [`Option`], where [`Version`]-to-[`Version`] comparison must return
    /// [`Option<Ordering>`](PartialOrd::partial_cmp). (No operator
    /// overloads back this: a cross-type `PartialEq` whose `==` meant
    /// membership would violate the trait's transitivity contract.)
    ///
    /// The three cases are also mutually exclusive: composition validates
    /// that the start bound lies within the end bound (rejecting the pair
    /// with [`Crossed`] otherwise), so everything the start subtracts the
    /// end keeps — no version can fail both bounds.
    ///
    /// The trichotomy is [`bounded`](Self::bounded)'s six-way placement
    /// coarsened by each bound's inclusivity — `Before → Less`;
    /// `AtStart → Less` under an excluded start, `Equal` under an
    /// included one; `Between → Equal`; `AtEnd → Equal` under an
    /// included end, `Greater` under an excluded one; `After` and
    /// `Concurrent → Greater` — so one fused comparison pass answers it,
    /// each operand stream decoded once. The
    /// `bounded_coarsens_to_placement` law in [`laws`](crate::laws) pins
    /// the table, [`contains`](Self::contains) riding as its `Equal` arm.
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
    /// The verdict is a pure function of the two causal comparisons
    /// against the bound versions — bound *kinds* never move it (they
    /// decide only how [`placement_of`](Self::placement_of) coarsens an
    /// at-bound verdict), and an unbounded side makes its verdicts
    /// unreachable: no start bound rules out [`Before`](Bounded::Before)
    /// and [`AtStart`](Bounded::AtStart); no end bound rules out
    /// [`AtEnd`](Bounded::AtEnd), [`After`](Bounded::After), and
    /// [`Concurrent`](Bounded::Concurrent).
    ///
    /// One comparison pass answers both relations: the version and the
    /// bound versions are walked simultaneously, each decoded once —
    /// against two for the version when the comparisons are composed —
    /// with the composition's early exits preserved (a version
    /// concurrent to a bound is decided at the first opposing interval).
    /// The `bounded_matches_bound_relations` law in [`laws`](crate::laws)
    /// pins the fused verdict to the two comparisons on every law
    /// consumer, and `bounded_coarsens_to_placement` pins the
    /// coarsening.
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

/// Where a version sits relative to a [`Range`]: the full-resolution
/// placement behind [`placement_of`](Range::placement_of)'s trichotomy.
///
/// The variants read as an ordered line — `Before, AtStart, Between,
/// AtEnd, After` — with [`Concurrent`](Self::Concurrent) off the axis
/// beside the end. The region variants (`Before`, `Between`, `After`,
/// `Concurrent`) follow the range semantics (a difference of causal
/// down-sets; see the [module docs](self)); the at-bound variants
/// (`AtStart`, `AtEnd`) report *raw equality* to a bound's version,
/// deliberately independent of the bound's kind: whether the range keeps
/// or subtracts a version sitting exactly at a bound is the bound's
/// inclusivity question, answered in the coarsening to
/// [`placement_of`](Range::placement_of), never baked into the variant.
/// (An `AtStart` that meant subtracted-or-kept would collapse into
/// `Before` or `Between` and add nothing.)
///
/// The one misreading to rule out: `Concurrent` is an **end-bound**
/// verdict. A version *concurrent to the start bound but within the end
/// bound* is [`Between`](Self::Between) — start bounds subtract only
/// their causal past, so versions concurrent to a start are kept, the
/// module's deliberate keep-concurrent-versions behavior.
///
/// **Vocabulary kinship, divergent semantics**: span placement's
/// [`Placement`] reuses `Before`, `Between`, and `After` — the same
/// question against a different object. There the words are raw
/// strict-order facts against two concrete versions (a version
/// concurrent to the span's start is `Concurrent(Start)`, never
/// `Between`); here the region variants fold the range semantics above.
/// [`Dominance`] reuses the same three words a third way, coarser than
/// either: each of its verdicts is a *bucket* of `Placement`s keyed to
/// how much of the span the probe dominates, folding concurrencies and
/// endpoint hits into the buckets (its variant docs carry the exact
/// tables). On a two-bounded range this verdict is exactly a coarsening
/// of the nine-state placement, pinned by the
/// `bounded_coarsens_span_place` law in [`laws`](crate::laws).
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

/// A causal span: an ordered pair of versions `lo <= hi` and the
/// chain segment between them.
///
/// A genuinely different object from [`Range`]: a range's bounds are
/// down-set cut-points with inclusivity kinds, so its verdicts fold
/// range semantics; a span is two *concrete versions*, and every
/// verdict about it is a raw order fact against the endpoints, with no
/// inclusivity to fold. [`place`](Self::place) answers the placement
/// question at full resolution — the nine [`Placement`] regions — and
/// [`dominance_of`](Self::dominance_of) coarsens it to the three-way
/// [`Dominance`] verdict.
///
/// Construction has three doors: [`new`](Self::new) validates the
/// pair, rejecting a reversed or incomparable one with [`Crossed`];
/// [`ordered`](Self::ordered) trusts a caller who already holds
/// `lo <= hi` structurally and skips the validating comparison; and
/// the derived constructors on [`Version`] — [`span`](Version::span)
/// and [`span_all`](Version::span_all), beside the join/meet family
/// they compose — *derive* the span as a collection's lattice hull,
/// total where the first two must reject or trust.
///
/// ```
/// use before::{Clock, causally::{Dominance, Endpoint, Span, Placement}};
///
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let a3 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's whole line
///
/// let span = Span::new(&a1, &a3).unwrap();
/// assert_eq!(span.place(&a2), Placement::Between);
/// assert_eq!(span.place(&b1), Placement::Concurrent(Endpoint::Both));
/// // A reversed or incomparable pair is not a span.
/// assert!(Span::new(&a3, &a1).is_err());
/// assert!(Span::new(&a1, &b1).is_err());
/// // The dominance coarsening: a3 dominates the whole span, a2
/// // only its start, and b1 not even that.
/// assert_eq!(span.dominance_of(&a3), Dominance::After);
/// assert_eq!(span.dominance_of(&a2), Dominance::Between);
/// assert_eq!(span.dominance_of(&b1), Dominance::Before);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span<'a> {
    lo: Cow<'a, Version>,
    hi: Cow<'a, Version>,
}

impl<'a> Span<'a> {
    /// The validating door: the span `[lo, hi]`, checking that the
    /// pair is ordered.
    ///
    /// # Errors
    ///
    /// [`Crossed`] unless `lo <= hi` — a reversed pair is rejected, and
    /// so is an incomparable one, where neither version bounds the
    /// other and no chain segment exists between them.
    pub fn new(lo: &'a Version, hi: &'a Version) -> Result<Self, Crossed> {
        match lo.partial_cmp(hi) {
            Some(Ordering::Less | Ordering::Equal) => Ok(Self {
                lo: Cow::Borrowed(lo),
                hi: Cow::Borrowed(hi),
            }),
            Some(Ordering::Greater) | None => Err(Crossed),
        }
    }

    /// The trusted door: the span `[lo, hi]` from a caller who
    /// already holds `lo <= hi`, skipping [`new`](Self::new)'s
    /// validating comparison.
    ///
    /// For callers whose pairs are ordered *structurally* — a floor and
    /// ceiling maintained as meet and join of one underlying set, say —
    /// where re-validating per construction would pay a causal
    /// comparison for a fact already invariant. Everyone else should
    /// use [`new`](Self::new).
    ///
    /// The caller **must** guarantee `lo <= hi`. On a pair that
    /// violates it, every verdict of [`place`](Self::place) and
    /// [`dominance_of`](Self::dominance_of) is unspecified and
    /// meaningless.
    ///
    /// # Panics
    ///
    /// Debug builds assert `lo <= hi` and panic when it fails; release
    /// builds construct the span unchecked.
    pub fn ordered(lo: &'a Version, hi: &'a Version) -> Self {
        debug_assert!(
            lo <= hi,
            "Span::ordered requires lo <= hi: the caller's structural guarantee failed"
        );
        Self {
            lo: Cow::Borrowed(lo),
            hi: Cow::Borrowed(hi),
        }
    }

    /// The crate-internal owned door: a span from endpoints the caller
    /// derived as one collection's meet and join.
    ///
    /// [`Version::span`] and [`Version::span_all`] construct through
    /// here — their endpoints are minted owned, so the span borrows
    /// nothing. The caller's derivation must guarantee `lo <= hi`
    /// (a meet/join pair over one nonempty collection always does).
    pub(crate) fn owned(lo: Version, hi: Version) -> Span<'static> {
        debug_assert!(
            lo <= hi,
            "Span::owned requires lo <= hi: the endpoints must be one collection's meet and join"
        );
        Span {
            lo: Cow::Owned(lo),
            hi: Cow::Owned(hi),
        }
    }

    /// Places `probe` against this span at full resolution: the
    /// nine-way [`Placement`] verdict, the finest partition of the
    /// question the partial order admits.
    ///
    /// The name deliberately echoes
    /// [`placement_of`](Range::placement_of) — the same question,
    /// asked of a different object. The verdict is a pure transcription
    /// of the two causal comparisons against the endpoints; the
    /// `span_place_matches_relations` law in [`laws`](crate::laws)
    /// pins it on every law consumer, and
    /// `degenerate_span_place_is_partial_cmp` pins the coincident
    /// span `[v, v]` to pairwise comparison itself.
    ///
    /// One fused comparison pass answers both relations: the probe and
    /// the endpoint streams are walked simultaneously, each decoded
    /// once — against two probe decodes when the comparisons are
    /// composed. An endpoint whose relation is decided (a detected
    /// concurrency) stops being scanned, and once both endpoints have
    /// refuted the verdict returns at the deciding interval; no earlier
    /// exit exists at full resolution, because distinguishing
    /// `Concurrent(Start)` from `Concurrent(Both)` needs the other
    /// endpoint's relation to complete. When only the dominance
    /// question is asked, [`dominance_of`](Self::dominance_of) bails
    /// earlier.
    pub fn place(&self, probe: &Version) -> Placement {
        place::span(probe.view(), self.lo.view(), self.hi.view())
    }

    /// How much of this span `probe` dominates: the three-way
    /// [`Dominance`] verdict, [`place`](Self::place) coarsened to the
    /// dominance question.
    ///
    /// The coarsening table, pinned by the
    /// `span_dominance_coarsens_place` law in
    /// [`laws`](crate::laws):
    ///
    /// - [`After`](Dominance::After) ⟸ `At(End)`, `At(Both)`, `After`.
    /// - [`Between`](Dominance::Between) ⟸ `At(Start)`, `Between`,
    ///   `Concurrent(End)`.
    /// - [`Before`](Dominance::Before) ⟸ `Before`,
    ///   `Concurrent(Start)`, `Concurrent(Both)`.
    ///
    /// The coarser question buys the placement family's earliest exit:
    /// the verdict reads only the endpoint-at-or-below-probe
    /// directions, so the moment `lo <= probe` is refuted the answer is
    /// [`Before`](Dominance::Before) regardless of `hi` and the fused
    /// walk returns at the refuting interval — where
    /// [`place`](Self::place) must sweep on — and the moment
    /// `hi <= probe` is refuted the `hi` stream stops being scanned
    /// while `lo` decides [`Between`](Dominance::Between) against
    /// [`Before`](Dominance::Before). On sweeps with no refutation the
    /// cost is [`place`](Self::place)'s exactly.
    pub fn dominance_of(&self, probe: &Version) -> Dominance {
        place::dominance(probe.view(), self.lo.view(), self.hi.view())
    }
}

/// A [`Span`] endpoint, as a verdict payload: *which* endpoint an
/// at- or beside-the-span verdict speaks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    /// The span's lower endpoint, `lo`.
    Start,
    /// The span's upper endpoint, `hi`.
    End,
    /// Both endpoints at once. What that means is the carrying
    /// verdict's: see [`Placement::At`] and [`Placement::Concurrent`].
    Both,
}

/// Where a version sits relative to a [`Span`]: the placement
/// family's finest verdict.
///
/// In a partial order, a point sits in exactly one of nine regions
/// relative to a span — below, within, above, at either or both
/// endpoints, or beside it in one of three ways distinguished by which
/// endpoint still bounds it. The enum's shape is the proof of that
/// count: three bare variants for the regions on the chain through the
/// span, and two endpoint-qualified variants times three
/// [`Endpoint`] payloads for the rest — `3 + 2×3 = 9` — while the seven
/// combinations `lo <= hi` forbids (below `lo` yet not strictly below
/// `hi`; equal to `lo` yet above or beside `hi`; concurrent to `lo` yet
/// at or above `hi`) have no spelling at all.
/// Each variant's doc states the raw relations it reports and the
/// relations its payload forces.
///
/// **Vocabulary kinship, divergent semantics**: `Before`, `Between`,
/// and `After` deliberately echo [`Bounded`]'s words — the same
/// question, asked of a different object. `Bounded`'s region verdicts
/// fold range semantics (a version concurrent to the start bound is
/// `Bounded::Between`, because start bounds keep concurrent versions);
/// `Placement`'s variants are raw strict-order facts against two
/// concrete versions (a version concurrent to `lo` is
/// `Concurrent(Start)`, never `Between`). [`Dominance`] reuses the
/// words a third way, coarser than both: each of its verdicts is a
/// bucket of these nine regions (its variant docs carry the exact
/// tables). On a two-bounded range,
/// [`bounded`](Range::bounded) is exactly a coarsening of this verdict,
/// pinned by the `bounded_coarsens_span_place` law in
/// [`laws`](crate::laws).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placement {
    /// Strictly below the whole span: `p < lo`, hence `p < hi`.
    Before,
    /// Exactly at an endpoint:
    ///
    /// - `At(Start)`: `p == lo < hi`.
    /// - `At(End)`: `lo < p == hi`.
    /// - `At(Both)`: `p == lo == hi`. Equality to one endpoint of a
    ///   coincident span is equality to both, so on `lo == hi`
    ///   every at-endpoint verdict is `At(Both)` — the coincident
    ///   span is the common single-version case, not a corner.
    At(Endpoint),
    /// Strictly inside: `lo < p < hi`.
    Between,
    /// Beside the span: incomparable to the endpoint(s) the payload
    /// names, with the opposite relation forced by `lo <= hi`:
    ///
    /// - `Concurrent(Start)`: `p ∥ lo`, forcing `p < hi` (at or above
    ///   `hi` would put `p` above `lo`).
    /// - `Concurrent(End)`: `p ∥ hi`, forcing `p > lo` (at or below
    ///   `lo` would put `p` below `hi`).
    /// - `Concurrent(Both)`: `p ∥ lo` and `p ∥ hi`.
    Concurrent(Endpoint),
    /// Strictly above the whole span: `p > hi`, hence `p > lo`.
    After,
}

/// How much of a [`Span`] a probe dominates:
/// [`Placement`] coarsened to the dominance question, "is the probe
/// causally at or after the span's content?"
///
/// The three verdicts a filter over version-bounded regions consumes: a
/// probe dominating the whole span has seen everything the span
/// covers; one dominating only the start has seen some of it; one
/// dominating not even the start has seen none of it.
///
/// **Vocabulary kinship, divergent semantics**: `After`, `Between`,
/// and `Before` echo [`Placement`]'s and [`Bounded`]'s words at a
/// third, coarser resolution, and the names understate what each
/// bucket folds in: the variants are position-named by the probe's
/// relation to the span's *endpoints as dominance thresholds*, and
/// each deliberately buckets concurrent and at-endpoint placements
/// with the regions (a probe concurrent to the start is
/// `Dominance::Before`, where `Placement` says `Concurrent(Start)`
/// and a range's `bounded` would keep it `Between`). Each variant's
/// doc states its exact [`Placement`] bucket; the
/// `span_dominance_coarsens_place` law in [`laws`](crate::laws) pins
/// the tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dominance {
    /// The probe dominates the whole span: `hi <= p`, and with it
    /// every version the span covers.
    ///
    /// The bucket of [`Placement`]s with `hi <= p` — `At(End)`,
    /// `At(Both)`, and `After`: at the end counts as dominance, so
    /// this is "at or beyond the whole span", not only strictly
    /// after it.
    After,
    /// The probe dominates the start but not the whole: `lo <= p`,
    /// while `hi` is above or beside the probe.
    ///
    /// The bucket of [`Placement`]s with `lo <= p` but not `hi <= p`
    /// — `At(Start)`, `Between`, and `Concurrent(End)`: at the start
    /// counts as dominance, and a probe concurrent to the end still
    /// dominates the start, so "between" here means between the
    /// endpoints *as dominance thresholds*, incomparability to the
    /// end included.
    Between,
    /// The probe does not dominate even the start: `lo` is above or
    /// beside the probe — and with it `hi`.
    ///
    /// The bucket of [`Placement`]s without `lo <= p` — `Before`,
    /// `Concurrent(Start)`, and `Concurrent(Both)`: a probe
    /// *concurrent* to the start dominates none of the span, so it
    /// lands here beside the strictly-below region, not in
    /// [`Between`](Self::Between).
    Before,
}

impl RangeBounds<Version> for Range<'_> {
    fn start_bound(&self) -> Bound<&Version> {
        self.start
    }

    fn end_bound(&self) -> Bound<&Version> {
        self.end
    }
}

#[cfg(test)]
mod tests;
