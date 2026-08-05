//! The `From` doors into [`Query`]: atoms, spans, versions, and
//! borrowed queries, so `impl Into<Query>` APIs accept the whole
//! vocabulary.

use std::marker::PhantomData;

use super::forms::{after, before, Ceiling, Floor};
use super::polarity::{Neutral, Polarity};
use super::{Query, Span, Version};

/// A bare floor is a neutral query (no holes), which conjoins into
/// either polarity.
impl<'a> From<Floor<'a>> for Query<'a, Neutral> {
    fn from(floor: Floor<'a>) -> Query<'a, Neutral> {
        Query {
            floor: Some(floor.at),
            ceiling: None,
            holes: Vec::new(),
            polarity: PhantomData,
        }
    }
}

/// A bare ceiling is a neutral query, dually to the [`Floor`]
/// conversion.
impl<'a> From<Ceiling<'a>> for Query<'a, Neutral> {
    fn from(ceiling: Ceiling<'a>) -> Query<'a, Neutral> {
        Query {
            floor: None,
            ceiling: Some(ceiling.at),
            holes: Vec::new(),
            polarity: PhantomData,
        }
    }
}

/// A borrowed query converts by cloning (`O(1)` per stored bound:
/// buffer-sharing clones), so APIs taking `impl Into<Query>` accept a
/// held query without consuming it.
impl<'a, P: Polarity> From<&Query<'a, P>> for Query<'a, P> {
    fn from(query: &Query<'a, P>) -> Query<'a, P> {
        query.clone()
    }
}

/// A [`Span`]'s segment as a query: `after(lo) & before(hi)`, which
/// admits exactly the versions the span covers. Hole-free, hence
/// [`Neutral`].
impl<'a> From<Span<'a>> for Query<'a, Neutral> {
    fn from(span: Span<'a>) -> Query<'a, Neutral> {
        let (lo, hi) = span.into_parts();
        after(lo) & before(hi)
    }
}

/// A borrowed [`Span`]'s segment as a query, lending its endpoints —
/// the borrowing spelling of the consuming conversion.
impl<'a> From<&'a Span<'_>> for Query<'a, Neutral> {
    fn from(span: &'a Span<'_>) -> Query<'a, Neutral> {
        after(span.meet()) & before(span.join())
    }
}

/// A [`Version`] as a query: `after(v) & before(v)`, which by
/// antisymmetry admits exactly `v` — the singleton query.
impl From<Version> for Query<'static, Neutral> {
    fn from(version: Version) -> Query<'static, Neutral> {
        // One buffer-sharing clone fills the second atom; both bounds
        // read one stored buffer.
        after(version.clone()) & before(version)
    }
}

/// A borrowed [`Version`] as a query — the singleton, lending the
/// version to both atoms.
impl<'a> From<&'a Version> for Query<'a, Neutral> {
    fn from(version: &'a Version) -> Query<'a, Neutral> {
        after(version) & before(version)
    }
}
