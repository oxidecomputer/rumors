//! Named, composable constructors for causal [`Version`] ranges.
//!
//! On totally ordered values a range is an interval. On causal [`Version`]s
//! — which are only *partially* ordered — the useful generalization is a
//! **difference of down-sets**: keep the versions contained in the end
//! bound, subtract the versions contained in the start bound. The
//! constructors here name each bound's meaning so a filter reads as a
//! sentence, and every start composes with every end:
//!
//! | | end unbounded | [`known_at(e)`](known_at): `v <= e` | [`before(e)`](before): `v < e` |
//! |---|---|---|---|
//! | **start unbounded** | [`all()`](all) | `known_at(&e)` | `before(&e)` |
//! | **[`not_before(s)`](not_before): subtract `v < s`** | `not_before(&s)` | `not_before(&s).known_at(&e)` | `not_before(&s).before(&e)` |
//! | **[`since(s)`](since): subtract `v <= s`** | `since(&s)` | `since(&s).known_at(&e)`, a.k.a. [`delta`] | `since(&s).before(&e)`, a.k.a. [`delta_before`] |
//!
//! The asymmetry inherent to the partial order: a start bound of either
//! kind keeps versions *concurrent* to it (subtraction removes only the
//! bound's causal past — "everything since `s`" must not drop other
//! parties' concurrent versions), while an end bound of either kind drops
//! them (keeping demands containment).
//!
//! Every constructor returns a [`Range`], which implements
//! [`RangeBounds<Version>`] so it can be handed to any version-ranged API,
//! and offers [`contains`](Range::contains) as the authoritative membership
//! predicate.
//!
//! ```
//! use before::{Clock, causally};
//!
//! let mut alice = Clock::seed();
//! let mut bob = alice.fork();
//! let a1 = alice.tick().clone();
//! let b1 = bob.tick().clone(); // concurrent to a1
//! let a2 = alice.tick().clone(); // a1 < a2
//!
//! // A start bound subtracts only its causal past: versions concurrent to
//! // it pass.
//! assert!(causally::since(&a1).contains(&a2));
//! assert!(causally::since(&a1).contains(&b1));
//! assert!(!causally::since(&a1).contains(&a1));
//! // `not_before` differs only at the bound itself.
//! assert!(causally::not_before(&a1).contains(&a1));
//!
//! // An end bound demands containment: concurrent versions are dropped.
//! assert!(causally::known_at(&a2).contains(&a1));
//! assert!(!causally::known_at(&a2).contains(&b1));
//!
//! // Every start composes with every end, in either order.
//! let range = causally::since(&a1).known_at(&a2);
//! assert!(range.contains(&a2));
//! assert!(!range.contains(&b1));
//! assert_eq!(causally::delta(&a1, &a2), range);
//! ```

use std::cmp::Ordering;
use std::ops::{Bound, RangeBounds};

use crate::Version;

/// A causal version range: a pair of [`Bound`]s denoting a difference of
/// causal down-sets (see the [module docs](self) for the semantics and the
/// full constructor table).
///
/// Build one with the module's constructors and refine it with the
/// same-named methods; every composition is valid, in any order, and
/// setting a bound that is already set keeps the latest value. The struct
/// implements [`RangeBounds<Version>`] for use with version-ranged APIs.
///
/// Note that [`Range::contains`] — the causal membership predicate — is
/// deliberately *not* [`RangeBounds::contains`]: the trait's default method
/// requires the item to dominate the start bound, which on a partial order
/// silently drops versions concurrent to it. The inherent method shadows
/// the default so the natural call gets the causal semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range<'a> {
    start: Bound<&'a Version>,
    end: Bound<&'a Version>,
}

/// The unbounded range: every version. The identity for composition —
/// refine it with any of [`Range`]'s methods.
pub fn all<'a>() -> Range<'a> {
    Range {
        start: Bound::Unbounded,
        end: Bound::Unbounded,
    }
}

/// Everything *strictly since* `start`: versions not contained in `start`,
/// i.e. its causal future and everything concurrent to it. `start` itself
/// is excluded — this is the resume/subscription shape, where the boundary
/// version has already been seen.
pub fn since(start: &Version) -> Range<'_> {
    all().since(start)
}

/// Everything *not strictly before* `start`: like [`since`], but `start`
/// itself is included — the replay-the-boundary shape. (The name follows
/// X.509's `notBefore`: on a partial order, "not before" is honest where
/// "at or after" would not be, since concurrent versions are neither.)
pub fn not_before(start: &Version) -> Range<'_> {
    all().not_before(start)
}

/// Everything *known at* `end`: versions contained in `end` — its causal
/// past, inclusive.
pub fn known_at(end: &Version) -> Range<'_> {
    all().known_at(end)
}

/// Everything *strictly before* `end`: versions contained in `end`,
/// exclusive of `end` itself.
pub fn before(end: &Version) -> Range<'_> {
    all().before(end)
}

/// The causal delta from `start` to `end`: everything known at `end` but
/// not at `start` — exactly what a replica at `start` must receive to reach
/// `end`. Shorthand for [`since(start)`](since)[`.known_at(end)`](Range::known_at).
pub fn delta<'a>(start: &'a Version, end: &'a Version) -> Range<'a> {
    since(start).known_at(end)
}

/// The half-open causal delta: everything strictly since `start` and
/// strictly before `end`. Shorthand for
/// [`since(start)`](since)[`.before(end)`](Range::before).
pub fn delta_before<'a>(start: &'a Version, end: &'a Version) -> Range<'a> {
    since(start).before(end)
}

impl<'a> Range<'a> {
    /// Refine the start bound to *strictly since* `start` (see [`since`]).
    pub fn since(self, start: &'a Version) -> Self {
        Self {
            start: Bound::Excluded(start),
            ..self
        }
    }

    /// Refine the start bound to *not strictly before* `start` (see
    /// [`not_before`]).
    pub fn not_before(self, start: &'a Version) -> Self {
        Self {
            start: Bound::Included(start),
            ..self
        }
    }

    /// Refine the end bound to *known at* `end` (see [`known_at`]).
    pub fn known_at(self, end: &'a Version) -> Self {
        Self {
            end: Bound::Included(end),
            ..self
        }
    }

    /// Refine the end bound to *strictly before* `end` (see [`before`]).
    pub fn before(self, end: &'a Version) -> Self {
        Self {
            end: Bound::Excluded(end),
            ..self
        }
    }

    /// The causal membership predicate: whether `version` is contained in
    /// the end bound and *not* contained in the start bound. Equivalent to
    /// [`placement_of`](Self::placement_of) returning
    /// [`Equal`](Ordering::Equal).
    ///
    /// Per bound kind, for a version `v`:
    ///
    /// - start unbounded: nothing subtracted; [`since(s)`](since): `v <= s`
    ///   subtracted; [`not_before(s)`](not_before): `v < s` subtracted.
    /// - end unbounded: everything kept; [`known_at(e)`](known_at): `v <= e`
    ///   kept; [`before(e)`](before): `v < e` kept.
    ///
    /// This deliberately shadows the [`RangeBounds::contains`] default,
    /// whose start check would also drop versions concurrent to the start
    /// bound (see [`Range`]).
    pub fn contains(&self, version: &Version) -> bool {
        self.placement_of(version) == Ordering::Equal
    }

    /// Totally order `version` against this range: where the causal order
    /// on [`Version`]s alone is partial, a version's placement relative to
    /// a range is always one of exactly three cases.
    ///
    /// - [`Less`](Ordering::Less): the start bound subtracts it — it is in
    ///   the range's past.
    /// - [`Equal`](Ordering::Equal): the range [`contains`](Self::contains)
    ///   it.
    /// - [`Greater`](Ordering::Greater): the end bound does not contain it
    ///   — its causal future *or* something concurrent to it; "beyond the
    ///   range", not necessarily after every version in it.
    ///
    /// The totality lives in the signature: a bare [`Ordering`], no
    /// [`Option`], where [`Version`]-to-[`Version`] comparison must return
    /// [`Option<Ordering>`](PartialOrd::partial_cmp). (No operator
    /// overloads back this: a cross-type `PartialEq` whose `==` meant
    /// membership would violate the trait's transitivity contract.)
    ///
    /// For a *crossed* range (whose start bound is not within its end
    /// bound) a version can fail both bounds; such a version classifies as
    /// [`Less`](Ordering::Less). Well-formed ranges have no such case.
    pub fn placement_of(&self, version: &Version) -> Ordering {
        let past_start = match self.start {
            Bound::Unbounded => true,
            // Greater than or concurrent to the bound: not in its causal
            // past, so not subtracted.
            Bound::Excluded(start) => {
                matches!(version.partial_cmp(start), None | Some(Ordering::Greater))
            }
            // As above, but the bound itself also survives.
            Bound::Included(start) => matches!(
                version.partial_cmp(start),
                None | Some(Ordering::Equal | Ordering::Greater)
            ),
        };
        if !past_start {
            return Ordering::Less;
        }
        let within_end = match self.end {
            Bound::Unbounded => true,
            Bound::Included(end) => version <= end,
            Bound::Excluded(end) => version < end,
        };
        if within_end {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}

impl RangeBounds<Version> for Range<'_> {
    fn start_bound(&self) -> Bound<&Version> {
        self.start
    }

    fn end_bound(&self) -> Bound<&Version> {
        self.end
    }
}

#[cfg(test)]
mod tests;
