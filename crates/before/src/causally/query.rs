//! Evaluation of causal queries.
//!
//! A [`Query`] is a causal interval minus a same-polarity antichain of holes.
//! Evaluation turns those bounds into [`Demand`]s and checks them in one
//! synchronized traversal of the probe and every bound.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use super::polarity::{Hole, Neutral, Polarity};
use crate::span::Span;
use crate::version::place::filter::{self, Demand};
use crate::Version;

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
            // The walk's `Partial` certifies `floor <= hi` and `lo <= ceiling`,
            // the precondition the refinement decides by.
            Coverage::Partial => self.refine_partial(lo, hi),
        }
    }

    /// Refines the fused walk's `Partial` verdict over `[lo, hi]` to the exact
    /// verdict, deciding the emptiness that endpoint comparisons cannot reach.
    ///
    /// Call this only after [`filter::coverage`] has returned
    /// [`Coverage::Partial`] for this query's demands over `[lo, hi]`.
    /// Otherwise the result is unspecified, and debug builds panic when either
    /// certified relation below fails. That verdict certifies `floor <= hi` and
    /// `lo <= ceiling` for whichever bounds the query holds, and the span
    /// certifies `lo <= hi`.
    ///
    /// The admitted portion of the segment is the *clamped* segment `[lo ∨
    /// floor, hi ∧ ceiling]` minus the holes, where an absent bound clamps
    /// nothing. The clamped segment is nonempty iff `lo ∨ floor <= hi ∧
    /// ceiling`, which holds iff each of `lo` and `floor` is at most each of
    /// `hi` and `ceiling`. The walk's verdict and the span supply three of
    /// those four relations. The clamp is therefore crossed exactly when the
    /// query holds both bounds and `floor <= ceiling` fails, a property of the
    /// query alone that needs neither clamped endpoint.
    ///
    /// The holes share one polarity, so together they form one down-set (or,
    /// dually, one up-set), which covers the clamped segment iff it covers the
    /// clamped top (or bottom). A down-set and an up-set could jointly cover
    /// the segment while the down-set misses the top and the up-set misses the
    /// bottom, so no single endpoint test would see it; that is why the type
    /// refuses to mix them. Only that one clamped endpoint is built, and only
    /// when the query holds holes.
    fn refine_partial(&self, lo: &Version, hi: &Version) -> Coverage {
        let (floor, ceiling) = (self.floor.as_deref(), self.ceiling.as_deref());
        debug_assert!(
            floor.is_none_or(|floor| floor <= hi) && ceiling.is_none_or(|ceiling| lo <= ceiling),
            "a `Partial` coverage walk certifies `floor <= hi` and `lo <= ceiling`"
        );
        let clamp_is_nonempty = match (floor, ceiling) {
            (Some(floor), Some(ceiling)) => floor <= ceiling,
            _ => true,
        };
        if !clamp_is_nonempty {
            return Coverage::Empty;
        }
        if self.holes.is_empty() {
            return Coverage::Partial;
        }

        let endpoint = P::covering_endpoint(lo, hi, floor, ceiling);
        if filter::admits(&endpoint, Self::hole_demands(&self.holes)) {
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
