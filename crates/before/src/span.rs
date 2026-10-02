//! Causal intervals bounded by an ordered pair of [`Version`]s.

use std::borrow::Cow;
use std::cmp::Ordering;

use crate::error::Crossed;
use crate::version::place;
use crate::{Party, Version};

mod algebra;
mod own;
mod verdict;
mod wire;

pub use own::OwnSpan;
pub use verdict::{Dominance, Endpoint, Placement, Precedence};

#[cfg(test)]
mod tests;

/// An inclusive causal interval with ordered endpoints `lo <= hi`.
///
/// [`Span::new`] checks this ordering; [`Span::at`] creates the point span
/// `v <= v`. [`Span::place`] gives the complete relation between a [`Version`]
/// and the endpoints. [`Span::contains`],
/// [`dominance`](Span::dominance), and [`precedence`](Span::precedence) answer
/// coarser questions without computing more detail than they return.
///
/// | Operation                                           | Meaning                                                         |
/// |-----------------------------------------------------|-----------------------------------------------------------------|
/// | `v ^ w`, [`v.span(&w)`](Version::span)              | the tightest span containing `v` and `w`                        |
/// | [`Version::span_all`]                               | the tightest span containing a collection                       |
/// | [`Span::new(lo, hi)`](Span::new)                    | construct `lo <= hi`, rejecting reversed or concurrent endpoints|
/// | [`Span::at(v)`](Span::at)                           | the point span `v <= v`                                         |
/// | [`s.place(&v)`](Span::place)                        | the complete relation between `v` and both endpoints            |
/// | [`s.contains(&v)`](Span::contains)                  | whether `v` lies within the span                                |
/// | `a \| b`, `a & b`                                   | pointwise join or meet of matching endpoints                    |
/// | `a + b`, `a * b`                                    | containment union or optional intersection                      |
/// | [`s.project(&p)`](Span::project), `&s / &p`         | a lazy view of both endpoints projected onto [`Party`] `p`      |
/// | [`s.encode()`](Span::encode)/[`Span::decode`]       | encode or decode the canonical wire form                        |
///
/// # The span algebra
///
/// Spans support two related algebras:
///
/// - The **pointwise** operations apply the [`Version`] lattice to matching
///   endpoints:
///   - `a | b` ([`join`](Span::join)) has endpoints `lo_a | lo_b <= hi_a | hi_b`;
///   - `a & b` ([`meet`](Span::meet)) has endpoints `lo_a & lo_b <= hi_a & hi_b`.
///
/// - The **containment** operations treat spans as sets of versions:
///   - `a + b` ([`union`](Span::union)) has endpoints `lo_a & lo_b <= hi_a | hi_b`;
///   - `a * b` ([`intersect`](Span::intersect)) has endpoints `lo_a | lo_b <= hi_a & hi_b`,
///     or [`None`] when the spans are non-overlapping.
///
/// The assigning operators exist for the three total operations. Intersection
/// is partial and therefore has no `*=` form. Each operation also has an
/// `_all` form for a collection.
///
/// [`Span::project`] applies [`Version::project`] to both endpoints.
/// [`Span::encode`] concatenates the two self-delimiting endpoint encodings;
/// [`Span::decode`] validates both endpoints and their
/// ordering.
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct Span<'a> {
    lo: Cow<'a, Version>,
    hi: Cow<'a, Version>,
}

impl<'a> Span<'a> {
    /// Constructs the span `lo <= hi`, checking that the pair is ordered.
    ///
    /// Endpoints may be borrowed or owned.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_cmp.html")))]
    #[cfg_attr(
        not(doc),
        doc = "comparison: `O(n)` in total input bytes; `O(|a| + |b|)`"
    )]
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

    /// The point [`Span`] `version <= version`.
    ///
    /// The version may be borrowed or owned. Classification methods recognize
    /// the result as a point directly, without first comparing its endpoints.
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
        // Cloning a Cow either repeats the borrow or shares Version storage.
        // The endpoints can therefore be recognized as equal in O(1).
        let hi = lo.clone();
        Span { lo, hi }
    }

    /// Construct a span from endpoints already known to be ordered.
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

    /// Compares `version` against this [`Span`] at full resolution, returning a
    /// [`Placement`] verdict.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_place.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |version|)`")]
    ///
    /// A point span produced by [`Span::at`] requires one causal comparison
    /// with its endpoint.
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
    /// // A concurrent version is not within the span.
    /// assert_eq!(span.place(&b1), Placement::Concurrent(Endpoint::Both));
    /// ```
    pub fn place(&self, version: &Version) -> Placement {
        // Shared storage proves that the endpoints are equal without reading
        // them, so one comparison determines placement. Equal endpoints in
        // separate allocations take the general path below.
        if self.lo.ptr_eq(&self.hi) {
            return match version.partial_cmp(self.lo()) {
                Some(Ordering::Less) => Placement::Before,
                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
                Some(Ordering::Greater) => Placement::After,
                None => Placement::Concurrent(Endpoint::Both),
            };
        }
        place::span(version, &self.lo, &self.hi)
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_dominance.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |version|)`")]
    ///
    /// A point span produced by [`Span::at`] requires one causal comparison
    /// with its endpoint.
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
        // For the point span [v, v], `Between` is impossible: the answer is
        // `After` exactly when v <= version, and `Before` otherwise. Shared
        // storage identifies this case without comparing the endpoints.
        if self.lo.ptr_eq(&self.hi) {
            // `hi <= version` asks whether the point lies in the version's
            // causal past.
            return if matches!(
                self.hi().partial_cmp(version),
                Some(Ordering::Less | Ordering::Equal)
            ) {
                Dominance::After
            } else {
                Dominance::Before
            };
        }
        place::dominance(version, &self.lo, &self.hi)
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_precedence.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |version|)`")]
    ///
    /// A point span produced by [`Span::at`] requires one causal comparison
    /// with its endpoint.
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
        // For the point span [v, v], `Between` is impossible: the answer is
        // `Before` exactly when version <= v, and `After` otherwise. Shared
        // storage identifies this case without comparing the endpoints.
        if self.lo.ptr_eq(&self.hi) {
            // `version <= lo` asks whether the point lies in the version's
            // causal future.
            return if matches!(
                version.partial_cmp(self.lo()),
                Some(Ordering::Less | Ordering::Equal)
            ) {
                Precedence::Before
            } else {
                Precedence::After
            };
        }
        place::precedence(version, &self.lo, &self.hi)
    }

    /// Whether this [`Span`] contains `other`, in the containment order:
    /// membership `lo <= v <= hi` for a [`Version`], and `lo <= other.lo()
    /// && other.hi() <= hi` for a whole [`Span`].
    ///
    /// The argument may be a borrowed or owned [`Version`] or [`Span`].
    ///
    /// # Complexity
    ///
    /// A [`Version`] is classified in one linear pass over the two endpoints
    /// and the version:
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_contains.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |version|)`")]
    ///
    /// Testing a [`Version`] against a point span produced by [`Span::at`]
    /// compares it once with the endpoint.
    ///
    /// A [`Span`] requires two causal comparisons: one for its lower endpoints
    /// and one for its upper endpoints. Each comparison costs:
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_cmp.html")))]
    #[cfg_attr(
        not(doc),
        doc = "comparison: `O(n)` in total input bytes; `O(|a| + |b|)`"
    )]
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
        // A point span asks whether `lo <= v <= hi`. Recognizing shared
        // endpoints avoids reading v once for each endpoint comparison.
        if let Some(version) = other.shared_endpoint() {
            // One point span contains another exactly when their versions are
            // equal.
            if let Some(endpoint) = self.shared_endpoint() {
                return version == endpoint;
            }
            return place::contains(version, &self.lo, &self.hi);
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

    /// The endpoint when both bounds share the same version storage.
    ///
    /// This recognizes point spans without comparing their versions. Equal
    /// endpoints in separate allocations return `None` and take the general
    /// path.
    fn shared_endpoint(&self) -> Option<&Version> {
        self.lo.ptr_eq(&self.hi).then(|| self.lo())
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

    /// Returns an equivalent span with owned endpoints.
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

/// Borrows a version as a [`Cow`].
///
/// # Complexity
///
/// `O(1)`.
impl<'a> From<&'a Version> for Cow<'a, Version> {
    fn from(version: &'a Version) -> Cow<'a, Version> {
        Cow::Borrowed(version)
    }
}

/// Moves a version into a [`Cow`].
///
/// # Complexity
///
/// `O(1)`.
impl From<Version> for Cow<'_, Version> {
    fn from(version: Version) -> Self {
        Cow::Owned(version)
    }
}

/// The point span `[version, version]`, identical to [`Span::at`].
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

/// The point span at a borrowed version, identical to [`Span::at`].
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

/// A borrowed view of a span, identical to [`Span::reborrow`].
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
