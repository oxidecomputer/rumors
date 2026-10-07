//! The type-level restriction that every hole in a query points in the same
//! causal direction.
//!
//! Uniform polarity makes exact span coverage decidable without searching
//! combinations of holes. The sealed dispatch states the order-dual behavior
//! of [`Down`] and [`Up`] once, leaving query evaluation generic.

use std::borrow::Cow;

use crate::Version;

/// One subtracted hole: the complement of an elementary bound at a
/// stored version.
///
/// What the hole subtracts is the owning query's polarity: a [`Down`] hole
/// subtracts `v <= at` (`v < at` when `strict`), an [`Up`] hole subtracts `at
/// <= v` (`at < v` when `strict`). A [`Neutral`] query holds none.
#[derive(Clone)]
pub(super) struct Hole<'a> {
    pub(super) at: Cow<'a, Version>,
    pub(super) strict: bool,
}

mod sealed {
    // The dispatch signatures mention crate-private types (`Hole`, the walk
    // demands). The trait is nameable only inside this crate, so the lint's
    // reachability finding is theoretical.
    #![allow(private_interfaces)]

    use core::cmp::Ordering;
    use std::borrow::Cow;

    use super::{Hole, Version};
    use crate::version::place::filter::Demand;

    /// The polarity dispatch: how one hole of this polarity behaves, stated
    /// once per marker.
    pub trait Sealed {
        /// The fused walks' demand for one hole.
        fn hole_demand(strict: bool) -> Demand;

        /// Builds the endpoint of the clamped segment `[lo ∨ floor, hi ∧
        /// ceiling]` that decides whether one of this polarity's holes covers
        /// the whole clamped segment.
        ///
        /// A down-set covers the clamped segment iff it covers the top `hi ∧
        /// ceiling`; an up-set covers it iff it covers the bottom `lo ∨
        /// floor`. An absent bound clamps nothing, so the endpoint is then
        /// borrowed rather than built.
        fn covering_endpoint<'v>(
            lo: &'v Version,
            hi: &'v Version,
            floor: Option<&Version>,
            ceiling: Option<&Version>,
        ) -> Cow<'v, Version>;

        /// Whether `hole` still subtracts something from an interval bounded by
        /// `floor`/`ceiling`.
        fn hole_survives(
            hole: &Hole<'_>,
            floor: Option<&Version>,
            ceiling: Option<&Version>,
        ) -> bool;
        /// Whether hole `a` subtracts a superset of what hole `b` subtracts.
        fn absorbs(a: &Hole<'_>, b: &Hole<'_>) -> bool;
        /// The negated atom name a hole renders as in `Debug`.
        fn hole_name(strict: bool) -> &'static str;
    }

    impl Sealed for super::Down {
        fn hole_demand(strict: bool) -> Demand {
            if strict {
                Demand::NotStrictlyBefore
            } else {
                Demand::NotBefore
            }
        }

        fn covering_endpoint<'v>(
            _lo: &'v Version,
            hi: &'v Version,
            _floor: Option<&Version>,
            ceiling: Option<&Version>,
        ) -> Cow<'v, Version> {
            match ceiling {
                Some(ceiling) => Cow::Owned(hi & ceiling),
                None => Cow::Borrowed(hi),
            }
        }

        fn hole_survives(
            hole: &Hole<'_>,
            floor: Option<&Version>,
            _ceiling: Option<&Version>,
        ) -> bool {
            match floor {
                Some(floor) => {
                    if hole.strict {
                        floor < hole.at.as_ref()
                    } else {
                        floor <= hole.at.as_ref()
                    }
                }
                None => true,
            }
        }

        fn absorbs(a: &Hole<'_>, b: &Hole<'_>) -> bool {
            // `{v <= b} ⊆ {v <= a}` whenever `b < a` regardless of strictness
            // (everything at most `b` is then strictly below `a`); at equal
            // bounds the inclusive hole covers the strict one.
            match b.at.partial_cmp(&a.at) {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => !a.strict || b.strict,
                Some(Ordering::Greater) | None => false,
            }
        }

        fn hole_name(strict: bool) -> &'static str {
            if strict {
                "!strictly_before"
            } else {
                "!before"
            }
        }
    }

    impl Sealed for super::Up {
        fn hole_demand(strict: bool) -> Demand {
            if strict {
                Demand::NotStrictlyAfter
            } else {
                Demand::NotAfter
            }
        }

        fn covering_endpoint<'v>(
            lo: &'v Version,
            _hi: &'v Version,
            floor: Option<&Version>,
            _ceiling: Option<&Version>,
        ) -> Cow<'v, Version> {
            match floor {
                Some(floor) => Cow::Owned(lo | floor),
                None => Cow::Borrowed(lo),
            }
        }

        fn hole_survives(
            hole: &Hole<'_>,
            _floor: Option<&Version>,
            ceiling: Option<&Version>,
        ) -> bool {
            match ceiling {
                Some(ceiling) => {
                    if hole.strict {
                        hole.at.as_ref() < ceiling
                    } else {
                        hole.at.as_ref() <= ceiling
                    }
                }
                None => true,
            }
        }

        fn absorbs(a: &Hole<'_>, b: &Hole<'_>) -> bool {
            // The order-dual of the down-side rule.
            match b.at.partial_cmp(&a.at) {
                Some(Ordering::Greater) => true,
                Some(Ordering::Equal) => !a.strict || b.strict,
                Some(Ordering::Less) | None => false,
            }
        }

        fn hole_name(strict: bool) -> &'static str {
            if strict {
                "!strictly_after"
            } else {
                "!after"
            }
        }
    }

    impl Sealed for super::Neutral {
        // A neutral query holds no holes, structurally: no construction
        // path adds one, so the dispatch is never consulted.
        fn hole_demand(_strict: bool) -> Demand {
            unreachable!("a neutral query holds no holes")
        }

        fn covering_endpoint<'v>(
            _lo: &'v Version,
            _hi: &'v Version,
            _floor: Option<&Version>,
            _ceiling: Option<&Version>,
        ) -> Cow<'v, Version> {
            unreachable!("a neutral query holds no holes")
        }

        fn hole_survives(
            _hole: &Hole<'_>,
            _floor: Option<&Version>,
            _ceiling: Option<&Version>,
        ) -> bool {
            unreachable!("a neutral query holds no holes")
        }

        fn absorbs(_a: &Hole<'_>, _b: &Hole<'_>) -> bool {
            unreachable!("a neutral query holds no holes")
        }

        fn hole_name(_strict: bool) -> &'static str {
            unreachable!("a neutral query holds no holes")
        }
    }
}

/// A query's polarity: which complement family it may subtract.
///
/// Restricting every hole to one direction lets
/// [`Query::coverage`](crate::causally::Query::coverage) remain exact without
/// searching combinations of opposing holes.
///
/// The marker types describe which holes a query may contain:
///
/// - A [`Neutral`] query has no holes; it is a pure causal
///   range.
/// - A [`Down`] query has one or more holes which are
///   *down-sets* (sets of versions with a common upper-bound).
/// - An [`Up`] query has one or more holes which are
///   *up-sets* (sets of versions with a common lower-bound).
///
/// Queries of opposite polarity cannot be combined.
pub trait Polarity: sealed::Sealed + Send + Sync + 'static {}

/// Holes in a [`Query<'_, Down>`](crate::causally::Query) subtract sets of [`Version`]s
/// with a common *upper* bound.
#[derive(Debug, Clone, Copy)]
pub enum Down {}

/// Holes in a [`Query<'_, Up>`](crate::causally::Query) subtract sets of [`Version`]s
/// with a common *lower* bound.
#[derive(Debug, Clone, Copy)]
pub enum Up {}

/// Any [`Query<'_, Neutral>`](crate::causally::Query) has no subtracted sets of
/// [`Version`]s.
#[derive(Debug, Clone, Copy)]
pub enum Neutral {}

impl Polarity for Down {}
impl Polarity for Up {}
impl Polarity for Neutral {}
