//! The `&` algebra and the normal form it maintains.
//!
//! `&` is predicate intersection, total on every pairing the polarity algebra
//! admits: atoms are neutral, neutral conjoins into anything, and each polarity
//! conjoins with itself. The one absent pairing is `Down` with `Up`, because
//! this can create computationally intractible queries. Every impl delegates to
//! the same-polarity merge ([`Query::and`]), which maintains the normal form,
//! so construction order cannot change what a query admits (pinned behaviorally
//! by the conjunction laws in `crate::laws`).

use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::BitAnd;

use super::forms::{Ceiling, Floor};
use super::polarity::{Down, Hole, Neutral, Polarity, Up};
use super::{Query, Version};

impl<'a, P: Polarity> Query<'a, P> {
    /// Conjunction with a query of the same polarity.
    ///
    /// Floors join, ceilings meet, and every hole from either side is
    /// re-admitted against the merged bounds, so absorption and pruning cannot
    /// be evaded by construction order: a hole is dropped when the bound on
    /// its own side already avoids everything it subtracts or a kept hole from
    /// the other side subtracts a superset, and it evicts the other side's
    /// kept holes it covers.
    ///
    /// Each operand's holes are already a pairwise-unabsorbed antichain
    /// (constructors mint at most one hole; every multi-hole query came
    /// through this merge), so same-side pairs are never compared — only the
    /// cross pairs are probed for absorption, in both directions.
    ///
    /// Pruning is *comparative* — against the interval bound and the other
    /// side's holes — never a semantic emptiness judgment: a hole nothing can
    /// fall into rides through inert, subtracting nothing on every path,
    /// rather than minting a corner case here.
    fn and(self, other: Query<'a, P>) -> Query<'a, P> {
        let floor = match (self.floor, other.floor) {
            (None, floor) | (floor, None) => floor,
            (Some(a), Some(b)) => Some(Cow::Owned(Version::join_refs(&a, &b))),
        };
        let ceiling = match (self.ceiling, other.ceiling) {
            (None, ceiling) | (ceiling, None) => ceiling,
            (Some(a), Some(b)) => Some(Cow::Owned(Version::meet_refs(&a, &b))),
        };
        let survives =
            |hole: &Hole<'a>| P::hole_survives(hole, floor.as_deref(), ceiling.as_deref());
        let mut kept: Vec<Hole<'a>> = self.holes.into_iter().filter(survives).collect();
        let mut added: Vec<Hole<'a>> = Vec::new();
        for hole in other.holes {
            if !survives(&hole) {
                continue;
            }
            if kept.iter().any(|held| P::absorbs(held, &hole)) {
                continue;
            }
            kept.retain(|held| !P::absorbs(&hole, held));
            added.push(hole);
        }
        kept.append(&mut added);
        Query {
            floor,
            ceiling,
            holes: kept,
            polarity: PhantomData,
        }
    }
}

/// Elementary conjunction of two floors, staying elementary: the bounds
/// [`join`](Version::join).
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_conjoin_floors.html"))]
impl<'a> BitAnd for Floor<'a> {
    type Output = Floor<'a>;

    fn bitand(self, rhs: Floor<'a>) -> Floor<'a> {
        Floor {
            at: Cow::Owned(Version::join_refs(&self.at, &rhs.at)),
        }
    }
}

/// Elementary conjunction of two ceilings, staying elementary: the bounds
/// [`meet`](Version::meet).
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_conjoin_ceilings.html"))]
impl<'a> BitAnd for Ceiling<'a> {
    type Output = Ceiling<'a>;

    fn bitand(self, rhs: Ceiling<'a>) -> Ceiling<'a> {
        Ceiling {
            at: Cow::Owned(Version::meet_refs(&self.at, &rhs.at)),
        }
    }
}

/// Lifts a `&` operand into a query at the output polarity.
trait Conjoin<'a, P: Polarity> {
    fn lift(self) -> Query<'a, P>;
}

impl<'a, P: Polarity> Conjoin<'a, P> for Floor<'a> {
    fn lift(self) -> Query<'a, P> {
        Query {
            floor: Some(self.at),
            ceiling: None,
            holes: Vec::new(),
            polarity: PhantomData,
        }
    }
}

impl<'a, P: Polarity> Conjoin<'a, P> for Ceiling<'a> {
    fn lift(self) -> Query<'a, P> {
        Query {
            floor: None,
            ceiling: Some(self.at),
            holes: Vec::new(),
            polarity: PhantomData,
        }
    }
}

impl<'a, P: Polarity> Conjoin<'a, P> for Query<'a, P> {
    fn lift(self) -> Query<'a, P> {
        self
    }
}

impl<'a> Conjoin<'a, Down> for Query<'a, Neutral> {
    fn lift(self) -> Query<'a, Down> {
        self.adopt()
    }
}

impl<'a> Conjoin<'a, Up> for Query<'a, Neutral> {
    fn lift(self) -> Query<'a, Up> {
        self.adopt()
    }
}

macro_rules! conjoin {
    ($($lhs:ty, $rhs:ty => $out:ty;)*) => {$(
        #[doc = "Conjunction of [`Query`]s."]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_conjoin_bounded_holes.html"))]
        impl<'a> BitAnd<$rhs> for $lhs {
            type Output = $out;

            fn bitand(self, rhs: $rhs) -> $out {
                Conjoin::<'a, _>::lift(self).and(Conjoin::<'a, _>::lift(rhs))
            }
        }
    )*};
}

conjoin! {
    // Atoms against atoms and neutral queries: hole-free landings.
    Floor<'a>, Ceiling<'a> => Query<'a, Neutral>;
    Ceiling<'a>, Floor<'a> => Query<'a, Neutral>;
    Floor<'a>, Query<'a, Neutral> => Query<'a, Neutral>;
    Query<'a, Neutral>, Floor<'a> => Query<'a, Neutral>;
    Ceiling<'a>, Query<'a, Neutral> => Query<'a, Neutral>;
    Query<'a, Neutral>, Ceiling<'a> => Query<'a, Neutral>;
    Query<'a, Neutral>, Query<'a, Neutral> => Query<'a, Neutral>;
    // Down-polar landings: atoms and neutral queries adopt the
    // polarity of the holed operand.
    Floor<'a>, Query<'a, Down> => Query<'a, Down>;
    Query<'a, Down>, Floor<'a> => Query<'a, Down>;
    Ceiling<'a>, Query<'a, Down> => Query<'a, Down>;
    Query<'a, Down>, Ceiling<'a> => Query<'a, Down>;
    Query<'a, Neutral>, Query<'a, Down> => Query<'a, Down>;
    Query<'a, Down>, Query<'a, Neutral> => Query<'a, Down>;
    Query<'a, Down>, Query<'a, Down> => Query<'a, Down>;
    // Up-polar landings, dually. Down with Up is deliberately
    // absent: the polarity boundary (see the module docs).
    Floor<'a>, Query<'a, Up> => Query<'a, Up>;
    Query<'a, Up>, Floor<'a> => Query<'a, Up>;
    Ceiling<'a>, Query<'a, Up> => Query<'a, Up>;
    Query<'a, Up>, Ceiling<'a> => Query<'a, Up>;
    Query<'a, Neutral>, Query<'a, Up> => Query<'a, Up>;
    Query<'a, Up>, Query<'a, Neutral> => Query<'a, Up>;
    Query<'a, Up>, Query<'a, Up> => Query<'a, Up>;
}
