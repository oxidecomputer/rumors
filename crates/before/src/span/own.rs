//! The [`Span`]-equivalent of [`OwnVersion`].

use std::cmp::Ordering;
use std::ops::Div;

use crate::{
    span::{Dominance, Endpoint, Placement, Precedence},
    OwnVersion, Party, Span, Version,
};

/// The part of a [`Span`] contributed by a particular [`Party`], as a lazy
/// view.
///
/// # Example
///
/// ```
/// use before::{causally::Dominance, Clock};
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let b1 = bob.tick().clone();
/// let both = &a1 | &b1;
/// let span = a1.span(&both);
///
/// // Alice's view of the span drops bob's contribution: a1 already
/// // dominates everything alice owns of it — no projection is built.
/// assert_eq!((&span / alice.party()).dominance(&a1), Dominance::After);
/// // Against the unprojected span, a1 dominates only the start.
/// assert_eq!(span.dominance(&a1), Dominance::Between);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct OwnSpan<'a> {
    /// The party whose owned region gates both endpoints.
    party: &'a Party,
    /// The span being projected.
    span: &'a Span<'a>,
}

impl<'a> OwnSpan<'a> {
    /// The view's low endpoint, projected: `span.lo() / party`,
    /// as the lazy [`OwnVersion`] view.
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
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let both = &a1 | &bob.tick().clone();
    /// let span = a1.span(&both);
    /// let view = &span / alice.party();
    /// // a1 is alice's own: its projection is itself.
    /// assert_eq!(view.lo(), a1);
    /// ```
    pub fn lo(&self) -> OwnVersion<'a> {
        self.span.lo() / self.party
    }

    /// The view's high endpoint, projected: `span.hi() / party`,
    /// dually to [`lo`](Self::lo).
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
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let both = &a1 | &bob.tick().clone();
    /// let span = a1.span(&both);
    /// let view = &span / alice.party();
    /// // Alice's view of the join drops bob's tick.
    /// assert_eq!(view.hi(), a1);
    /// ```
    pub fn hi(&self) -> OwnVersion<'a> {
        self.span.hi() / self.party
    }

    /// Compares `version` against this [`OwnSpan`] at full resolution,
    /// rendering a nine-way [`Placement`] verdict: [`Span::place`], against the
    /// projected endpoints, without materializing the projection.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_span_place.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{causally::{Endpoint, Placement}, Clock};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let both = &a1 | &bob.tick().clone();
    /// let span = a1.span(&both);
    ///
    /// // Alice's view drops bob's contribution, collapsing the span onto
    /// // the point a1: placement lands at both endpoints at once…
    /// assert_eq!((&span / alice.party()).place(&a1), Placement::At(Endpoint::Both));
    /// // …while against the unprojected span, a1 is only the start.
    /// assert_eq!(span.place(&a1), Placement::At(Endpoint::Start));
    /// ```
    pub fn place(&self, version: &Version) -> Placement {
        let (lo, hi) = (self.lo(), self.hi());
        match version.partial_cmp(&lo) {
            Some(Ordering::Less) => Placement::Before,
            Some(Ordering::Equal) => match version.partial_cmp(&hi) {
                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
                _ => Placement::At(Endpoint::Start),
            },
            Some(Ordering::Greater) => match version.partial_cmp(&hi) {
                Some(Ordering::Less) => Placement::Between,
                Some(Ordering::Equal) => Placement::At(Endpoint::End),
                Some(Ordering::Greater) => Placement::After,
                None => Placement::Concurrent(Endpoint::End),
            },
            None => match version.partial_cmp(&hi) {
                None => Placement::Concurrent(Endpoint::Both),
                _ => Placement::Concurrent(Endpoint::Start),
            },
        }
    }

    /// Determines how much of this [`OwnSpan`] `version` *dominates*, rendering a
    /// three-way [`Dominance`] verdict:
    ///
    /// - A [`Version`] is [`After`](Dominance::After) an [`OwnSpan`] if it is
    ///   greater than or equal to both endpoints of the span.
    /// - A [`Version`] is [`Between`](Dominance::Between) an [`OwnSpan`] if
    ///   it is greater than or equal to the lower bound of the [`OwnSpan`],
    ///   but strictly less than or concurrent to its upper bound.
    /// - A [`Version`] is [`Before`](Dominance::Before) an [`OwnSpan`] if it
    ///   is strictly less than or concurrent to both endpoints of the span.
    ///
    /// This is a coarsening of [`place`](OwnSpan::place)'s [`Placement`] verdict
    /// which can be computed more efficiently.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_span_dominance.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{causally::Dominance, Clock};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let both = &a1 | &bob.tick().clone();
    /// let span = a1.span(&both);
    ///
    /// // Alice's view drops bob's contribution: a1 dominates all of it…
    /// assert_eq!((&span / alice.party()).dominance(&a1), Dominance::After);
    /// // …but only the start of the unprojected span.
    /// assert_eq!(span.dominance(&a1), Dominance::Between);
    /// ```
    pub fn dominance(&self, version: &Version) -> Dominance {
        if !matches!(
            version.partial_cmp(&self.lo()),
            Some(Ordering::Greater | Ordering::Equal)
        ) {
            return Dominance::Before;
        }
        if matches!(
            version.partial_cmp(&self.hi()),
            Some(Ordering::Greater | Ordering::Equal)
        ) {
            Dominance::After
        } else {
            Dominance::Between
        }
    }

    /// Determines how much of this [`OwnSpan`] `version` *precedes*, rendering a
    /// three-way [`Precedence`] verdict:
    ///
    /// - A [`Version`] is [`Before`](Precedence::Before) an [`OwnSpan`] if it is
    ///   less than or equal to both endpoints of the span.
    /// - A [`Version`] is [`Between`](Precedence::Between) an [`OwnSpan`] if
    ///   it is less than or equal to the upper bound of the [`OwnSpan`],
    ///   but strictly greater than or concurrent to its lower bound.
    /// - A [`Version`] is [`After`](Precedence::After) an [`OwnSpan`] if it
    ///   is strictly greater than or concurrent to both endpoints of the span.
    ///
    /// This is a coarsening of [`place`](OwnSpan::place)'s [`Placement`] verdict
    /// which can be computed more efficiently.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_span_precedence.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{causally::Precedence, Clock};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    /// let both = &a1 | &b1;
    /// let span = a1.span(&both);
    ///
    /// // b1 precedes the unprojected join, but alice's view drops it…
    /// assert_eq!(span.precedence(&b1), Precedence::Between);
    /// // …so nothing of the view lies at or after b1.
    /// assert_eq!((&span / alice.party()).precedence(&b1), Precedence::After);
    /// ```
    pub fn precedence(&self, version: &Version) -> Precedence {
        if !matches!(
            version.partial_cmp(&self.hi()),
            Some(Ordering::Less | Ordering::Equal)
        ) {
            return Precedence::After;
        }
        if matches!(
            version.partial_cmp(&self.lo()),
            Some(Ordering::Less | Ordering::Equal)
        ) {
            Precedence::Before
        } else {
            Precedence::Between
        }
    }

    /// Whether `version` lies within the [`OwnSpan`]: `lo <= version <= hi`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_span_contains.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let both = &a1 | &bob.tick().clone();
    /// let span = a1.span(&both);
    ///
    /// // The unprojected join lies beyond alice's view of the segment.
    /// assert!(span.contains(&both));
    /// assert!(!(&span / alice.party()).contains(&both));
    /// ```
    pub fn contains(&self, version: &Version) -> bool {
        matches!(
            version.partial_cmp(&self.lo()),
            Some(Ordering::Greater | Ordering::Equal)
        ) && matches!(
            version.partial_cmp(&self.hi()),
            Some(Ordering::Less | Ordering::Equal)
        )
    }

    /// Materializes the projected span: the explicit, eager form of
    /// this view.
    ///
    /// One [`OwnVersion::to_version`] per endpoint; the projection is
    /// monotone (the type's docs carry the argument), so the
    /// projected pair is ordered and the construction revalidates
    /// nothing.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_project.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let span = a1.span(&a1);
    /// // The seed owns its whole history: the projection is the span.
    /// assert_eq!((&span / alice.party()).to_span(), span);
    /// ```
    pub fn to_span(&self) -> Span<'static> {
        Span::owned(self.lo().to_version(), self.hi().to_version())
    }
}

/// `&span / &party`: the part of the [`Span`] wholly owned by the [`Party`],
/// as a lazy [`OwnSpan`] view.
///
/// [`Span::project`] is the named spelling of the same view.
///
/// # Complexity
///
/// `O(1)`.
///
/// # Example
///
/// ```
/// use before::{causally::Placement, Clock};
/// let mut alice = Clock::seed();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let span = a1.span(&a2);
/// let view = &span / alice.party();
/// // The seed owns everything: the view places like the span.
/// assert_eq!(view.place(&a1), span.place(&a1));
/// ```
impl<'a> Div<&'a Party> for &'a Span<'a> {
    type Output = OwnSpan<'a>;
    fn div(self, party: &'a Party) -> OwnSpan<'a> {
        OwnSpan { party, span: self }
    }
}

/// Materializes the projection, as [`to_span`](OwnSpan::to_span).
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_project.html"))]
///
/// # Example
///
/// ```
/// use before::{Clock, Span};
/// let mut alice = Clock::seed();
/// let a1 = alice.tick().clone();
/// let span = a1.span(&a1);
/// assert_eq!(Span::from(&span / alice.party()), (&span / alice.party()).to_span());
/// ```
impl From<OwnSpan<'_>> for Span<'static> {
    fn from(view: OwnSpan<'_>) -> Span<'static> {
        view.to_span()
    }
}
