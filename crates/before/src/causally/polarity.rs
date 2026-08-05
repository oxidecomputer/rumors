//! The polarity markers and the sealed per-marker dispatch.
//!
//! Everything a query does with a hole — the walk demand it
//! contributes, its membership and covering predicates, the vacuity
//! and absorption tests behind the conjunction normal form, its
//! `Debug` spelling — is one method on the sealed dispatch trait,
//! stated once per marker. Concentrating the whole behavioral
//! difference between [`Down`] and [`Up`] in this one table is what
//! lets every other file treat the two polarities uniformly.

use std::borrow::Cow;

use crate::Version;

/// One subtracted hole: the complement of an elementary bound at a
/// stored version.
///
/// What the hole subtracts is the owning query's polarity: a [`Down`]
/// hole subtracts `v <= at` (`v < at` when `strict`), an [`Up`] hole
/// subtracts `at <= v` (`at < v` when `strict`). A [`Neutral`] query
/// holds none.
#[derive(Clone)]
pub(super) struct Hole<'a> {
    pub(super) at: Cow<'a, Version>,
    pub(super) strict: bool,
}

mod sealed {
    // The dispatch signatures mention crate-private types (`Hole`, the
    // walk demands). The trait is nameable only inside this crate —
    // its one public appearance is `Polarity`'s supertrait position,
    // from which no external caller can reach these items — so the
    // lint's reachability finding is theoretical.
    #![allow(private_interfaces)]

    use core::cmp::Ordering;

    use super::super::{le, lt};
    use super::{Hole, Version};
    use crate::version::skyline::place::filter::Demand;

    /// The polarity dispatch: how one hole of this polarity behaves,
    /// stated once per marker. Sealed — the three markers are the
    /// whole space, and the exactness argument quantifies over
    /// exactly them.
    pub trait Sealed {
        /// The fused walks' demand for one hole.
        fn hole_demand(strict: bool) -> Demand;
        /// Whether `hole` subtracts `probe` — the hole's own
        /// membership predicate.
        fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool;
        /// Whether `hole` subtracts everything the clamped segment
        /// covers: a down-set covering the clamped top covers all of
        /// it, an up-set dually reaching the clamped bottom.
        fn hole_covers(hole: &Hole<'_>, clamped_lo: &Version, clamped_hi: &Version) -> bool;
        /// Whether `hole` still subtracts something from an interval
        /// bounded by `floor`/`ceiling` — the vacuity test, always
        /// comparative, against the bound on the hole's own side.
        fn hole_survives(
            hole: &Hole<'_>,
            floor: Option<&Version>,
            ceiling: Option<&Version>,
        ) -> bool;
        /// Whether hole `a` subtracts a superset of what hole `b`
        /// subtracts — the absorption test.
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

        fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
            if hole.strict {
                lt(probe, &hole.at)
            } else {
                le(probe, &hole.at)
            }
        }

        fn hole_covers(hole: &Hole<'_>, _clamped_lo: &Version, clamped_hi: &Version) -> bool {
            Self::hole_subtracts(hole, clamped_hi)
        }

        fn hole_survives(
            hole: &Hole<'_>,
            floor: Option<&Version>,
            _ceiling: Option<&Version>,
        ) -> bool {
            match floor {
                Some(floor) => {
                    if hole.strict {
                        lt(floor, &hole.at)
                    } else {
                        le(floor, &hole.at)
                    }
                }
                None => true,
            }
        }

        fn absorbs(a: &Hole<'_>, b: &Hole<'_>) -> bool {
            // `{v <= b} ⊆ {v <= a}` whenever `b < a` regardless of
            // strictness (everything at most `b` is then strictly below
            // `a`); at equal bounds the inclusive hole covers the strict
            // one.
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

        fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
            if hole.strict {
                lt(&hole.at, probe)
            } else {
                le(&hole.at, probe)
            }
        }

        fn hole_covers(hole: &Hole<'_>, clamped_lo: &Version, _clamped_hi: &Version) -> bool {
            Self::hole_subtracts(hole, clamped_lo)
        }

        fn hole_survives(
            hole: &Hole<'_>,
            _floor: Option<&Version>,
            ceiling: Option<&Version>,
        ) -> bool {
            match ceiling {
                Some(ceiling) => {
                    if hole.strict {
                        lt(&hole.at, ceiling)
                    } else {
                        le(&hole.at, ceiling)
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

        fn hole_subtracts(_hole: &Hole<'_>, _probe: &Version) -> bool {
            unreachable!("a neutral query holds no holes")
        }

        fn hole_covers(_hole: &Hole<'_>, _clamped_lo: &Version, _clamped_hi: &Version) -> bool {
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

/// A query's hole polarity: which complement family it may subtract.
///
/// The three markers — [`Down`], [`Up`], and the hole-free
/// [`Neutral`] — are the whole space (the trait is sealed). The
/// [module docs](super) give the argument the boundary enforces.
///
/// The supertraits let a generic `Query<P>` consumer stay `Send`,
/// `Sync`, and `'static` without restating per-marker facts: every
/// marker is an uninhabited unit enum, so all three hold trivially.
pub trait Polarity: sealed::Sealed + Send + Sync + 'static {}

/// Holes subtract *down-sets*: the [`since`](super::since) family,
/// everything some version already covers.
#[derive(Debug, Clone, Copy)]
pub enum Down {}

/// Holes subtract *up-sets*: the [`until`](super::until) family,
/// everything covering some version.
#[derive(Debug, Clone, Copy)]
pub enum Up {}

/// No holes at all: a pure inclusive interval, conjoinable into
/// either polarity — the [`Query`](super::Query) default.
#[derive(Debug, Clone, Copy)]
pub enum Neutral {}

impl Polarity for Down {}
impl Polarity for Up {}
impl Polarity for Neutral {}
