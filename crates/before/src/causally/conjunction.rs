//! The `&` algebra and the normal form it maintains.
//!
//! `&` is predicate intersection, total on every pairing the polarity
//! algebra admits: atoms are neutral, neutral conjoins into anything,
//! and each polarity conjoins with itself — the one absent pairing is
//! `Down` with `Up`, the module's deliberate boundary. Every impl
//! delegates to the same-polarity merge ([`Query::and`]), which
//! maintains the normal form, so construction order cannot change
//! what a query admits (pinned behaviorally by the conjunction laws
//! in `crate::laws`).

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
    /// re-admitted against the merged bounds and the growing
    /// antichain, so absorption and pruning cannot be evaded by
    /// construction order.
    fn and(self, other: Query<'a, P>) -> Query<'a, P> {
        let floor = match (self.floor, other.floor) {
            (None, floor) | (floor, None) => floor,
            (Some(a), Some(b)) => Some(Cow::Owned(Version::join_refs(&a, &b))),
        };
        let ceiling = match (self.ceiling, other.ceiling) {
            (None, ceiling) | (ceiling, None) => ceiling,
            (Some(a), Some(b)) => Some(Cow::Owned(Version::meet_refs(&a, &b))),
        };
        let mut merged = Query {
            floor,
            ceiling,
            holes: Vec::with_capacity(self.holes.len() + other.holes.len()),
            polarity: PhantomData,
        };
        for hole in self.holes.into_iter().chain(other.holes) {
            merged.push_hole(hole);
        }
        merged
    }

    /// Admit one hole, maintaining the normal form: drop it if the
    /// bound on its own side already avoids everything it subtracts
    /// or an existing hole subtracts a superset; evict existing holes
    /// it covers.
    ///
    /// Pruning is *comparative* — against the interval bound and the
    /// sibling holes — never a semantic emptiness judgment: a hole
    /// nothing can fall into rides through inert, subtracting nothing
    /// on every path, rather than minting a corner case here.
    fn push_hole(&mut self, hole: Hole<'a>) {
        if !P::hole_survives(&hole, self.floor.as_deref(), self.ceiling.as_deref()) {
            return;
        }
        if self.holes.iter().any(|held| P::absorbs(held, &hole)) {
            return;
        }
        self.holes.retain(|held| !P::absorbs(&hole, held));
        self.holes.push(hole);
    }
}

/// Elementary conjunction of two floors, staying elementary: the
/// bounds join (two demands to sit at-or-above collapse to one at
/// their join).
impl<'a> BitAnd for Floor<'a> {
    type Output = Floor<'a>;

    fn bitand(self, rhs: Floor<'a>) -> Floor<'a> {
        Floor {
            at: Cow::Owned(Version::join_refs(&self.at, &rhs.at)),
        }
    }
}

/// Elementary conjunction of two ceilings, staying elementary: the
/// bounds meet — the order-dual of `&` on [`Floor`].
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
        #[doc = "Conjunction: predicate intersection, through the"]
        #[doc = "same-polarity merge's normal form."]
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
