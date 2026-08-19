//! Causal spans: ordered pairs of concrete upper-/lower-bounding [`Version`]s.
//! The contract documentation, the operation table, and the algebra live on
//! [`Span`].

use std::borrow::Cow;
use std::cmp::Ordering;

use crate::codec;
use crate::error::Crossed;
use crate::version::skyline::place;
use crate::{Party, Version};

mod algebra;
mod own;
mod verdict;
mod wire;

pub use own::OwnSpan;
pub use verdict::{Dominance, Endpoint, Placement, Precedence};

#[cfg(test)]
mod tests;

/// A causal span: an ordered pair of [`Version`]s `lo <= hi`.
///
/// A [`Span`] can be used to efficiently answer where a [`Version`] falls
/// relative to its causal interval: [`place`](Span::place) at the finest
/// granularity, with cheaper coarsenings of its verdict computed by
/// [`precedence`](Span::precedence), [`dominance`](Span::dominance), and
/// [`contains`](Span::contains).
///
/// | Operation                                             | Meaning                                                             |
/// |-------------------------------------------------------|---------------------------------------------------------------------|
/// | `v ^ w`, [`v.span(&w)`](Version::span)                | the tightest span containing `v` and `w`                            |
/// | [`Version::span_all`]                                 | …containing a whole collection                                     |
/// | [`Span::new`]`(lo, hi)`                               | the span `lo <= hi`; errors unless `lo <= hi`                       |
/// | [`Span::at`]`(v)`                                     | the singleton span `v <= v`                                         |
/// | [`s.place(&v)`](Span::place)                          | `v` against the bounds, finest granularity ([`Placement`])          |
/// | [`s.dominance(&v)`](Span::dominance)                  | three-way verdict: is `v` past the span? ([`Dominance`])            |
/// | [`s.precedence(&v)`](Span::precedence)                | three-way verdict: is `v` before it? ([`Precedence`])               |
/// | [`s.contains(&v)`](Span::contains)                    | membership of `v`; a span argument asks whole-span containment      |
/// | `a \| b`, `a & b`                                     | the *pointwise* lattice: `\|`/`&` on each endpoint pair             |
/// | `a + b`                                               | the *union*: the tightest span covering both                        |
/// | `a * b`                                               | the *intersection*: the largest common part; `None` if disjoint     |
/// | `a \|= b`, `a &= b`, `a += b`                         | the total operators' assigning forms (`*` is partial: no `*=`)      |
/// | `s + v`, `v + s`, `s \| v`, `v & s`, …               | a `Version` operand is its point span (`*` takes true spans only)   |
/// | `&s / &p`, [`s.project(&p)`](Span::project)           | the lazy projection view ([`OwnSpan`])                              |
/// | [`encode`](Span::encode) / [`decode`](Span::decode)   | the canonical wire form                                             |
///
/// # The span algebra
///
/// [`Span`] participates in two distinct lattice structures:
///
/// - The **pointwise** lattice borrows the version lattice's own symbols and
///   lifts them to each endpoint of the [`Span`]:
///   - `a | b` ([`join`](Span::join)) has endpoints `lo_a | lo_b <= hi_a | hi_b`;
///   - `a & b` ([`meet`](Span::meet)) has endpoints `lo_a & lo_b <= hi_a & hi_b`.
///
/// - The **containment** lattice uses arithmetic symbols to treat [`Span`]s as sets of
///   [`Version`]s under union and intersection:
///   - `a + b` ([`union`](Span::union)) has endpoints `lo_a & lo_b <= hi_a | hi_b`;
///   - `a * b` ([`intersect`](Span::intersect)) has endpoints `lo_a | lo_b <= hi_a & hi_b`,
///     or [`None`] when the spans are non-overlapping.
///
/// All operators have a variadic extension ([`join_all`](Span::join_all),
/// [`meet_all`](Span::meet_all), [`union_all`](Span::union_all),
/// [`intersect_all`](Span::intersect_all)), each one a balanced fold.
///
/// Projection applies [`Version::project`] pointwise to the low and high ends
/// of the span: for a given [`Span`] `s`, `s / &p` yields the span `(lo / &p)
/// <= (hi / &p)`.
///
/// # The wire form
///
/// A [`Span`] has a canonical
/// [`encode`](Span::encode)/[`decode`](Span::decode): the concatenation of its
/// `lo` [`Version`] followed by its `hi` [`Version`] with no additional
/// delimitation.
///
/// # Example
///
/// ```
/// use before::{Clock, Dominance, Endpoint, Span, Placement};
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
    /// Constructs the span `lo <= hi`, checking that the pair is ordered.
    ///
    /// Each endpoint is anything [`Into`] a [`Cow`] of [`Version`], which
    /// permits borrowed or owned arguments to be passed as desired.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_cmp.html"))]
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

    /// The coincident [`Span`] `version <= version`: the span at one point.
    ///
    /// That single point may be anything [`Into`] a [`Cow`] of [`Version`],
    /// which permits a borrowed or owned argument to be passed as desired.
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

    /// Reborrows this [`Span`]'s endpoints: the same `lo <= hi` span with a
    /// fresh, shorter lifetime.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
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
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_place.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Endpoint, Placement, Span};
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
        // constructors or the wire decode stores one buffer twice, so the
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
        place::span(
            version.view().live(),
            self.lo.view().live(),
            self.hi.view().live(),
        )
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
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_dominance.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Dominance, Span};
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
        place::dominance(
            version.view().live(),
            self.lo.view().live(),
            self.hi.view().live(),
        )
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
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_precedence.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Precedence, Span};
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
        place::precedence(
            version.view().live(),
            self.lo.view().live(),
            self.hi.view().live(),
        )
    }

    /// Whether this [`Span`] contains `other`, in the containment order:
    /// membership `lo <= v <= hi` for a [`Version`], and `lo <= other.lo()
    /// && other.hi() <= hi` for a whole [`Span`].
    ///
    /// The argument is anything [`Into`] a [`Span`]: a borrowed or owned
    /// [`Version`], or a borrowed or owned [`Span`].
    ///
    /// # Complexity
    ///
    /// A [`Version`] requires only a single fused walk across `lo`, `v`, and
    /// `hi` together:
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_contains.html"))]
    ///
    /// A [`Span`] requires two causal comparisons: one to compare the two `lo`
    /// endpoints and a second to compare the two `hi` endpoint, where seach
    /// comparison costs:
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_cmp.html"))]
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
    /// let span = Span::new(&a1, &a3).unwrap();
    /// // Both endpoints are within…
    /// assert!(span.contains(&a1) && span.contains(&a3));
    /// // …while a version above the span, or concurrent to an
    /// // endpoint, is not.
    /// assert!(!span.contains(alice.tick().clone()));
    /// assert!(!span.contains(&b1));
    ///
    /// // A span is contained iff both its endpoints are…
    /// assert!(span.contains(&a1.span(&a2)));
    /// assert!(span.contains(span.reborrow()));
    /// // …and one reaching outside is not.
    /// assert!(!a1.span(&a2).contains(&span));
    /// assert!(!span.contains(&a2.span(&b1)));
    /// ```
    pub fn contains<'b>(&self, other: impl Into<Span<'b>>) -> bool {
        let other = other.into();
        // A coincident argument is a membership probe: `lo <= v && v <= hi`
        // in one fused walk over the three streams, each decoded once, where
        // the endpoint comparisons below would decode the probe twice. Clone
        // identity certifies the argument's coincidence in `O(1)` (the
        // version conversions store one buffer twice), and coincident endpoints in
        // distinct buffers still answer correctly through the general arm.
        if other.is_coincident() {
            let version = other.lo();
            // The coincident receiver collapses membership further, to
            // equality: on `lo == hi` the segment is one version, and
            // equality of canonical streams is byte equality — one compare,
            // no walk.
            if self.lo.view().ptr_eq(self.hi.view()) {
                return codec::canonical_eq(version.view(), self.lo().view());
            }
            return place::contains(
                version.view().live(),
                self.lo.view().live(),
                self.hi.view().live(),
            );
        }
        // A span is contained iff both its endpoints are: every version
        // between them lies within `self` by transitivity of the bounds.
        matches!(
            self.lo().partial_cmp(other.lo()),
            Some(Ordering::Less | Ordering::Equal)
        ) && matches!(
            other.hi().partial_cmp(self.hi()),
            Some(Ordering::Less | Ordering::Equal)
        )
    }

    /// The part of this [`Span`] wholly owned by a [`Party`], as a lazy
    /// [`OwnSpan`] view.
    ///
    /// Identical to the operator form `&self / party`.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let span = a1.span(&a2);
    /// // The named and operator spellings build the same view...
    /// let view = span.project(alice.party());
    /// assert_eq!(view.to_span(), (&span / alice.party()).to_span());
    /// // ...and the seed owns everything: the view places like the span.
    /// assert_eq!(view.place(&a1), span.place(&a1));
    /// ```
    pub fn project(&'a self, party: &'a Party) -> OwnSpan<'a> {
        self / party
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
    /// The hull constructors, the wire decode, and the algebra's point combines all
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
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
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

/// The borrowing view of a span, identical to [`Span::reborrow`].
///
/// This is the conversion that lets `impl Into<Span>` arguments (e.g.
/// [`contains`](Span::contains), [`Query::coverage`](crate::causally::Query::coverage))
/// accept `&Span` beside owned spans and versions.
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
/// let stored = Span::at(v);
/// let view = Span::from(&stored); // borrows; `stored` stays usable
/// assert_eq!(view, stored);
/// ```
impl<'a> From<&'a Span<'_>> for Span<'a> {
    fn from(span: &'a Span<'_>) -> Span<'a> {
        span.reborrow()
    }
}
