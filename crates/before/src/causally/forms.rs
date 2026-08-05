//! The construction vocabulary: the two elementary atoms, their
//! negations and widenings, and the named forms built from them.
//!
//! Every public constructor lives here, so the file reads as the
//! language's lexicon: [`after`]/[`before`] are the atoms, `!` and
//! [`or_concurrent`](Floor::or_concurrent) reach the four negated
//! forms, and the named forms ([`since`], [`until`], [`delta`],
//! [`toward`], the strict relations, [`all`]) are spellings of
//! expressions a caller could write by hand.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Not;

use super::polarity::{Down, Hole, Up};
use super::{le, Query, Version};

/// The elementary lower atom, built by [`after`]: keeps the versions
/// at or above its bound, `at <= v`.
///
/// Conjoin it with `&`, negate it with `!`, or widen it with
/// [`or_concurrent`](Self::or_concurrent).
#[derive(Clone)]
pub struct Floor<'a> {
    pub(super) at: Cow<'a, Version>,
}

/// The elementary upper atom, built by [`before`]: keeps the versions
/// at or below its bound, `v <= at` — the dual of [`Floor`] in every
/// clause.
#[derive(Clone)]
pub struct Ceiling<'a> {
    pub(super) at: Cow<'a, Version>,
}

/// Everything at or after `p`: the versions that contain `p`,
/// including `p` itself — `p <= v`.
///
/// Versions concurrent to `p` are dropped; keep them with
/// [`or_concurrent`](Floor::or_concurrent) or `!`.
pub fn after<'a>(p: impl Into<Cow<'a, Version>>) -> Floor<'a> {
    Floor { at: p.into() }
}

/// Everything at or before `e`: the versions `e` contains, including
/// `e` itself — `v <= e`, its causal past.
///
/// Versions concurrent to `e` are dropped; keep them with
/// [`or_concurrent`](Ceiling::or_concurrent) or `!`.
pub fn before<'a>(e: impl Into<Cow<'a, Version>>) -> Ceiling<'a> {
    Ceiling { at: e.into() }
}

/// Everything strictly after `p`: the versions that contain `p` and
/// more — `p < v`.
///
/// Equivalent to `after(p) & !before(p)`; the hole makes the query
/// [`Down`]-polar.
pub fn strictly_after<'a>(p: impl Into<Cow<'a, Version>>) -> Query<'a, Down> {
    let at = p.into();
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

/// Everything strictly before `e`: the versions `e` contains,
/// excluding `e` itself — `v < e`.
///
/// Equivalent to `before(e) & !after(e)`; the hole makes the query
/// [`Up`]-polar.
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

/// Everything `s` does not already contain: its causal future and
/// everything concurrent to it.
///
/// Equivalent to `!before(s)`, and named for the resume shape: the
/// boundary version has been seen, so keep what it hasn't. `s` itself
/// is excluded; other parties' concurrent versions pass.
pub fn since<'a>(s: impl Into<Cow<'a, Version>>) -> Query<'a, Down> {
    !before(s)
}

/// Everything that does not yet contain `t`: its strict causal past
/// and everything concurrent to it.
///
/// Equivalent to `!after(t)`, and [`since`]'s dual: `since` keeps
/// what its bound has not seen, `until` keeps what has not seen its
/// bound. `t` itself is excluded; other parties' concurrent versions
/// pass.
pub fn until<'a>(t: impl Into<Cow<'a, Version>>) -> Query<'a, Up> {
    !after(t)
}

/// The causal delta from `s` to `e`: everything `e` covers that `s`
/// does not.
///
/// Equivalent to `since(s) & before(e)`, and total for any bound
/// pair: concurrent bounds ask the anti-entropy question (nonempty —
/// it holds `e` itself), and a reversed pair denotes the empty query.
pub fn delta<'a>(
    s: impl Into<Cow<'a, Version>>,
    e: impl Into<Cow<'a, Version>>,
) -> Query<'a, Down> {
    since(s) & before(e)
}

/// The frontier from `p` toward `t`: everything that has reached `p`
/// but not yet `t`.
///
/// Equivalent to `after(p) & until(t)`, and [`delta`]'s dual: where
/// `delta` measures a difference of pasts (what `e` covers that `s`
/// does not), `toward` measures a difference of futures — which
/// states have applied `p` and still lack `t`. Total for any bound
/// pair, and half-open at `t` exactly as `delta` is at `s`.
pub fn toward<'a>(p: impl Into<Cow<'a, Version>>, t: impl Into<Cow<'a, Version>>) -> Query<'a, Up> {
    after(p) & until(t)
}

/// The unbounded query: every version.
///
/// The identity for `&`, and [`Neutral`](super::Neutral), so it
/// conjoins into either polarity.
pub fn all<'a>() -> Query<'a> {
    Query::unbounded()
}

impl<'a> Floor<'a> {
    /// Whether `version` is at or above the bound: `at <= v`.
    pub fn contains(&self, version: &Version) -> bool {
        le(&self.at, version)
    }

    /// Widens the atom to also keep versions concurrent to its bound:
    /// `at <= v` or `v ∥ at`, which is `¬(v < at)` — a [`Down`]-polar
    /// query.
    ///
    /// `!` reaches the *other* side's complement, `¬(at <= v)`; the
    /// [module docs](super) tabulate all four negated forms.
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
    pub fn contains(&self, version: &Version) -> bool {
        le(version, &self.at)
    }

    /// Widens the atom to also keep versions concurrent to its bound:
    /// `v <= at` or `v ∥ at`, which is `¬(at < v)` — an [`Up`]-polar
    /// query, dual to [`Floor::or_concurrent`].
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
///
/// Negation is where concurrency enters the language: the atom
/// demanded an order relation, and the complement holds wherever that
/// relation fails. Only the two atoms negate; `!(a & b)` would be a
/// union (see the [module docs](super)).
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
/// `v ∥ s` — an [`Up`]-polar query, dual to `!` on [`Ceiling`].
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
