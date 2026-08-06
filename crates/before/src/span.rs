//! Causal spans: ordered pairs of concrete upper-/lower-bounding [`Version`]s.
//!
//! A *span* is two concrete versions `lo <= hi` representing all the
//! [`Version`]s `lo <= v <= hi`.
//!
//! [`Span::place`] resolves the relationship of some [`Version`] `v` to the
//! [`Span`]'s bounds at the finest degree of granularity, while the coarsenings
//! answer cheaper questions: [`Span::dominance`] and [`Span::precedence`]
//! render three-way [`Dominance`] and [`Precedence`] verdicts, while
//! [`Span::contains`] renders bare membership of a [`Version`] within the
//! [`Span`].
//!
//! Every nonempty collection of versions has a tightest containing span, which
//! may be computed by [`Version::span`] and [`Version::span_all`].
//!
//! # The span algebra
//!
//! Spans can be manipulated according to two distinct lattice structures:
//!
//! - **The containment order** (set-like symbols): `a | b` is the
//!   *union* — the tightest span covering both `a` and `b`, with endpoints
//!   `[lo_a & lo_b, hi_a | hi_b]` — and   `a & b` is the *intersection*
//!   — the largest span which is fully covered by both, with endpoints
//!   `[lo_a | lo_b, hi_a & hi_b]`, returning [`None`] when the
//!   spans are non-overlapping.
//! - **The pointwise order** (arithmetic symbols): `a + b` and
//!   `a * b` lift the version lattice itself to spans, pointwise:
//!   `a + b` yields a span with endpoints [lo_a | lo_b, hi_a | hi_b],
//!   while `a * b` yield a span with endpoints [lo_a & lo_b, hi_a & hi_b].
//!
//! The containment operators also have method spellings, [`Span::union`] and
//! [`Span::intersection`].
//!
//! Each operator has a variadic extension in the idiom of
//! [`Version::span_all`]: [`Span::union_all`], [`Span::intersect_all`],
//! [`Span::sum_all`], [`Span::product_all`] are each implemented as one
//! balanced fold, which is much more efficient than folding the operator
//! linearly across a list of inputs.
//!
//! Like [`Version`]s, [`Span`]s support the `/` projection operator: `&span /
//! &party` is [`OwnSpan`], a lazy view equivalent to the span with endpoints
//! `[lo / &p, hi / &p]`.
//!
//! # The wire form
//!
//! A [`Span`] has a canonical byte encoding, just like [`Clock`](crate::Clock),
//! [`Version`], and [`Party`](crate::Party): the meet's [`Version::encode`]
//! bytes, followed by the join's.
//! Each component is byte-aligned, independently canonical, and
//! self-delimiting, so the two concatenate with no length prefix.

use std::borrow::Cow;
use std::cmp::Ordering;

use crate::codec;
use crate::error::Crossed;
use crate::version::skyline::place;
use crate::Version;

mod algebra;
mod own;
mod verdict;
mod wire;

pub use own::OwnSpan;
pub use verdict::{Dominance, Endpoint, Placement, Precedence};

#[cfg(test)]
mod tests;

/// A causal span: an ordered pair of versions `lo <= hi` representing all the
/// [`Version`]s `lo <= v <= hi`.
///
/// # Example
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
/// assert_eq!(span.dominance(&a3), Dominance::After);
/// assert_eq!(span.dominance(&a2), Dominance::Between);
/// assert_eq!(span.dominance(&b1), Dominance::Before);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span<'a> {
    lo: Cow<'a, Version>,
    hi: Cow<'a, Version>,
}

impl<'a> Span<'a> {
    /// Constructs the span `[lo, hi]`, checking that the pair is ordered.
    ///
    /// Each endpoint is anything [`Into`] a [`Cow`] of [`Version`], which
    /// permits borrowed or owned arguments to be passed as desired.
    ///
    /// # Complexity
    ///
    /// `O(|lo| + |hi|)`, to validate that `lo <= hi`.
    ///
    /// # Errors
    ///
    /// Returns [`Crossed`] unless `lo <= hi`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    ///
    /// let span = Span::new(&a1, &a2).unwrap();
    /// assert_eq!((span.lo(), span.hi()), (&a1, &a2));
    /// // A reversed or incomparable pair is not a span.
    /// assert!(Span::new(&a2, &a1).is_err());
    /// assert!(Span::new(&a1, &b1).is_err());
    /// ```
    pub fn new(
        lo: impl Into<Cow<'a, Version>>,
        hi: impl Into<Cow<'a, Version>>,
    ) -> Result<Self, Crossed> {
        let (lo, hi) = (lo.into(), hi.into());
        match lo.as_ref().partial_cmp(hi.as_ref()) {
            Some(Ordering::Less | Ordering::Equal) => Ok(Self { lo, hi }),
            Some(Ordering::Greater) | None => Err(Crossed),
        }
    }

    /// The coincident [`Span`] `[version, version]`: the span at one point.
    ///
    /// The point is anything [`Into`] a [`Cow`] of [`Version`], which permits a
    /// borrowed or owned argument to be passed as desired.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let v = alice.tick().clone();
    /// let point = Span::at(v.clone()); // owned; `Span::at(&v)` lends
    /// // Both endpoints are the version, and the span is exactly
    /// // the singleton hull.
    /// assert_eq!((point.lo(), point.hi()), (&v, &v));
    /// assert_eq!(point, v.span(&v));
    /// assert_eq!(Span::from(v.clone()), point);
    /// assert_eq!(Span::from(&v), point);
    /// ```
    pub fn at(version: impl Into<Cow<'a, Version>>) -> Span<'a> {
        let lo = version.into();
        // A borrowed endpoint is lent twice; an owned one moves in
        // and its buffer-sharing clone fills the second slot — either
        // way the pair reads one shared buffer, the O(1) coincidence
        // certificate every fast path reads.
        let hi = lo.clone();
        Span { lo, hi }
    }

    /// Internal-only: A span from endpoints the caller derived as one
    /// collection's meet and join.
    pub(crate) fn owned(lo: Version, hi: Version) -> Span<'static> {
        Span {
            lo: Cow::Owned(lo),
            hi: Cow::Owned(hi),
        }
    }

    /// Reborrows this [`Span`]'s endpoints: the same `[lo, hi]` span with a
    /// fresh, shorter lifetime.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, causally::Span};
    ///
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    ///
    /// // A stored span lends a view of itself without being consumed.
    /// let stored: Span<'static> = a1.span(&a2); // owned endpoints
    /// let view: Span<'_> = stored.reborrow();
    /// assert_eq!(view, stored); // the same endpoints, byte for byte
    /// assert_eq!(view.dominance(&a2), stored.dominance(&a2));
    /// ```
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    pub fn reborrow(&self) -> Span<'_> {
        Span {
            lo: Cow::Borrowed(self.lo()),
            hi: Cow::Borrowed(self.hi()),
        }
    }

    /// Compares `version` against this [`Span`] at full resolution, rendering a
    /// nine-way [`Placement`] verdict.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |version|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, causally::{Endpoint, Placement, Span}};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    ///
    /// let span = Span::new(&a1, &a3).unwrap();
    /// assert_eq!(span.place(&a1), Placement::At(Endpoint::Start));
    /// assert_eq!(span.place(&a2), Placement::Between);
    /// // A concurrent version is beside the span, not within it.
    /// assert_eq!(span.place(&b1), Placement::Concurrent(Endpoint::Both));
    /// ```
    pub fn place(&self, version: &Version) -> Placement {
        // The coincident span collapses placement to pairwise
        // comparison — the `degenerate_span_place_is_partial_cmp` law
        // in [`laws`](crate::laws) — and clone identity certifies
        // `lo == hi` in `O(1)`: a coincident span built by the hull
        // doors or the wire decode stores one buffer twice, so the
        // fused three-stream walk would read that buffer twice where
        // one pair sweep answers. Coincident endpoints in distinct
        // buffers still take the fused walk below.
        if self.lo.view().ptr_eq(self.hi.view()) {
            return match version.partial_cmp(self.lo()) {
                Some(Ordering::Less) => Placement::Before,
                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
                Some(Ordering::Greater) => Placement::After,
                None => Placement::Concurrent(Endpoint::Both),
            };
        }
        place::span(version.view(), self.lo.view(), self.hi.view())
    }

    /// Determines how much of this [`Span`] `version` *dominates*, rendering a
    /// three-way [`Dominance`] verdict:
    ///
    /// - A [`Version`] is [`After`](Dominance::After) a [`Span`] if it is
    ///   greater than or equal to both endpoints of the span.
    /// - A [`Version`] is [`Between`](Dominance::Between) a [`Span`] if
    ///   it is greater than or equal to the lower bound of the [`Span`],
    ///   but strictly less than or concurrent to its upper bound.
    /// - A [`Version`] is [`Before`](Dominance::Before) a [`Span`] if it
    ///   is strictly less than or concurrent to both endpoints of the span.
    ///
    /// This is a coarsening of [`place`](Span::place)'s [`Placement`] verdict
    /// which can be computed more efficiently.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |version|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, causally::{Dominance, Span}};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// let span = Span::new(&a2, &a3).unwrap();
    /// // a3 dominates the whole span, a2 only its start, a1 not even that.
    /// assert_eq!(span.dominance(&a3), Dominance::After);
    /// assert_eq!(span.dominance(&a2), Dominance::Between);
    /// assert_eq!(span.dominance(&a1), Dominance::Before);
    /// ```
    pub fn dominance(&self, version: &Version) -> Dominance {
        // The coincident span collapses the dominance question to one
        // containment: on `lo == hi` the `After` bucket is exactly `hi <=
        // probe` and everything else is `Before` (`Between` needs the endpoints
        // to differ).
        //
        // Clone identity certifies the coincidence in `O(1)` so one
        // single-bound placement (each stream decoded once) answers where the
        // fused walk would read the shared buffer twice.
        //
        // This is the compressed-subtree classification fast path: a node whose
        // version bounds coincide is classified against one stream, not two.
        if self.lo.view().ptr_eq(self.hi.view()) {
            // `hi <= probe` is exactly membership in the probe's causal
            // past (`causally::before(probe).contains(hi)`).
            return if matches!(
                self.hi().partial_cmp(version),
                Some(Ordering::Less | Ordering::Equal)
            ) {
                Dominance::After
            } else {
                Dominance::Before
            };
        }
        place::dominance(version.view(), self.lo.view(), self.hi.view())
    }

    /// Determines how much of this [`Span`] `version` *precedes*, rendering a
    /// three-way [`Precedence`] verdict:
    ///
    /// - A [`Version`] is [`Before`](Precedence::Before) a [`Span`] if it is
    ///   less than or equal to both endpoints of the span.
    /// - A [`Version`] is [`Between`](Precedence::Between) a [`Span`] if
    ///   it is less than or equal to the upper bound of the [`Span`],
    ///   but strictly greater than or concurrent to its lower bound.
    /// - A [`Version`] is [`After`](Precedence::After) a [`Span`] if it
    ///   is strictly greater than or concurrent to both endpoints of the span.
    ///
    /// This is a coarsening of [`place`](Span::place)'s [`Placement`] verdict
    /// which can be computed more efficiently.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |version|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, causally::{Precedence, Span}};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// let span = Span::new(&a1, &a2).unwrap();
    /// // a1 precedes the whole span, a2 only its end, a3 not even that.
    /// assert_eq!(span.precedence(&a1), Precedence::Before);
    /// assert_eq!(span.precedence(&a2), Precedence::Between);
    /// assert_eq!(span.precedence(&a3), Precedence::After);
    /// ```
    pub fn precedence(&self, version: &Version) -> Precedence {
        // The coincident span collapses the precedence question to one
        // containment: on `lo == hi` the `Before` bucket is exactly `probe <=
        // lo` and everything else is `After` (`Between` needs the endpoints to
        // differ).
        //
        // Clone identity certifies the coincidence in `O(1)` so one
        // single-bound placement (each stream decoded once) answers where the
        // fused walk would read the shared buffer twice.
        //
        // This is the compressed-subtree classification fast path, mirrored: a
        // node whose version bounds coincide is classified against one stream,
        // not two.
        if self.lo.view().ptr_eq(self.hi.view()) {
            // `probe <= lo` is exactly membership in the probe's causal
            // future (`causally::after(probe).contains(lo)`).
            return if matches!(
                version.partial_cmp(self.lo()),
                Some(Ordering::Less | Ordering::Equal)
            ) {
                Precedence::Before
            } else {
                Precedence::After
            };
        }
        place::precedence(version.view(), self.lo.view(), self.hi.view())
    }

    /// Whether `version` lies within this [`Span`]: `lo <= version <= hi`.
    ///
    /// # Complexity
    ///
    /// `O(|self| + |version|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    ///
    /// let span = Span::new(&a1, &a2).unwrap();
    /// // Both endpoints are within…
    /// assert!(span.contains(&a1) && span.contains(&a2));
    /// // …while a version above the span, or concurrent to an
    /// // endpoint, is not.
    /// assert!(!span.contains(&a3));
    /// assert!(!span.contains(&b1));
    /// ```
    pub fn contains(&self, version: &Version) -> bool {
        // The coincident span collapses membership to equality: on `lo == hi`
        // the segment is one version, and equality of canonical streams is
        // byte equality — one compare, no walk.
        //
        // Clone identity certifies the coincidence in `O(1)`.
        if self.lo.view().ptr_eq(self.hi.view()) {
            return codec::canonical_eq(version.view(), self.lo().view());
        }
        place::contains(version.view(), self.lo.view(), self.hi.view())
    }

    /// The span's upper bound.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let span = Span::new(&a1, &a2).unwrap();
    /// assert_eq!(span.hi(), &a2);
    /// ```
    pub fn hi(&self) -> &Version {
        &self.hi
    }

    /// The span's lower bound.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let span = Span::new(&a1, &a2).unwrap();
    /// assert_eq!(span.lo(), &a1);
    /// ```
    pub fn lo(&self) -> &Version {
        &self.lo
    }

    /// Whether both endpoints read one shared stored buffer: the coincident
    /// span's `O(1)` certificate.
    ///
    /// The hull doors, the wire decode, and the algebra's point combines all
    /// store a coincident span's one stream twice (clones share the buffer), so
    /// clone identity certifies `lo == hi` without a walk. Coincident endpoints
    /// in distinct buffers are still equal — they just take the general walks.
    fn is_coincident(&self) -> bool {
        self.lo.view().ptr_eq(self.hi.view())
    }

    /// Destructures this span into its owned `(lo, hi)` endpoints.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let (lo, hi) = Span::new(&a1, &a2).unwrap().into_parts();
    /// assert_eq!((lo, hi), (a1, a2));
    /// ```
    pub fn into_parts(self) -> (Version, Version) {
        (self.lo.into_owned(), self.hi.into_owned())
    }

    /// Settles this span onto owned endpoints, erasing the borrow lifetime.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, causally::Span};
    ///
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let owned: Span<'static> = {
    ///     let borrowed = Span::new(&a1, &a2).unwrap();
    ///     borrowed.into_owned() // outlives the borrows
    /// };
    /// assert_eq!((owned.lo(), owned.hi()), (&a1, &a2));
    /// ```
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    pub fn into_owned(self) -> Span<'static> {
        Span {
            lo: Cow::Owned(self.lo.into_owned()),
            hi: Cow::Owned(self.hi.into_owned()),
        }
    }
}

/// Lends this version to a [`Cow`]-accepting callsite, providing automatic
/// reference lifting for methods on [`Span`]s which take [`Version`]s.
///
/// # Complexity
///
/// `O(1)`.
impl<'a> From<&'a Version> for Cow<'a, Version> {
    fn from(version: &'a Version) -> Cow<'a, Version> {
        Cow::Borrowed(version)
    }
}

/// Moves this version into a [`Cow`]-accepting callsite, dually to the lending
/// lift.
///
/// # Complexity
///
/// `O(1)`.
impl From<Version> for Cow<'_, Version> {
    fn from(version: Version) -> Self {
        Cow::Owned(version)
    }
}

/// The coincident span `[version, version]`, identical to [`Span::at`].
///
/// # Complexity
///
/// `O(1)`.
///
/// # Example
///
/// ```
/// use before::{Clock, Span};
/// let mut alice = Clock::seed();
/// let v = alice.tick().clone();
/// assert_eq!(Span::from(v.clone()), Span::at(&v));
/// ```
impl From<Version> for Span<'static> {
    fn from(version: Version) -> Span<'static> {
        Span::at(version)
    }
}

/// The coincident span at a borrowed version, identical to [`Span::at`].
///
/// # Complexity
///
/// `O(1)`.
///
/// # Example
///
/// ```
/// use before::{Clock, Span};
/// let mut alice = Clock::seed();
/// let v = alice.tick().clone();
/// let point = Span::from(&v); // borrows; `v` stays usable
/// assert_eq!(point, v.span(&v));
/// ```
impl<'a> From<&'a Version> for Span<'a> {
    fn from(version: &'a Version) -> Span<'a> {
        Span::at(version)
    }
}
