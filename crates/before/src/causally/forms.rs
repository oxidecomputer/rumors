//! The query construction vocabulary: the two elementary atoms, their negations
//! and widenings, and the named forms built from them.
//!
//! Every public constructor lives here, so the file reads as the language's
//! lexicon: [`after`]/[`before`] are the atoms, `!` and
//! [`or_concurrent`](Floor::or_concurrent) reach the four negated forms, and
//! the named forms ([`since`], [`until`], [`delta`], [`toward`], the strict
//! relations, [`all`]) are spellings of expressions a caller could write by
//! hand by composition of atoms.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Not;

use super::polarity::{Down, Hole, Up};
use super::{le, Query, Version};

/// Built by [`after`]: keeps the versions at or above its bound, `at <= v`.
///
/// Conjoin it with `&`, negate it with `!`, or widen it with
/// [`or_concurrent`](Self::or_concurrent).
#[derive(Clone)]
pub struct Floor<'a> {
    pub(super) at: Cow<'a, Version>,
}

/// Built by [`before`]: keeps the versions at or below its bound, `v <= at`.
#[derive(Clone)]
pub struct Ceiling<'a> {
    pub(super) at: Cow<'a, Version>,
}

/// Everything at or after `s`.
///
/// Versions concurrent to `s` are dropped; keep them with
/// [`or_concurrent`](Floor::or_concurrent).
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::after;
///
/// assert!(after(&a1).contains(&a2)); // the causal future
/// assert!(after(&a1).contains(&a1)); // the bound itself: inclusive
/// assert!(!after(&a1).contains(&b1)); // concurrent versions drop
/// assert!(after(&a1).or_concurrent().contains(&b1)); // unless widened
/// ```
pub fn after<'a>(s: impl Into<Cow<'a, Version>>) -> Floor<'a> {
    Floor { at: s.into() }
}

/// Everything at or before `e`.
///
/// Versions concurrent to `e` are dropped; keep them with
/// [`or_concurrent`](Ceiling::or_concurrent).
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::before;
///
/// assert!(before(&a2).contains(&a1)); // the causal past
/// assert!(before(&a2).contains(&a2)); // the bound itself: inclusive
/// assert!(!before(&a2).contains(&b1)); // concurrent versions drop
/// assert!(before(&a2).or_concurrent().contains(&b1)); // unless widened
/// ```
pub fn before<'a>(e: impl Into<Cow<'a, Version>>) -> Ceiling<'a> {
    Ceiling { at: e.into() }
}

/// Everything strictly after `s`.
///
/// Equivalent to `after(s) & !before(s)`.
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::strictly_after;
///
/// assert!(strictly_after(&a1).contains(&a2)); // the strict future
/// assert!(!strictly_after(&a1).contains(&a1)); // the bound is excluded
/// assert!(!strictly_after(&a1).contains(&b1)); // so is concurrency
/// ```
pub fn strictly_after<'a>(s: impl Into<Cow<'a, Version>>) -> Query<'a, Down> {
    let at = s.into();
    let hole = Hole {
        at: at.clone(),
        strict: false,
    };
    Query {
        floor: Some(at),
        ceiling: None,
        holes: vec![hole],
        polarity: PhantomData,
    }
}

/// Everything strictly before `e`.
///
/// Equivalent to `before(e) & !after(e)`.
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::strictly_before;
///
/// assert!(strictly_before(&a2).contains(&a1)); // the strict past
/// assert!(!strictly_before(&a2).contains(&a2)); // the bound is excluded
/// assert!(!strictly_before(&a2).contains(&b1)); // so is concurrency
/// ```
pub fn strictly_before<'a>(e: impl Into<Cow<'a, Version>>) -> Query<'a, Up> {
    let at = e.into();
    let hole = Hole {
        at: at.clone(),
        strict: false,
    };
    Query {
        floor: None,
        ceiling: Some(at),
        holes: vec![hole],
        polarity: PhantomData,
    }
}

/// Everything `s` does not already contain.
///
/// This matches anything in `s`'s strict causal future, everything concurrent
/// to it, but *not* `s`.
///
/// Equivalent to `!before(s)`.
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::since;
///
/// assert!(since(&a1).contains(&a2)); // the strict future…
/// assert!(since(&a1).contains(&b1)); // …and everything concurrent
/// assert!(!since(&a1).contains(&a1)); // s itself is already contained
/// ```
pub fn since<'a>(s: impl Into<Cow<'a, Version>>) -> Query<'a, Down> {
    !before(s)
}

/// Everything that does not yet contain `e`.
///
/// This matches anything in `e`'s strict causal past, everything concurrent to
/// it, but *not* `e`.
///
/// Equivalent to `!after(e)`.
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::until;
///
/// assert!(until(&a2).contains(&a1)); // the strict past…
/// assert!(until(&a2).contains(&b1)); // …and everything concurrent
/// assert!(!until(&a2).contains(&a2)); // e is already reached
/// ```
pub fn until<'a>(e: impl Into<Cow<'a, Version>>) -> Query<'a, Up> {
    !after(e)
}

/// Everything in the causal past of `e` (including `e` itself) but nothing in
/// the causal past of `s` (including `s` itself).
///
/// Equivalent to `since(s) & before(e)`.
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::delta;
/// let a3 = alice.tick().clone();
///
/// let d = delta(&a1, &a3);
/// assert!(d.contains(&a2) && d.contains(&a3)); // e's past, e included
/// assert!(!d.contains(&a1)); // s's past excluded, s included
/// assert!(!d.contains(&b1)); // concurrent: outside e's past
/// ```
pub fn delta<'a>(
    s: impl Into<Cow<'a, Version>>,
    e: impl Into<Cow<'a, Version>>,
) -> Query<'a, Down> {
    since(s) & before(e)
}

/// Everything in the causal future of `s` (including `s` itself) but nothing in
/// the causal future of `e` (including `e` itself).
///
/// Equivalent to `after(p) & until(t)`.
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::toward;
/// let a3 = alice.tick().clone();
///
/// let t = toward(&a1, &a3);
/// assert!(t.contains(&a1) && t.contains(&a2)); // s's future, s included
/// assert!(!t.contains(&a3)); // t is already reached: excluded
/// assert!(!t.contains(&b1)); // concurrent: outside s's future
/// ```
pub fn toward<'a>(s: impl Into<Cow<'a, Version>>, t: impl Into<Cow<'a, Version>>) -> Query<'a, Up> {
    after(s) & until(t)
}

/// All versions.
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's line
/// use before::causally::all;
///
/// assert!(all().contains(&a1) && all().contains(&a2));
/// assert!(all().contains(&b1)); // concurrency included: no constraints
/// ```
pub fn all<'a>() -> Query<'a> {
    Query::unbounded()
}

impl<'a> Floor<'a> {
    /// Whether `version` is at or above the bound: `at <= v`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/floor_contains.html"))]
    pub fn contains(&self, version: &Version) -> bool {
        le(&self.at, version)
    }

    /// Widens the query to additionally include all concurrent versions.
    pub fn or_concurrent(self) -> Query<'a, Down> {
        Query {
            floor: None,
            ceiling: None,
            holes: vec![Hole {
                at: self.at,
                strict: true,
            }],
            polarity: PhantomData,
        }
    }
}

impl<'a> Ceiling<'a> {
    /// Whether `version` is at or below the bound: `v <= at`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ceiling_contains.html"))]
    pub fn contains(&self, version: &Version) -> bool {
        le(version, &self.at)
    }

    /// Widens the query to additionally include all concurrent versions.
    pub fn or_concurrent(self) -> Query<'a, Up> {
        Query {
            floor: None,
            ceiling: None,
            holes: vec![Hole {
                at: self.at,
                strict: true,
            }],
            polarity: PhantomData,
        }
    }
}

/// The complement of the upper atom: `!before(s)` keeps `v > s` or
/// `v ∥ s` — this is [`since`], a [`Down`]-polar query.
impl<'a> Not for Ceiling<'a> {
    type Output = Query<'a, Down>;

    fn not(self) -> Query<'a, Down> {
        Query {
            floor: None,
            ceiling: None,
            holes: vec![Hole {
                at: self.at,
                strict: false,
            }],
            polarity: PhantomData,
        }
    }
}

/// The complement of the lower atom: `!after(s)` keeps `v < s` or
/// `v ∥ s` — this is [`until`], an [`Up`]-polar query.
impl<'a> Not for Floor<'a> {
    type Output = Query<'a, Up>;

    fn not(self) -> Query<'a, Up> {
        Query {
            floor: None,
            ceiling: None,
            holes: vec![Hole {
                at: self.at,
                strict: false,
            }],
            polarity: PhantomData,
        }
    }
}

impl fmt::Debug for Floor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "after({:?})", self.at)
    }
}

impl fmt::Debug for Ceiling<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "before({:?})", self.at)
    }
}
