//! Evaluation of causal queries.
//!
//! A [`Query`] is a causal interval minus a same-polarity antichain of holes.
//! Evaluation turns those bounds into [`Demand`]s and checks them in one
//! synchronized traversal of the probe and every bound.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use super::polarity::{Hole, Neutral, Polarity};
use super::{le, Version};
use crate::span::Span;
use crate::version::place::filter::{self, Demand};

/// A causal predicate over [`Version`]s and [`Span`]s.
///
/// Queries are composed from the atomic queries in this module, which may be
/// negated with `!` and combined with `&`.
///
/// Not all queries may be combined: arbitrary negation can encode Boolean
/// satisfiability when deciding exact [`Span`] overlap. The [`Polarity`]
/// restriction enforced by the types of [`Query`] avoids that combinatorial
/// search.
///
/// Queries intentionally have no structural equality. Construction order and
/// redundant exclusions can produce different stored forms for the same
/// predicate; use [`contains`](Self::contains) or [`coverage`](Self::coverage)
/// to compare behavior.
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct Query<'a, P: Polarity = Neutral> {
    pub(super) floor: Option<Cow<'a, Version>>,
    pub(super) ceiling: Option<Cow<'a, Version>>,
    pub(super) holes: Vec<Hole<'a>>,
    pub(super) polarity: PhantomData<P>,
}

/// Clones in `O(k)` time and space for `k` stored bounds. Each cloned bound
/// shares its version buffer.
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

/// How much of a [`Span`]'s segment a [`Query`] admits.
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

    /// Build an unbounded query from inclusive holes that are already a
    /// pairwise-unabsorbed antichain.
    #[cfg(feature = "meter")]
    pub(crate) fn from_inclusive_holes(holes: Vec<Version>) -> Query<'static, P> {
        Query {
            floor: None,
            ceiling: None,
            holes: holes
                .into_iter()
                .map(|at| Hole {
                    at: Cow::Owned(at),
                    strict: false,
                })
                .collect(),
            polarity: PhantomData,
        }
    }

    /// Every bound in deterministic read order.
    fn demands(&self) -> impl Iterator<Item = (&Version, Demand)> {
        self.floor
            .as_deref()
            .map(|p| (p, Demand::After))
            .into_iter()
            .chain(Self::hole_demands(&self.holes))
            .chain(self.ceiling.as_deref().map(|e| (e, Demand::Before)))
    }

    /// The stored holes as the stream demands consumed by the fused walks.
    fn hole_demands<'b>(holes: &'b [Hole<'a>]) -> impl Iterator<Item = (&'b Version, Demand)> {
        holes
            .iter()
            .map(|hole| (hole.at.as_ref(), P::hole_demand(hole.strict)))
    }

    /// Whether the query admits `version`.
    ///
    /// # Complexity
    ///
    /// With `k` stored bounds and `n` total operand bytes, evaluation takes
    /// `O(k·n)` time and `O(n + k)` auxiliary space. Memory includes a fixed
    /// cost per bound, so many small bounds can have substantial overhead. The
    /// charts below show fixed-bound shapes, where time reduces to `O(n)`:
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_contains_floor.html")))]
    #[cfg_attr(
        not(doc),
        doc = "floor only: `O(n)` in total input bytes; `O(|self| + |version|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_contains_ceiling.html")))]
    #[cfg_attr(
        not(doc),
        doc = "ceiling only: `O(n)` in total input bytes; `O(|self| + |version|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_contains_floor_ceiling.html")))]
    #[cfg_attr(
        not(doc),
        doc = "floor + ceiling: `O(n)` in total input bytes; `O(|self| + |version|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_contains_hole.html")))]
    #[cfg_attr(
        not(doc),
        doc = "one hole: `O(n)` in total input bytes; `O(|self| + |version|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_contains_floor_hole.html")))]
    #[cfg_attr(
        not(doc),
        doc = "floor + hole: `O(n)` in total input bytes; `O(|self| + |version|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_contains_ceiling_hole.html")))]
    #[cfg_attr(
        not(doc),
        doc = "ceiling + hole: `O(n)` in total input bytes; `O(|self| + |version|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_contains_floor_ceiling_hole.html")))]
    #[cfg_attr(
        not(doc),
        doc = "floor + ceiling + hole: `O(n)` in total input bytes; `O(|self| + |version|)`"
    )]
    pub fn contains(&self, version: &Version) -> bool {
        filter::admits(version, self.demands())
    }

    /// How much of `span` this query admits.
    ///
    /// The probe is anything [`Into`] a [`Span`]: a span itself, or a
    /// [`Version`] (borrowed or owned).
    ///
    /// # Complexity
    ///
    /// With `k` stored bounds and `n` total operand bytes, evaluation takes
    /// `O(k·n)` time and `O(n + k)` auxiliary space. Memory includes a fixed
    /// cost per bound, so many small bounds can have substantial overhead. The
    /// charts below show fixed-bound shapes, where time reduces to `O(n)`:
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_coverage_floor.html")))]
    #[cfg_attr(
        not(doc),
        doc = "floor only: `O(n)` in total input bytes; `O(|self| + |span|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_coverage_ceiling.html")))]
    #[cfg_attr(
        not(doc),
        doc = "ceiling only: `O(n)` in total input bytes; `O(|self| + |span|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_coverage_floor_ceiling.html")))]
    #[cfg_attr(
        not(doc),
        doc = "floor + ceiling: `O(n)` in total input bytes; `O(|self| + |span|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_coverage_hole.html")))]
    #[cfg_attr(
        not(doc),
        doc = "one hole: `O(n)` in total input bytes; `O(|self| + |span|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_coverage_floor_hole.html")))]
    #[cfg_attr(
        not(doc),
        doc = "floor + hole: `O(n)` in total input bytes; `O(|self| + |span|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_coverage_ceiling_hole.html")))]
    #[cfg_attr(
        not(doc),
        doc = "ceiling + hole: `O(n)` in total input bytes; `O(|self| + |span|)`"
    )]
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_coverage_floor_ceiling_hole.html")))]
    #[cfg_attr(
        not(doc),
        doc = "floor + ceiling + hole: `O(n)` in total input bytes; `O(|self| + |span|)`"
    )]
    pub fn coverage<'s>(&self, span: impl Into<Span<'s>>) -> Coverage {
        let span = span.into();
        let (lo, hi) = (span.lo(), span.hi());
        if lo.ptr_eq(hi) {
            return if self.contains(lo) {
                Coverage::Full
            } else {
                Coverage::Empty
            };
        }
        match filter::coverage(lo, hi, self.demands()) {
            Coverage::Full => Coverage::Full,
            Coverage::Empty => Coverage::Empty,
            Coverage::Partial => self.refine_partial(lo, hi),
        }
    }

    /// The clamp refinement behind [`coverage`](Self::coverage)'s `Partial`
    /// arm: the exact emptiness decision the fused endpoint walk cannot reach.
    ///
    /// The admitted portion of the segment is the *clamped* segment `[lo ∨
    /// floor, hi ∧ ceiling]` minus the holes. A crossed clamp is empty
    /// outright. A down-set covering the clamped top covers the whole clamped
    /// segment, *because* every hole shares one polarity: the joint-covering
    /// case that would escape this endpoint test needs both polarities at once,
    /// which the type refuses.
    fn refine_partial(&self, lo: &Version, hi: &Version) -> Coverage {
        let clamped_lo: Cow<'_, Version> = match self.floor.as_deref() {
            Some(floor) => Cow::Owned(lo | floor),
            None => Cow::Borrowed(lo),
        };
        let clamped_hi: Cow<'_, Version> = match self.ceiling.as_deref() {
            Some(ceiling) => Cow::Owned(hi & ceiling),
            None => Cow::Borrowed(hi),
        };
        if !le(&clamped_lo, &clamped_hi) {
            return Coverage::Empty;
        }
        if self.holes.is_empty() {
            return Coverage::Partial;
        }

        let endpoint = P::covering_endpoint(&clamped_lo, &clamped_hi);
        if filter::admits(endpoint, Self::hole_demands(&self.holes)) {
            Coverage::Partial
        } else {
            Coverage::Empty
        }
    }

    /// Converts every borrowed bound into a buffer-sharing owned version.
    ///
    /// # Complexity
    ///
    /// `O(k)` time and space for `k` stored bounds. Owned versions move;
    /// borrowed versions share their stored buffers.
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
    /// Recasts the hole-free query at any polarity: a neutral query holds no
    /// holes structurally, so the reinterpretation moves no meaning.
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

/// Renders the query as the causal expression it denotes.
///
/// Strict holes use their equivalent negated strict atoms, such as
/// `!strictly_before(v)`.
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
