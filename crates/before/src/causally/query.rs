//! Evaluation of causal queries.
//!
//! A [`Query`] is a causal interval minus a same-polarity antichain of holes.
//! Evaluation compiles those bounds into [`Demand`]s for the skyline filter.
//! Large antichains are divided into batches sized from the encoded operands.
//! Each batch shares one probe traversal while the number of live cursors
//! remains proportional to those operands.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use super::polarity::{Hole, Neutral, Polarity};
use super::{le, Version};
use crate::codec::BitsView;
use crate::span::Span;
use crate::version::skyline::place::filter::{self, Demand};

/// A causal filter on [`Version`]s and [`Span`]s within a restricted [`Query`]
/// language.
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
/// inert holes can produce different stored forms for the same predicate; use
/// [`contains`](Self::contains) or [`coverage`](Self::coverage) to compare
/// behavior.
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

    /// One batch of bounds in deterministic read order.
    fn demands<'b>(
        &'b self,
        holes: &'b [Hole<'a>],
    ) -> impl Iterator<Item = (BitsView<'b>, Demand)> {
        self.floor
            .as_deref()
            .map(|p| (p.view().live(), Demand::After))
            .into_iter()
            .chain(Self::hole_demands(holes))
            .chain(
                self.ceiling
                    .as_deref()
                    .map(|e| (e.view().live(), Demand::Before)),
            )
    }

    /// The stored holes as the stream demands consumed by the fused walks.
    fn hole_demands<'b>(holes: &'b [Hole<'a>]) -> impl Iterator<Item = (BitsView<'b>, Demand)> {
        holes
            .iter()
            .map(|hole| (hole.at.view().live(), P::hole_demand(hole.strict)))
    }

    /// Encoded bytes retained by the query's bounds.
    fn bound_bytes(&self) -> usize {
        self.floor
            .iter()
            .chain(self.ceiling.iter())
            .map(|bound| bound.as_bytes().len())
            .chain(self.holes.iter().map(|hole| hole.at.as_bytes().len()))
            .fold(0, usize::saturating_add)
    }

    /// Whether the query admits `version`.
    ///
    /// # Complexity
    ///
    /// With `k` stored bounds, evaluation takes `O(k·n)` time and `O(n)`
    /// auxiliary space for `n` total encoded operand bytes. More precisely, if
    /// `i` is the number of intervals in the streams' common tree overlay and
    /// `p` is the encoded size of `version`'s payloads, time is
    /// `O(n + k·(i + p))`. The charts below show fixed-bound shapes, where
    /// this reduces to `O(n)`:
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
        let input_bytes = self.bound_bytes().saturating_add(version.as_bytes().len());
        let capacity = filter::membership_capacity(input_bytes);
        let fixed = usize::from(self.floor.is_some()) + usize::from(self.ceiling.is_some());
        let first = self.holes.len().min(capacity.saturating_sub(fixed));
        if !filter::admits(version.view().live(), self.demands(&self.holes[..first])) {
            return false;
        }
        self.holes[first..]
            .chunks(capacity)
            .all(|holes| filter::admits(version.view().live(), Self::hole_demands(holes)))
    }

    /// How much of `span` this query admits.
    ///
    /// The probe is anything [`Into`] a [`Span`]: a span itself, or a
    /// [`Version`] (borrowed or owned).
    ///
    /// # Complexity
    ///
    /// With `k` stored bounds, evaluation takes `O(k·n)` time and `O(n)`
    /// auxiliary space for `n` total encoded operand bytes. More precisely, if
    /// `i` is the number of intervals in the streams' common tree overlay and
    /// `p` is the encoded size of the span endpoints' payloads, time is
    /// `O(n + k·(i + p))`. The charts below show fixed-bound shapes, where
    /// this reduces to `O(n)`:
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
        if lo.view().ptr_eq(hi.view()) {
            return if self.contains(lo) {
                Coverage::Full
            } else {
                Coverage::Empty
            };
        }
        let input_bytes = self
            .bound_bytes()
            .saturating_add(lo.as_bytes().len())
            .saturating_add(hi.as_bytes().len());
        let capacity = filter::coverage_capacity(input_bytes);
        let fixed = usize::from(self.floor.is_some()) + usize::from(self.ceiling.is_some());
        let first = self.holes.len().min(capacity.saturating_sub(fixed));
        let mut verdict = filter::coverage(
            lo.view().live(),
            hi.view().live(),
            self.demands(&self.holes[..first]),
        );
        if verdict == Coverage::Empty {
            return Coverage::Empty;
        }
        for holes in self.holes[first..].chunks(capacity) {
            let next = filter::coverage(
                lo.view().live(),
                hi.view().live(),
                Self::hole_demands(holes),
            );
            if next == Coverage::Empty {
                return Coverage::Empty;
            }
            if next == Coverage::Partial {
                verdict = Coverage::Partial;
            }
        }
        match verdict {
            Coverage::Full => Coverage::Full,
            Coverage::Partial => self.refine_partial(lo, hi, input_bytes),
            Coverage::Empty => unreachable!("empty batches return immediately"),
        }
    }

    /// The clamp refinement behind [`coverage`](Self::coverage)'s `Partial`
    /// arm: the exact emptiness decision the first endpoint walk cannot reach.
    ///
    /// The admitted portion of the segment is the *clamped* segment `[lo ∨
    /// floor, hi ∧ ceiling]` minus the holes. A crossed clamp is empty
    /// outright. A down-set covering the clamped top covers the whole clamped
    /// segment, *because* every hole shares one polarity: the joint-covering
    /// case that would escape this endpoint test needs both polarities at once,
    /// which the type refuses. Each batch shares one endpoint traversal among
    /// its holes instead of decoding the endpoint once per hole.
    fn refine_partial(&self, lo: &Version, hi: &Version, input_bytes: usize) -> Coverage {
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
        let capacity = filter::membership_capacity(input_bytes);
        for holes in self.holes.chunks(capacity) {
            if !filter::admits(endpoint.view().live(), Self::hole_demands(holes)) {
                return Coverage::Empty;
            }
        }
        Coverage::Partial
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
