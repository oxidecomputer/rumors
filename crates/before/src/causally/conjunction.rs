//! Intersection of causal queries.
//!
//! Neutral bounds can intersect either polarity, and queries of the same
//! polarity can intersect each other. Intersecting [`Down`] with [`Up`] is not
//! supported because exact coverage could then require searching combinations
//! of opposing holes.
//!
//! Every supported pairing reduces to [`Query::and`], which merges interval
//! bounds and restores the hole antichain.

use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::BitAnd;

use super::forms::{Ceiling, Floor};
use super::polarity::{Down, Hole, Neutral, Polarity, Up};
use super::{Query, Version};

impl<'a, P: Polarity> Query<'a, P> {
    /// Intersect with a query of the same polarity and restore normal form.
    ///
    /// The merged floor is the join and the merged ceiling is the meet. A hole
    /// is discarded when the merged interval cannot reach it or a hole from
    /// the other operand subtracts a superset.
    ///
    /// Each input already holds an antichain, so only holes from opposite
    /// operands need comparison. A hole outside every realizable version is
    /// inert but remains in the representation; this merge compares bounds and
    /// does not solve query emptiness.
    ///
    /// With `k` left holes, `m` right holes, and `n` total encoded bytes, the
    /// worst-case time is `O(n(k + m + 1))`; output space is `O(n)`.
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
        let mut kept = self.holes;
        kept.retain(survives);
        let mut added = other.holes;
        added.retain(|hole| {
            if !survives(hole) {
                return false;
            }
            if kept.iter().any(|held| P::absorbs(held, hole)) {
                return false;
            }
            kept.retain(|held| !P::absorbs(hole, held));
            true
        });
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
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_conjoin_floors.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n)` in total input bytes; `O(|a| + |b|)`: one fused join walk over the two bounds"
)]
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
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_conjoin_ceilings.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n)` in total input bytes; `O(|a| + |b|)`: one fused meet walk over the two bounds"
)]
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
        #[doc = "Intersection of two causal filters."]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[doc = "With `k` left holes, `m` right holes, and `n` total encoded bytes, worst-case time is `O(n(k + m + 1))`."]
        #[doc = "Output space is `O(n)`. Fixed hole counts are linear in `n`."]
        #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/query_conjoin_bounded_holes.html")))]
        #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(n)` for this fixed one-hole shape")]
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
