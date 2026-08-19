//! The interval-tree-clock event tree, [`Version`].

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::iter::Sum;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Div};
use core::str::FromStr;
use std::io::{self, Read, Write};

use crate::codec;
use crate::error::{Decode, Parse};
use crate::span::Span;
use crate::Party;

pub(crate) mod hull_traffic;
mod own;
mod rank;
mod ranked;
mod ticks;
// The skyline coding and its operation kernels: the stored representation
// and every algorithm over it. `pub` where the meter feature re-exports it
// so the resource-envelope suite can pin its internals; crate-private
// otherwise — no public item leaks the representation.
#[cfg(any(test, feature = "meter"))]
pub mod skyline;
#[cfg(not(any(test, feature = "meter")))]
pub(crate) mod skyline;

pub use own::OwnVersion;
#[cfg(feature = "borsh")]
pub(crate) use rank::decode_stream as decode_rank_stream;
pub use rank::Rank;
pub use ranked::Ranked;
pub use ticks::{Limbs, Ticks};

#[cfg(test)]
mod tests;

/// A causal version: a timestamp from a [`Party`]'s history.
///
/// Comparison and the lattice operations [`join`](Version::join) (`|`) and
/// [`meet`](Version::meet) (`&`) are what give versions meaning in relation to
/// one another; [`tick`](Version::tick) and [`ticks`](Version::ticks) record
/// *new* history (a join or meet only combines histories already recorded).
///
/// | Operation                                 | Meaning                                                        |
/// |-------------------------------------------|----------------------------------------------------------------|
/// | `a == b`                                  | identical causal history                                       |
/// | `a < b`, `a <= b`                         | `a` is causally dominated by `b`: every event in `a` is in `b` |
/// | [`a.concurrent(b)`](Version::concurrent)  | incomparable: neither dominates the other                      |
/// | `a \| b`, `a \|= b`                       | the *join* (least upper bound): the combined history of both   |
/// | `a & b`, `a &= b`                         | the *meet* (greatest lower bound): the history common to both  |
/// | [`a.tick(&p)`](Version::tick)             | record one new event for [`Party`] `p`                         |
/// | [`a.ticks(&p, k)`](Version::ticks)        | record `k` new events for [`Party`] `p`, in one pass           |
///
/// Comparison is **partial** ([`PartialOrd`], not [`Ord`]): two distinct
/// versions can be [`concurrent`](Version::concurrent), and then `a < b`, `a ==
/// b`, and `a > b` are all false.
///
/// # Complexity
///
/// Ordering is one causal comparison sweep over the two streams; equality
/// is a canonical byte compare (the skyline coding is a unique
/// representation, so byte equality is exactly causal equality):
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_cmp.html"))]
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_eq.html"))]
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// let mut b = a.fork();
/// let va = a.tick();
/// let vb = b.tick();
/// assert!(va.concurrent(vb));  // ticking two forks makes them concurrent
/// let merged = va | vb;
/// assert!(merged > va && merged > vb);  // the join dominates both inputs
/// ```
//
// A `Version` is always represented by its canonical skyline stream
// ([`codec::Bits`]): the raw byte slice IS the wire encoding.
//
// Canonical uniqueness makes byte equality exactly causal equality; `PartialEq`
// is the macro's byte-level stream compare (see `causal_cmp_impls!` and
// `codec::canonical_eq`), and the manual `Hash` below reads the same (raw
// bytes, live length) pair, so their consistency holds by construction. The
// container's backing store is refcounted (`bytes::Bytes`), which is what makes
// the derived `Clone` `O(1)`: a clone shares the buffer, and
// `codec::canonical_eq`'s clone-identity rung recognizes the sharing.
#[derive(Clone, Eq)]
pub struct Version(codec::Bits);

/// Hashes the canonical bytes, consistently with `Eq`'s byte compare.
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_hash.html"))]
impl Hash for Version {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        codec::canonical_hash(&self.0, state);
    }
}

impl Version {
    /// The empty [`Version`], representing no [`tick`](Version::tick)s.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// assert!(before::Version::new().is_empty());
    /// ```
    pub fn new() -> Self {
        // The canonical empty version is exactly the 2-bit stream `11`
        // (the single-leaf topology flag, then gamma(0), the single bit
        // `1` — see `is_empty`), marker-padded to the one static byte
        // `0b1110_0000`: construction allocates nothing, and every empty
        // version shares the one static buffer (clone identity holds
        // even across separate `new()` calls). A `static`, not a
        // `const`: a const's promoted allocation has no guaranteed
        // unique address, and the cross-call sharing claim rests on one.
        // The codec round-trip and text laws pin the constant against
        // the built form.
        static EMPTY_STREAM: &[u8] = &[0b1110_0000];
        Version::from_frozen(codec::Bits::from_canonical(bytes::Bytes::from_static(
            EMPTY_STREAM,
        )))
    }

    /// Whether this version records no events: equal to [`Version::new`].
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// assert!(v.is_empty());
    /// v.tick(&Party::seed());
    /// assert!(!v.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        // The canonical empty version is exactly the 2-bit stream `11`: a `1`
        // leaf flag, then gamma(0), the single bit `1` (see `Version::new` and
        // `codec::encode_int`). The stored skyline stream is a unique
        // representation, so this O(1) bit test is the whole question — no
        // allocation, no walk.
        skyline::is_empty_stream(self.0.live())
    }

    /// Advances this version by one event for `party`.
    ///
    /// Dealing directly with a [`Party`] and a [`Version`] permits one version
    /// to be [`tick`](Version::tick)ed by many parties, or one [`Party`] to
    /// [`tick`](Party::tick) many [`Version`]s; this is in contrast to a
    /// [`Clock`](crate::Clock), which binds the two together.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_tick.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// v.tick(&Party::seed());
    /// assert!(v > Version::new()); // one event: strictly after the empty history
    /// ```
    pub fn tick(&mut self, party: &Party) {
        *self = Version::from_bits(skyline::fill::tick(self.0.live(), party));
    }

    /// Advances this version by `k` events for `party`.
    ///
    /// This is identical to `k` sequential [`tick`](Self::tick)s, but computed
    /// much more efficiently.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_ticks.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Ticks, Version};
    /// let party = Party::seed();
    /// let mut v = Version::new();
    /// v.ticks(&party, 5u64);
    /// let mut w = Version::new();
    /// for _ in 0..5 {
    ///     w.tick(&party);
    /// }
    /// assert_eq!(v, w); // one call, same version as five sequential ticks
    /// // One call skips forward by a count no iteration could reach.
    /// let wide: Ticks = "100000000000000000000000000".parse().unwrap();
    /// v.ticks(&party, wide.clone());
    /// assert_eq!(v.min_ticks(), wide + Ticks::from(5u64));
    /// ```
    pub fn ticks(&mut self, party: &Party, k: impl Into<Ticks>) {
        let k = k.into();
        // The empty run is the identity, settled without re-freezing the stream
        // (a width test, not a value compare: no limb work).
        if k.0.bits() == 0 {
            return;
        }
        *self = Version::from_bits(skyline::fill::ticks(self.0.live(), party, &k.0));
    }

    /// Tests whether two [`Version`]s are concurrent (incomparable).
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_concurrent.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// assert!(va.concurrent(&vb)); // ticks on disjoint parties are concurrent
    /// ```
    pub fn concurrent<V: PartialOrd<Self>>(&self, version: &V) -> bool {
        version.partial_cmp(self).is_none()
    }

    /// The minimum number of [`tick`](Self::tick)s that could have produced
    /// this [`Version`], as an exact [`Ticks`] count at any magnitude.
    ///
    /// This is a floor over all causal histories: every sequence of
    /// [`fork`](crate::Clock::fork), `tick`, and [`join`](crate::Clock::join)
    /// that could have yielded this version must have performed at least this
    /// many ticks. The true history of this [`Version`] could have performed
    /// arbitrarily many more operations than this minimum.
    ///
    /// There is no corresponding maximum. For any nonempty version the tick
    /// count is unbounded above: an increment over an interval can always be
    /// refined into two concurrent increments over its halves (forked, ticked,
    /// rejoined), producing the same version with one more tick having
    /// comprised its true history.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_min_ticks.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Ticks, Version};
    /// assert_eq!(Version::new().min_ticks(), Ticks::ZERO);
    /// let mut p = Party::seed();
    /// let mut v = Version::new();
    /// v.ticks(&p, 5u64);
    /// assert_eq!(v.min_ticks(), Ticks::from(5u64));
    /// // Two events on far-apart shares stay concurrent peaks: the floor
    /// // counts both, though no single path through the version exceeds 1.
    /// let mut q = p.fork();
    /// let _ = p.fork(); // p keeps the leftmost quarter
    /// let r = q.fork(); // r takes the rightmost quarter
    /// let mut peaks = Version::new();
    /// peaks.tick(&p);
    /// peaks.tick(&r);
    /// assert_eq!(peaks.min_ticks(), Ticks::from(2u64));
    /// ```
    pub fn min_ticks(&self) -> Ticks {
        Ticks(skyline::query::min_ticks(self.0.live()))
    }

    /// This [`Version`]'s exact causal [`Rank`]: `v < w` implies `v.rank() <
    /// w.rank()`, so equal ranks are never causally ordered (same version, or
    /// concurrent).
    ///
    /// Sorting by `(rank, some-total-tiebreak)` therefore yields a linear
    /// extension of the causal order: causes always sort before their effects.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_rank.html"))]
    ///
    /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// a.tick();
    /// b.tick();
    /// let va = a.version().clone();
    /// let joined = &va | b.version();
    /// // Ticks grow the rank; the join dominates both sides' ranks.
    /// assert!(va.rank() < joined.rank());
    /// assert!(b.version().rank() < joined.rank());
    /// ```
    pub fn rank(&self) -> Rank {
        skyline::query::rank(self.0.live())
    }

    /// Views this version ordered totally by its causal rank, using its own
    /// lexicographic ordering as a deterministic tie-break for equal ranks.
    ///
    /// Prefer this to [`rank`](Version::rank) when you do not need to
    /// materialize the [`Rank`] itself, and merely need a causal ordering.
    ///
    /// See [`Ranked`] for more detail.
    ///
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Ranked, Version};
    /// let mut p = Party::seed();
    /// let q = p.fork();
    /// let mut half = Version::new();
    /// half.tick(&p); // one share's event
    /// let mut whole = half.clone();
    /// whole.tick(&q); // the other share's event: causally later
    /// // Compare by rank; no Rank is materialized on either side.
    /// assert!(half.ranked() < whole.ranked());
    /// // The method is exactly the borrowing view conversion.
    /// assert!(half.ranked() == Ranked::from(&half));
    /// assert_eq!(half.ranked().version(), &half);
    /// ```
    pub fn ranked(&self) -> Ranked<'_> {
        Ranked::from(self)
    }

    /// The version's shape — its step function over the unit id interval —
    /// as an iterator of [`Plateau`](crate::shape::Plateau)s, left to right.
    ///
    /// One item per maximal constant run: the height change entering it
    /// and the dyadic interval it spans. The walk borrows the version and
    /// streams its stored form in place; the item sequence and the version
    /// determine each other exactly. The [`shape`](crate::shape) module
    /// docs give the vocabulary and show height reconstruction; this is
    /// the entry point for renderers and analysis tooling that draw or
    /// inspect a version rather than compare it.
    ///
    /// # Complexity
    ///
    /// Draining the iterator is linear in the version's encoded size:
    /// each plateau costs `O(1)` plus its own rise's encoded width, and
    /// the walk itself performs no arithmetic.
    ///
    /// Arithmetic *you* do with the rises is priced separately. [`Ticks`]
    /// addition costs the operands' widths, so folding the rises into a
    /// running count — reconstructing heights, summing — can cost each
    /// step the running value's full width, quadratic over the drain in
    /// the worst case (many small rises against a wide running value).
    /// Typical shapes sit far from that bound.
    ///
    /// # Example
    ///
    /// ```
    /// use before::shape::{Plateau, Rise};
    /// use before::{Ticks, Version};
    ///
    /// let version: Version = "(1, 1, (0, 0, 2))".parse().unwrap();
    /// let plateaus: Vec<Plateau> = version.shape().collect();
    /// assert_eq!(
    ///     plateaus,
    ///     vec![
    ///         // The left half at height 2 (the first rise is absolute:
    ///         // the walk enters at height 0)...
    ///         Plateau { rise: Some(Rise::Up(Ticks::from(2u64))), depth: 1 },
    ///         // ...then quarters at heights 1 and 3.
    ///         Plateau { rise: Some(Rise::Down(Ticks::from(1u64))), depth: 2 },
    ///         Plateau { rise: Some(Rise::Up(Ticks::from(2u64))), depth: 2 },
    ///     ],
    /// );
    /// // Widths tile the unit interval: 1/2 + 1/4 + 1/4 = 1.
    /// let total: f64 = plateaus.iter().map(|p| 0.5f64.powi(p.depth as i32)).sum();
    /// assert_eq!(total, 1.0);
    /// ```
    pub fn shape(&self) -> crate::shape::Plateaus<'_> {
        crate::shape::Plateaus::of_version(self)
    }

    /// The *causal distance* between two [`Version`]s.
    ///
    /// This measures how much history two replicas would have to exchange to
    /// converge: zero when they agree, growing with every event neither shares.
    ///
    /// It is equal to the [`Rank`] of their symmetric difference, `(self |
    /// other).rank() - (self & other).rank()`, but more efficiently computed.
    ///
    /// This is a metric on the version lattice. [`Rank`] is a *valuation*: `(a
    /// | b).rank() + (a & b).rank() == a.rank() + b.rank()`. A strictly
    /// monotone valuation on a distributive lattice induces a metric, i.e.
    /// `distance` is symmetric, zero only between equal versions, and obeys the
    /// triangle inequality `a.distance(b) + b.distance(c) >= a.distance(c)`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_distance.html"))]
    ///
    /// Typical inputs run far below the worst case; `M` is the complexity of
    /// unbounded-integer multiplication (about `O(n log n)` in this
    /// implementation).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Rank, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone(); // concurrent to va
    ///
    /// assert_eq!(va.distance(&va), Rank::ZERO);   // identity of indiscernibles
    /// assert_eq!(va.distance(&vb), vb.distance(&va)); // symmetric
    /// // One event on each disjoint half: the join knows both, the meet
    /// // neither. Each event raised half the id interval by one, so the
    /// // area between the versions — their distance — is 2 · ½ = 1.
    /// assert_eq!(va.distance(&vb).to_string(), "1");
    /// ```
    pub fn distance(&self, other: &Version) -> Rank {
        // Equal operands sit at distance zero — the
        // `distance_to_self_is_zero` law in [`laws`](crate::laws) —
        // and canonical equality answers in `O(1)` on a shared buffer
        // (clone identity) or one byte compare, where the fused sweep
        // would fold the whole pair through the accumulator. Unequal
        // operands pay only the compare's early-exiting prefix.
        if codec::canonical_eq(&self.0, &other.0) {
            return Rank::ZERO;
        }
        skyline::query::distance(self.0.live(), other.0.live())
    }

    /// How far `self` lags behind `other`.
    ///
    /// This computes the [`Rank`] of the history `other` records that `self`
    /// does not, `(self | other).rank() - self.rank()`, but more efficiently.
    ///
    /// The directed half of [`distance`](Self::distance): this is zero when
    /// `other <= self` (`self` already knows everything `other` does), and the
    /// two directions sum to the symmetric distance: `a.lag(b) + b.lag(a) ==
    /// a.distance(b)`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_lag.html"))]
    ///
    /// Typical inputs run far below the worst case; `M` is the complexity of
    /// unbounded-integer multiplication (about `O(n log n)` in this
    /// implementation).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Rank, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone(); // concurrent to va
    ///
    /// assert!(va.lag(&va) == Rank::ZERO);     // nothing to learn from yourself
    /// assert!(va.lag(&vb) > Rank::ZERO);      // vb has an event va lacks
    /// assert_eq!(va.lag(&vb) + vb.lag(&va), va.distance(&vb)); // halves sum
    /// ```
    pub fn lag(&self, other: &Version) -> Rank {
        // A version lags itself by zero — the `lag_to_self_is_zero` law
        // in [`laws`](crate::laws) — with the same `O(1)`-on-clone
        // equality rung as [`distance`](Self::distance).
        if codec::canonical_eq(&self.0, &other.0) {
            return Rank::ZERO;
        }
        skyline::query::lag(self.0.live(), other.0.live())
    }

    /// The join (least upper bound) of this [`Version`] and `other`: their
    /// combined causal history.
    ///
    /// Identical to the operator form `self | other`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let merged = va.join(&vb);
    /// assert_eq!(merged, &va | &vb); // the operator spelling agrees
    /// assert!(merged >= va && merged >= vb);
    /// ```
    pub fn join(&self, other: &Version) -> Version {
        Self::join_refs(self, other)
    }

    /// The [`join`](Version::join) of `self` and every version in `iter`.
    ///
    /// Prefer this to iteratively [`join`](Version::join)ing [`Version`]s
    /// one-at-a-time, as it is more efficient.
    ///
    /// To join an iterator with no distinguished first element,
    /// [`sum`](Iterator::sum) or [`collect`](Iterator::collect) it.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html"))]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let all = va.join_all([&vb]); // by reference: both stay usable
    /// assert!(all >= va && all >= vb);
    /// assert_eq!(va.join_all(Vec::<Version>::new()), va); // nothing to add
    /// ```
    pub fn join_all<I>(&self, iter: I) -> Version
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        Self::balanced_fold(self.with_items(iter), Self::join_refs, Self::join_view)
            .expect("the fold is seeded with the receiver: never empty")
    }

    /// The meet (greatest lower bound) of this version and `other`: the
    /// history the two share.
    ///
    /// Identical to the operator form `self & other`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_meet.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let common = va.meet(&vb);
    /// assert_eq!(common, &va & &vb); // the operator spelling agrees
    /// assert!(common <= va && common <= vb);
    /// ```
    pub fn meet(&self, other: &Version) -> Version {
        Self::meet_refs(self, other)
    }

    /// The [`meet`](Version::meet) (greatest lower bound) of this version and
    /// every version in `iter`; for an empty iterator, a clone of `self`.
    ///
    /// Prefer this to iteratively [`meet`](Version::meet)ing [`Version`]s
    /// one-at-a-time, as it is more efficient.
    ///
    /// Unlike the join, the meet has no iterator-only form (no `Sum`
    /// counterpart): the empty meet would be the version dominating all
    /// others, but no such [`Version`] exists, since every version can
    /// [`tick`](Self::tick) higher without bound. The receiver is the
    /// guaranteed first operand that keeps the fold total.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_meet_all.html"))]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let common = va.meet_all([&vb]); // by reference: both stay usable
    /// assert!(common <= va && common <= vb);
    /// assert_eq!(va.meet_all(Vec::<Version>::new()), va); // nothing to share
    /// ```
    pub fn meet_all<I>(&self, iter: I) -> Version
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        Self::balanced_fold(self.with_items(iter), Self::meet_refs, Self::meet_view)
            .expect("the fold is seeded with the receiver: never empty")
    }

    /// The causal [`Span`] from this [`Version`] to `other`.
    ///
    /// This computes the tightest [`Span`] which encloses all [`Version`]s `v`
    /// such that `self & other <= v <= self | other`.
    ///
    /// Identical to the operator form `self ^ other`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_span.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Placement, Span};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let va2 = a.tick().clone();
    /// let vb = b.tick().clone(); // concurrent to alice's line
    ///
    /// // Comparable versions:
    /// assert_eq!(va2.span(&va), Span::new(&va, &va2).unwrap());
    /// assert_eq!(va.span(&va2), Span::new(&va, &va2).unwrap());
    /// // A concurrent pair has no reordering to repair, but it has a
    /// // span; both inputs sit strictly inside it:
    /// let span = va.span(&vb);
    /// assert_eq!(span.place(&va), Placement::Between);
    /// assert_eq!(span.place(&vb), Placement::Between);
    /// assert_eq!(&va ^ &vb, span); // the operator spelling agrees
    /// ```
    pub fn span(&self, other: &Version) -> Span<'static> {
        let (lo, hi) = Self::span_refs(self, other);
        Span::owned(lo, hi)
    }

    /// The causal [`Span`] enclosing `self` and all the [`Version`]s in `iter`.
    ///
    /// This computes the tightest [`Span`] which encloses all [`Version`]s `v`
    /// such that `self.meet_all(iter) <= v <= self.join_all(iter)`.
    ///
    /// Prefer this to iteratively [`span`](Version::meet)ing [`Version`]s
    /// one-at-a-time, as it is more efficient.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_span_all.html"))]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Version, Placement, Span};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let va2 = a.tick().clone();
    ///
    /// // The span of a collection: every input places within it.
    /// let span = va.span_all([&vb, &va2]);
    /// for v in [&va, &vb, &va2] {
    ///     assert!(!matches!(span.place(v), Placement::Before | Placement::After));
    /// }
    /// // The empty iterator is the coincident single-version span:
    /// assert_eq!(
    ///     va.span_all(Vec::<Version>::new()),
    ///     Span::new(&va, &va).unwrap(),
    /// );
    /// ```
    pub fn span_all<I>(&self, iter: I) -> Span<'static>
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        // One balanced fold, two-sided accumulator: the hull needs both lattice
        // directions, and carrying `(lo, hi)` through the counter feeds them
        // from a single pass — the caller's iterator is never buffered, and
        // each input is read at its combines alone.
        //
        // A leaf combine (two raw inputs) derives the pair hull through the
        // fused pair walk, each input decoded once for both endpoints; an
        // interior combine folds per side, because its two legs read
        // *different* operand pairs (`lo₁ ∧ lo₂` and `hi₁ ∨ hi₂`) — no shared
        // pair walk exists there to fuse.
        //
        // Adjacent clone-identical inputs (the receiver included) collapse
        // before the counter reads them ([`DedupRuns`]): both hull directions
        // are idempotent, so a run of one shared buffer is one input.
        let inputs = DedupRuns::new(self.with_items(iter), FoldInput::version).map(Hull::Input);
        let group = crate::fold::balanced_reduce(inputs, |a, b| {
            let (lo, hi) = match (a, b) {
                // A leaf combine: two raw inputs derive their pair hull in one
                // fused walk.
                (Hull::Input(a), Hull::Input(b)) => Self::span_refs(a.version(), b.version()),
                (Hull::Merged { mut lo, mut hi }, Hull::Input(b)) => {
                    let b = b.version();
                    lo.meet_view(b.view());
                    hi.join_view(b.view());
                    (lo, hi)
                }
                (
                    Hull::Merged {
                        lo: mut a_lo,
                        hi: mut a_hi,
                    },
                    Hull::Merged { lo: b_lo, hi: b_hi },
                ) => {
                    a_lo.meet_view(b_lo.view());
                    a_hi.join_view(b_hi.view());
                    (a_lo, a_hi)
                }
                // Unreachable through the counter's weight discipline (a
                // weight-0 lone input never sits below a merged group in the
                // closing drain), but the match stays total rather than
                // asserting: both sides' combiners are commutative, so folding
                // the raw input into the owned hull is value-identical.
                (Hull::Input(a), Hull::Merged { mut lo, mut hi }) => {
                    let a = a.version();
                    lo.meet_view(a.view());
                    hi.join_view(a.view());
                    (lo, hi)
                }
            };
            Hull::Merged { lo, hi }
        });
        match group.expect("the fold is seeded with the receiver: never empty") {
            // The receiver alone (an empty iterator): the coincident span, the
            // one place an input itself becomes the hull.
            Hull::Input(input) => {
                let v = input.version();
                Span::owned(v.clone(), v.clone())
            }
            Hull::Merged { lo, hi } => Span::owned(lo, hi),
        }
    }

    /// The part of this [`Version`] wholly owned by a [`Party`], as a
    /// lazy [`OwnVersion`] view.
    ///
    /// Identical to the operator form `&self / party`.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Clock;
    /// // Two disjoint halves each tick, then learn each other's history.
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// a.tick();
    /// b.tick();
    /// a.sync(&mut b).unwrap();
    /// let v = a.version().clone();
    /// // The named and operator spellings agree...
    /// assert_eq!(v.project(a.party()), &v / a.party());
    /// // ...and each half's contribution is a sub-version.
    /// assert!(v.project(a.party()) <= v && v.project(b.party()) <= v);
    /// ```
    pub fn project<'a>(&'a self, party: &'a Party) -> OwnVersion<'a> {
        self / party
    }

    /// The balanced reduction under [`join_all`](Self::join_all) and
    /// [`meet_all`](Self::meet_all) (which seed it with their receiver) and
    /// the `Sum`/`FromIterator` impls (which feed it the bare iterator).
    ///
    /// Folds items that [borrow](Borrow) as [`Version`] through
    /// [`crate::fold::balanced_reduce`] without cloning them on entry.
    ///
    /// Each combiner comes in the two forms ownership demands — `refs` combines
    /// two borrowed inputs into a fresh owned result
    /// ([`join_refs`](Self::join_refs)/[`meet_refs`](Self::meet_refs)), and
    /// `view` folds a borrowed stream into an owned group in place
    /// ([`join_view`](Self::join_view)/[`meet_view`](Self::meet_view)) — and
    /// [`Group`] carries which form each operand needs. `None` is the empty
    /// fold, which a receiver-seeded caller never sees; a lone input is cloned,
    /// the one place an input itself must become an owned result.
    ///
    /// Adjacent clone-identical inputs collapse before the counter reads them
    /// ([`DedupRuns`], citing the idempotence laws): both combiners are
    /// idempotent, so a run of one shared buffer is one operand.
    fn balanced_fold<I>(
        iter: I,
        refs: fn(&Version, &Version) -> Version,
        view: fn(&mut Version, &codec::Bits),
    ) -> Option<Version>
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        let inputs = DedupRuns::new(iter.into_iter(), Borrow::borrow);
        let group = crate::fold::balanced_reduce(inputs.map(Group::Input), |a, b| {
            Group::Merged(match (a, b) {
                (Group::Input(a), Group::Input(b)) => refs(a.borrow(), b.borrow()),
                (Group::Merged(mut a), Group::Input(b)) => {
                    view(&mut a, b.borrow().view());
                    a
                }
                (Group::Merged(mut a), Group::Merged(b)) => {
                    view(&mut a, b.view());
                    a
                }
                // Unreachable through the counter's weight discipline (a
                // weight-0 lone input never sits below a merged group in the
                // closing drain), but the match stays total rather than
                // asserting: both combiners are commutative, so folding the
                // borrowed side into the owned side is value-identical.
                (Group::Input(a), Group::Merged(mut b)) => {
                    view(&mut b, a.borrow().view());
                    b
                }
            })
        })?;
        Some(match group {
            Group::Input(input) => input.borrow().clone(),
            Group::Merged(version) => version,
        })
    }

    /// This version and then the caller's items: the never-empty input stream
    /// every receiver-seeded fold ([`join_all`](Self::join_all),
    /// [`meet_all`](Self::meet_all), [`span_all`](Self::span_all)) reads.
    fn with_items<I>(&self, iter: I) -> impl Iterator<Item = FoldInput<'_, I::Item>>
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        core::iter::once(FoldInput::Receiver(self)).chain(iter.into_iter().map(FoldInput::Item))
    }

    /// A read-only view of this version's stored skyline stream.
    pub(crate) fn view(&self) -> &codec::Bits {
        &self.0
    }

    /// The view-taking join core: fold an arbitrary skyline stream into this
    /// version in place.
    ///
    /// Every `|`/`|=` cell routes through here, so owned and borrowed operands
    /// join without transcoding.
    ///
    /// Before the merge sweep, two `O(1)` short-circuits settle the cases
    /// canonical form makes immediate: trivial equality (`a ∨ a = a`, a no-op,
    /// decided by a byte compare of the two unique streams) and the lattice
    /// identity `0 ∨ v = v` — an empty incoming leaves the current tree
    /// untouched, and an empty current adopts the incoming stream wholesale (a
    /// copy, byte-identical to what the merge would emit). The identity path is
    /// the common seed pattern: folds seeded with [`Version::new`] (the `|=`
    /// accumulation shape) hit it on their first join.
    pub(crate) fn join_view(&mut self, incoming: &codec::Bits) {
        if codec::canonical_eq(&self.0, incoming) {
            return; // a ∨ a = a
        }
        if skyline::is_empty_stream(incoming.live()) {
            return; // v ∨ 0 = v: nothing to fold in
        }
        if skyline::is_empty_stream(self.0.live()) {
            // 0 ∨ v = v: adopt the incoming stream wholesale. Both streams are
            // canonical, so the shared buffer (an `O(1)` refcount clone) equals
            // the merge byte for byte.
            *self = Version::from_frozen(incoming.clone());
            return;
        }
        *self = Version::from_bits(skyline::emit::join(self.0.live(), incoming.live()));
    }

    /// The borrowed-operands join: `a ∨ b` as a fresh [`Version`], reading both
    /// operands in place.
    ///
    /// [`join_view`](Self::join_view) for the case where neither operand is
    /// owned — the same short-circuits in the same order (keep the two in
    /// lockstep), with each hand-back arm cloning the operand that is itself
    /// the answer. The general path emits the merged stream straight from the
    /// two views, so borrowing costs no clone and no transcoding.
    pub(crate) fn join_refs(a: &Version, b: &Version) -> Version {
        if codec::canonical_eq(&a.0, &b.0) {
            return a.clone(); // a ∨ a = a
        }
        if skyline::is_empty_stream(b.0.live()) {
            return a.clone(); // v ∨ 0 = v
        }
        if skyline::is_empty_stream(a.0.live()) {
            return b.clone(); // 0 ∨ v = v
        }
        Version::from_bits(skyline::emit::join(a.0.live(), b.0.live()))
    }

    /// The view-taking meet core, the dual of [`join_view`](Self::join_view):
    /// meet an arbitrary skyline stream into this version in place.
    ///
    /// The `&`/`&=` matrix routes through here just as the `|`/`|=` matrix
    /// routes through `join_view`.
    ///
    /// The dual short-circuits apply: trivial equality (`a ∧ a = a`), and the
    /// empty version as the *absorbing* element, `0 ∧ v = 0` — an empty current
    /// is already the answer, and an empty incoming makes the result the empty
    /// version outright, no merge sweep either way.
    pub(crate) fn meet_view(&mut self, incoming: &codec::Bits) {
        if codec::canonical_eq(&self.0, incoming) {
            return; // a ∧ a == a
        }
        if skyline::is_empty_stream(self.0.live()) {
            return; // 0 ∧ v = 0: already empty, nothing can shrink it
        }
        if skyline::is_empty_stream(incoming.live()) {
            // v ∧ 0 = 0: the result is the empty version, whatever `v` was.
            *self = Version::new();
            return;
        }
        *self = Version::from_bits(skyline::emit::meet(self.0.live(), incoming.live()));
    }

    /// The borrowed-operands meet: `a ∧ b` as a fresh [`Version`], reading both
    /// operands in place.
    ///
    /// [`meet_view`](Self::meet_view) for the case where neither operand is
    /// owned, exactly as [`join_refs`](Self::join_refs) mirrors
    /// [`join_view`](Self::join_view): the same short-circuits in the same
    /// order — keep the two in lockstep.
    pub(crate) fn meet_refs(a: &Version, b: &Version) -> Version {
        if codec::canonical_eq(&a.0, &b.0) {
            return a.clone(); // a ∧ a = a
        }
        if skyline::is_empty_stream(a.0.live()) {
            return a.clone(); // 0 ∧ v = 0: `a` is already the answer
        }
        if skyline::is_empty_stream(b.0.live()) {
            return Version::new(); // v ∧ 0 = 0, whatever `v` was
        }
        Version::from_bits(skyline::emit::meet(a.0.live(), b.0.live()))
    }

    /// The borrowed-operands hull: `(a ∧ b, a ∨ b)` as fresh [`Version`]s,
    /// emitting only when it must.
    ///
    /// The first three rungs are [`meet_refs`](Self::meet_refs) and
    /// [`join_refs`](Self::join_refs)'s in the same order — keep the three in
    /// lockstep — each settling both endpoints at once, and the equal rung's
    /// two clones share one buffer (the coincident hull stores its stream
    /// once).
    ///
    /// The span ladder then adds the comparable rung the pair operations don't
    /// have: a comparable pair's hull IS the pair, reordered (the
    /// `span_is_the_pair_hull` law in [`laws`](crate::laws) — the meet and join
    /// of comparable versions are the smaller and the larger), so one
    /// comparison sweep hands the operands back as the endpoints, `O(1)`
    /// clones, zero emission.
    ///
    /// Only a concurrent pair reaches the fused emission walk
    /// (`skyline::emit::hull`, one pair walk feeding both output builders where
    /// composing the two emitters would decode each operand twice); the
    /// comparison it paid first is the sweep's early-exiting prefix, which
    /// stops at the second refuting interval.
    pub(crate) fn span_refs(a: &Version, b: &Version) -> (Version, Version) {
        use hull_traffic::Rung;
        if codec::canonical_eq(&a.0, &b.0) {
            hull_traffic::record(Rung::Equal);
            return (a.clone(), a.clone()); // a ∧ a = a = a ∨ a
        }
        if skyline::is_empty_stream(a.0.live()) {
            // 0 ∧ v = 0 (`a` is already the meet), 0 ∨ v = v.
            hull_traffic::record(Rung::Empty);
            return (a.clone(), b.clone());
        }
        if skyline::is_empty_stream(b.0.live()) {
            // v ∧ 0 = 0, v ∨ 0 = v.
            hull_traffic::record(Rung::Empty);
            return (Version::new(), a.clone());
        }
        match skyline::sweep::causal_cmp(a.0.live(), b.0.live()) {
            // The comparable case's answer IS an operand pair.
            Some(Ordering::Less) => {
                hull_traffic::record(Rung::Comparable);
                return (a.clone(), b.clone());
            }
            Some(Ordering::Greater) => {
                hull_traffic::record(Rung::Comparable);
                return (b.clone(), a.clone());
            }
            Some(Ordering::Equal) => unreachable!(
                "equal versions have byte-equal canonical streams, settled by the first rung"
            ),
            None => {}
        }
        hull_traffic::record(Rung::Concurrent);
        let hull = skyline::emit::hull(a.0.live(), b.0.live());
        // The fused walk folds the pair relation beside its emissions (an O(1)
        // flag pair riding sign reads the walk performs anyway), so the
        // ladder's classification is cross-checked at the only door that emits.
        debug_assert!(
            hull.relation.is_none(),
            "the comparison rung admits only concurrent pairs to the emitting walk"
        );
        (Version::from_bits(hull.lo), Version::from_bits(hull.hi))
    }

    /// Encodes this [`Version`] to bytes.
    ///
    /// Prefer [`as_bytes`](Version::as_bytes) to get a reference to the
    /// underlying encoding without cloning it.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_encode.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Version;
    /// let v = Version::new();
    /// assert_eq!(Version::decode(&v.encode()[..]).unwrap(), v);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// Encodes this [`Version`] to an arbitrary writer.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_encode.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Version;
    /// let mut buf = Vec::new();
    /// Version::new().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Version::new().encode());
    /// ```
    pub fn encode_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.as_bytes())
    }

    /// Encodes this [`Version`]'s [`Rank`] to bytes.
    ///
    /// Equivalent to `self.rank().encode()`, but more efficient. Exactly
    /// equivalent to `self.ranked().encode_rank()`, but more succinct.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_encode_rank.html"))]
    ///
    /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Rank, Version};
    /// let v = Version::new();
    /// assert_eq!(Rank::decode(&v.encode_rank()[..]).unwrap(), v.rank());
    /// ```
    pub fn encode_rank(&self) -> Vec<u8> {
        self.ranked().encode_rank()
    }

    /// Encodes this [`Version`]'s [`Rank`] to an arbitrary writer.
    ///
    /// Equivalent to `self.rank().encode_to(writer)`, but more efficient.
    /// Exactly equivalent to `self.ranked().encode_rank_to(writer)`, but more
    /// succinct.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_encode_rank.html"))]
    ///
    /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).
    ///
    /// # Example
    ///
    /// ```
    /// use before::Version;
    /// let mut buf = Vec::new();
    /// Version::new().encode_rank_to(&mut buf).unwrap();
    /// assert_eq!(buf, Version::new().rank().encode());
    /// ```
    pub fn encode_rank_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.ranked().encode_rank_to(writer)
    }

    /// Decodes a [`Version`] from a reader of canonical bytes.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_decode.html"))]
    ///
    /// Strict validation is one pass over the stream, and the result reuses the read buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Version;
    /// let bytes = Version::new().encode();
    /// assert_eq!(Version::decode(&bytes[..]).unwrap(), Version::new());
    /// ```
    pub fn decode<R: Read>(mut reader: R) -> Result<Self, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        // Validate over the whole buffer as bits, padding included: the
        // walk's input is the whole `8 · buf.len()`-bit view, and the marker
        // check judges the remainder.
        {
            let end = skyline::validate_prefix(codec::BitsView::whole(&buf))?;
            codec::require_marker_padding(&buf, end)?;
        }
        // Adopt the read buffer as the result's backing store without
        // copying: the padding check proved the buffer is the stream's one
        // marker-padded spelling — the canonical form the at-rest
        // container stores.
        Ok(Version::from_frozen(codec::Bits::from_canonical(
            buf.into(),
        )))
    }

    /// The exact length in bits of [`encode`](Self::encode) before its
    /// padding — the marker bit and zero-pad to the byte boundary, so
    /// `encode().len()` is `(encoded_bits() + 1).div_ceil(8)`.
    ///
    /// Instrument surface, public under the `meter` feature: the resource
    /// meters, coverage suites, and boundary pins denominate readings in
    /// exact encoded bit lengths. Applications measure wire cost as
    /// `encode().len()` or [`as_bytes`](Self::as_bytes)`.len()` — the byte
    /// length actually shipped.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Version;
    /// // The empty version is a single `0` leaf: a flag bit plus a value bit.
    /// assert_eq!(Version::new().encoded_bits(), 2);
    /// ```
    #[cfg(any(test, feature = "meter"))]
    pub fn encoded_bits(&self) -> u64 {
        self.0.len()
    }

    /// The canonical bytes of this [`Version`], borrowed.
    ///
    /// Their lexicographic order is an arbitrary total order with no causal
    /// meaning; use it only as a deterministic tiebreak between distinct
    /// versions. For causal comparison, use [`PartialOrd`] (`<=`) or
    /// [`concurrent`](Self::concurrent).
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// v.ticks(&Party::seed(), 5u64);
    /// assert_eq!(v.as_bytes(), v.encode().as_slice());
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        debug_assert!(
            codec::padding_is_canonical(&self.0),
            "non-canonical Version storage: the bytes must end in the `1 0*` padding",
        );
        self.0.as_raw_slice()
    }

    /// The stored skyline stream, borrowed as live bits.
    ///
    /// Test- and meter-only: the meter surface's `skyline::encode` and the
    /// differential bridges read it; production code goes through
    /// [`Self::as_bytes`] or the crate-internal `view`.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn as_bits(&self) -> codec::BitsView<'_> {
        self.0.live()
    }

    /// Freeze a normal-form skyline bit stream as a `Version`, canonicalizing
    /// its storage. The single build-side gate every built/parsed `Version`
    /// passes through.
    ///
    /// Callers guarantee canonical skyline form; the freeze seals the
    /// marker padding so the stored bytes are canonical (see
    /// [`codec::Bits::freeze`]).
    pub(crate) fn from_bits(bits: codec::BitsBuf) -> Self {
        Version(codec::Bits::freeze(bits))
    }

    /// Adopt an already-frozen canonical skyline stream as a `Version`: the
    /// decode-side gate, dual to the build-side [`from_bits`](Self::from_bits).
    ///
    /// Callers guarantee the stream is canonical skyline form in canonical
    /// storage — what a validated decode slice or another version's stored
    /// stream already is — so no re-canonicalization runs and adoption is
    /// `O(1)`.
    pub(crate) fn from_frozen(bits: codec::Bits) -> Self {
        Version(bits)
    }
}

/// An iterator adapter collapsing adjacent runs of one shared stored buffer
/// before a lattice fold reads them.
///
/// A run of clones is one operand under idempotence — the `merge_idempotent`
/// and `meet_idempotent` laws in [`laws`](crate::laws) (both at once for the
/// hull fold, whose accumulator carries one endpoint per direction) — and clone
/// identity (`codec::Bits::ptr_eq`, through the items' views) certifies the
/// duplication in `O(1)` without reading either stream. Only *adjacent*
/// duplicates collapse: the window is one item, so the collapse costs `O(1)`
/// state and the fold stays single-pass; scattered duplicates still fold —
/// correctly, at the combine's own equality rung.
///
/// `last` holds a **clone** of the last yielded item's version, not a raw
/// address: the clone keeps the run's buffer alive, so no freed allocation can
/// be reused at the same address mid-iteration and masquerade as a duplicate.
struct DedupRuns<I, F> {
    inner: I,
    /// Projects each item to the version it contributes.
    view: F,
    /// A clone of the last yielded item's version (see above).
    last: Option<Version>,
}

impl<I, F> DedupRuns<I, F> {
    fn new(inner: I, view: F) -> Self {
        DedupRuns {
            inner,
            view,
            last: None,
        }
    }
}

impl<I, F> Iterator for DedupRuns<I, F>
where
    I: Iterator,
    F: for<'a> Fn(&'a I::Item) -> &'a Version,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        loop {
            let item = self.inner.next()?;
            let version = (self.view)(&item);
            if self
                .last
                .as_ref()
                .is_some_and(|prev| prev.0.ptr_eq(&version.0))
            {
                continue; // an adjacent clone: idempotence drops it
            }
            self.last = Some(version.clone());
            return Some(item);
        }
    }
}

/// One group in [`Version::balanced_fold`]'s counter: an input exactly as the
/// caller supplied it (owned or borrowed through [`Borrow`], never cloned on
/// entry), or the owned version a combine produced.
///
/// Weight-0 counter entries are always lone [`Input`](Group::Input)s and every
/// combine's output is [`Merged`](Group::Merged), so the distinction lets each
/// combine pick the operand form its combiner needs — in place for borrowed
/// inputs, by view-fold for owned groups.
enum Group<B> {
    /// An input the fold has not yet combined, still in the caller's form.
    Input(B),
    /// The owned running result of one or more combines.
    Merged(Version),
}

/// One raw input to a receiver-seeded fold ([`Version::join_all`],
/// [`Version::meet_all`], [`Version::span_all`]).
///
/// The receiver rides by borrow, the caller's items in their own form
/// (owned or borrowed through [`Borrow`], never cloned on entry).
enum FoldInput<'r, B> {
    /// The receiver: the guaranteed first element that keeps the fold
    /// total.
    Receiver(&'r Version),
    /// One of the caller's items.
    Item(B),
}

impl<B: Borrow<Version>> FoldInput<'_, B> {
    /// The borrowed version this input contributes.
    fn version(&self) -> &Version {
        match self {
            FoldInput::Receiver(v) => v,
            FoldInput::Item(b) => b.borrow(),
        }
    }
}

/// Lends the contributed version, so [`Version::balanced_fold`] reads a
/// receiver-seeded input stream exactly as it reads a bare one.
impl<B: Borrow<Version>> Borrow<Version> for FoldInput<'_, B> {
    fn borrow(&self) -> &Version {
        self.version()
    }
}

/// One group in [`Version::span_all`]'s counter: [`Group`]'s shape with the
/// two-sided hull accumulator, so one balanced fold carries both lattice
/// directions.
enum Hull<'r, B> {
    /// An input the fold has not yet combined.
    Input(FoldInput<'r, B>),
    /// The owned running hull of one or more combines.
    Merged {
        /// The running meet of every input combined so far.
        lo: Version,
        /// The running join of every input combined so far.
        hi: Version,
    },
}

/// The empty [`Version`] (same as [`Version::new`]).
///
/// # Example
///
/// ```
/// assert_eq!(before::Version::default(), before::Version::new());
/// ```
impl Default for Version {
    fn default() -> Self {
        Self::new()
    }
}

// `Version` under `|` is a commutative idempotent monoid with identity
// [`Version::new`], so it folds from an iterator both ways std offers: `.sum()`
// over an `Iterator` and `.collect()` into a `Version`. Both run the balanced
// reduction behind [`join_all`](Version::join_all) with no receiver to seed it
// (the empty case is the empty version), taking the borrowed forms' references
// as they come. There is deliberately no meet counterpart here — the meet has
// no identity to give an empty iterator, so its only fold is the
// receiver-seeded [`Version::meet_all`].

/// Joins the iterator's versions; the empty sum is [`Version::new`].
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html"))]
///
/// Auxiliary space is `O(|iter|)`.
impl Sum<Version> for Version {
    fn sum<I: Iterator<Item = Version>>(iter: I) -> Version {
        Version::balanced_fold(iter, Version::join_refs, Version::join_view).unwrap_or_default()
    }
}

/// Joins the iterator's versions; the empty sum is [`Version::new`].
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html"))]
///
/// Auxiliary space is `O(|iter|)`.
impl<'a> Sum<&'a Version> for Version {
    fn sum<I: Iterator<Item = &'a Version>>(iter: I) -> Version {
        Version::balanced_fold(iter, Version::join_refs, Version::join_view).unwrap_or_default()
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html"))]
///
/// Auxiliary space is `O(|iter|)`.
impl FromIterator<Version> for Version {
    fn from_iter<I: IntoIterator<Item = Version>>(iter: I) -> Version {
        iter.into_iter().sum()
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html"))]
///
/// Auxiliary space is `O(|iter|)`.
impl<'a> FromIterator<&'a Version> for Version {
    fn from_iter<I: IntoIterator<Item = &'a Version>>(iter: I) -> Version {
        iter.into_iter().sum()
    }
}

/// Paper notation: `n` leaves, `(n, e1, e2)` nodes. E.g. `(1, 2, (0, (1, 0, 2),
/// 0))`.
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_display.html"))]
///
/// # Example
///
/// ```
/// use before::Version;
/// let v: Version = "(1, 2, (0, (1, 0, 2), 0))".parse().unwrap();
/// assert_eq!(v.to_string(), "(1, 2, (0, (1, 0, 2), 0))");
/// ```
impl Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&skyline::text::render(self.0.live()))
    }
}

/// The same format as `Display`.
///
/// # Example
///
/// ```
/// assert_eq!(format!("{:?}", before::Version::new()), "0");
/// ```
impl Debug for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

/// Parses paper notation (`n` or `(n, e1, e2)`), strictly rejecting
/// non-normal-form input.
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_fromstr.html"))]
///
/// # Example
///
/// ```
/// use before::Version;
/// let v: Version = "(1, 0, 1)".parse().unwrap();
/// assert_eq!(v.to_string(), "(1, 0, 1)");
/// ```
impl FromStr for Version {
    type Err = Parse;
    fn from_str(s: &str) -> Result<Self, Parse> {
        Ok(Version::from_bits(skyline::text::parse(s)?))
    }
}

/// An event leaf from its base value, e.g. `Version::try_from(3u64)`.
///
/// # Complexity
///
/// `O(1)`.
///
/// # Example
///
/// ```
/// use before::Version;
/// assert_eq!(Version::try_from(3).unwrap().to_string(), "3");
/// ```
impl TryFrom<u64> for Version {
    type Error = Parse;
    fn try_from(n: u64) -> Result<Self, Parse> {
        Ok(Version::from_bits(skyline::literal::leaf(n)))
    }
}

/// A [`Version`] from an `(n, left, right)` literal, e.g.
/// `Version::try_from((1u64, 0u64, (2u64, 0u64, 1u64)))`.
///
/// Rejects non-normal-form nodes (no zero-base child, or a collapsible `(n, m,
/// m)`).
///
/// # Complexity
///
/// `O(m)`, with `m` the built version's size in bytes.
///
/// # Example
///
/// ```
/// use before::Version;
/// let v = Version::try_from((1, 0, 1)).unwrap();
/// assert_eq!(v.to_string(), "(1, 0, 1)");
/// ```
impl<T, S> TryFrom<(u64, T, S)> for Version
where
    Version: TryFrom<T, Error = Parse> + TryFrom<S, Error = Parse>,
{
    type Error = Parse;
    fn try_from((n, l, r): (u64, T, S)) -> Result<Self, Parse> {
        let l = Version::try_from(l)?;
        let r = Version::try_from(r)?;
        Ok(Version::from_bits(skyline::literal::node(
            n,
            l.0.live(),
            r.0.live(),
        )?))
    }
}

// The join (`|`, `|=`) and meet (`&`, `&=`) matrices over owned and borrowed
// `Version` operands, duals of each other, mirroring the comparison matrix
// below. The `binop_matrix!` macro generates every cell of both families: four
// value-operator cells (lhs × rhs over {Version, &Version}) and two assign
// cells (rhs over {Version, &Version}).
//
// A value-operator cell turns its left operand into a fresh owned `Version`
// (`own` moves an owned `Version`, `clone` copies a borrowed one), then folds
// the right operand's view into it. An assign cell folds the right operand's
// view into the receiver in place. The two families differ only in the
// view-folding method each cell routes through: `Version::join_view` for join,
// `Version::meet_view` for meet.

/// Generates one binary-operator family's full matrix over owned and borrowed
/// `Version` operands.
///
/// Parameterized over the value operator `$Op::$op` (e.g. `BitOr::bitor`), its
/// assigning form `$Assign::$assign` (e.g. `BitOrAssign::bitor_assign`), and
/// the view-folding method `$view` every cell routes through (`join_view` or
/// `meet_view`). Each strategy — `own`/`clone` for value cells, `assign` for
/// assign cells — has its own `@cell` arm so the receiver `self` is written in
/// the same expansion as the method it belongs to (`self` cannot cross a
/// macro-invocation boundary).
macro_rules! binop_matrix {
    ($island:literal, $opdoc:literal, $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident;
     $($lhs:ty, $rhs:ty, $strat:tt);* $(;)?
    ) => {
        $( binop_matrix!(@cell $island, $opdoc, $Op::$op, $Assign::$assign, $view, $lhs, $rhs, $strat); )*
    };
    (@cell $island:literal, $opdoc:literal, $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, own) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self;
                out.$view(r.view());
                out
            }
        }
    };
    (@cell $island:literal, $opdoc:literal, $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, clone) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self.clone();
                out.$view(r.view());
                out
            }
        }
    };
    (@cell $island:literal, $opdoc:literal, $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, assign) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]
        impl $Assign<$rhs> for $lhs {
            fn $assign(&mut self, r: $rhs) {
                self.$view(r.view());
            }
        }
    };
}

// The join (`|`, `|=`) family. Routes through `Version::join_view`.
binop_matrix! {
    "version_join",
    "`a | b` and `a |= b`: the causal join, the operator matrix of [`Version::join`] over owned and borrowed operands.",
    BitOr::bitor, BitOrAssign::bitor_assign, join_view;
    // value operator: left operand becomes a fresh owned `Version`
    Version,  Version,  own;
    Version,  &Version, own;
    &Version, Version,  clone;
    &Version, &Version, clone;
    // assign: right operand folded into the left operand in place
    Version,  Version,  assign;
    Version,  &Version, assign;
}

// The meet (`&`, `&=`) family: the dual of the join matrix above, with the
// same cells and strategies, routing through `Version::meet_view` instead of
// `join_view`.
binop_matrix! {
    "version_meet",
    "`a & b` and `a &= b`: the causal meet, the operator matrix of [`Version::meet`] over owned and borrowed operands.",
    BitAnd::bitand, BitAndAssign::bitand_assign, meet_view;
    // value operator: left operand becomes a fresh owned `Version`
    Version,  Version,  own;
    Version,  &Version, own;
    &Version, Version,  clone;
    &Version, &Version, clone;
    // assign: right operand folded into the left operand in place
    Version,  Version,  assign;
    Version,  &Version, assign;
}

// ───────────────────────── the pair hull (`^`) ─────────────────────────
//
// `a ^ b` is `a.span(&b)`: the tightest `Span` containing both operands, `[a &
// b, a | b]`. Unlike the join and meet matrices above, the result leaves the
// operand type — a `Span`, not a `Version` — so the family has no assigning
// form (nothing of the receiver's type to assign back) and no owned-operand
// strategy: every cell reads both operands in place and mints the endpoints
// owned, exactly as the named method does.

/// Generates the span (`^`) matrix over owned and borrowed `Version`
/// operands.
///
/// Every cell delegates to [`Version::span`]; `Borrow::borrow` coerces an owned
/// or borrowed operand uniformly to `&Version`, so one arm covers all four
/// cells.
macro_rules! span_matrix {
    ($island:literal, $opdoc:literal, $($lhs:ty, $rhs:ty);* $(;)?) => {
        $(
            #[doc = $opdoc]
            #[doc = ""]
            #[doc = "# Complexity"]
            #[doc = ""]
            #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]
            impl BitXor<$rhs> for $lhs {
                type Output = Span<'static>;
                fn bitxor(self, r: $rhs) -> Span<'static> {
                    Version::span(self.borrow(), r.borrow())
                }
            }
        )*
    };
}

span_matrix! {
    "version_span",
    "`a ^ b`: the pair hull, the operator matrix of [`Version::span`].",
    Version,  Version;
    Version,  &Version;
    &Version, Version;
    &Version, &Version;
}

// ─────────────────────── projection onto a party (`/`) ───────────────────────
//
// `&v / &p` names `p`'s contribution to `v`: the value wherever `p` owns
// the region, zero everywhere else. The operator borrows both operands
// (never consuming or cloning the linear `Party`) and builds the
// [`OwnVersion`] view in O(1); comparisons decide against the view
// directly, and only the explicit [`OwnVersion::to_version`] pays the
// projection's product-growth materialization.
//
// Algebraic shape (exercised by `crate::laws`' projection laws): the
// projection is a sub-version (`v/p <= v`) and idempotent
// (`(v/p)/p == v/p`). It is additive across a fork
// (`v/p == v/p_left | v/p_right` for disjoint halves), and so a
// homomorphism of both join and meet (`(a|b)/p == a/p | b/p`,
// `(a&b)/p == a/p & b/p`); the whole-interval party leaves `v` unchanged.
// Projection can still raise `min_ticks` (carving one broad tick into
// disjoint peaks), so it is not monotone under `<=`.

/// `&v / &p`: the part of the [`Version`] `v` contributed within
/// [`Party`] `p`'s id region (zero everywhere `p` does not own), as a
/// lazy [`OwnVersion`] view.
///
/// [`Version::project`] is the named spelling of the same view.
///
/// # Complexity
///
/// `O(1)`.
///
/// # Example
///
/// ```
/// use before::Clock;
/// // Two disjoint halves each tick, then learn each other's history.
/// let mut a = Clock::seed();
/// let mut b = a.fork();
/// a.tick();
/// b.tick();
/// a.sync(&mut b).unwrap();
/// let v = a.version().clone();
/// // Each half's contribution is a sub-version, and the two rejoin to `v`.
/// assert!(&v / a.party() <= v && &v / b.party() <= v);
/// assert_eq!((&v / a.party()).to_version() | (&v / b.party()).to_version(), v);
/// ```
impl<'a> Div<&'a Party> for &'a Version {
    type Output = OwnVersion<'a>;
    fn div(self, party: &'a Party) -> OwnVersion<'a> {
        OwnVersion {
            party,
            version: self,
        }
    }
}

// Causal comparison over owned and borrowed `Version` operands, reading current
// state in place. Every cell comes from this macro, so the comparison matrix
// reads as a matrix. Each ordering cell delegates to the skyline comparison
// sweep; each equality cell is a byte compare of the two stored streams
// (`codec::canonical_eq`) — the skyline coding is a canonical unique
// representation, so byte equality is exactly causal equality. The `Version`
// derive list deliberately omits `PartialEq`/`PartialOrd` so the macro is the
// single source of both (see the note on the derive above).
macro_rules! causal_cmp_impls {
    ($($lhs:ty, $rhs:ty);* $(;)?) => {
        $(
            impl PartialEq<$rhs> for $lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    codec::canonical_eq(self.view(), o.view())
                }
            }
            impl PartialOrd<$rhs> for $lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    skyline::sweep::causal_cmp(self.view().live(), o.view().live())
                }
            }
            impl PartialEq<$rhs> for &$lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    codec::canonical_eq(self.view(), o.view())
                }
            }
            impl PartialOrd<$rhs> for &$lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    skyline::sweep::causal_cmp(self.view().live(), o.view().live())
                }
            }
            impl PartialEq<&$rhs> for $lhs {
                fn eq(&self, o: &&$rhs) -> bool {
                    codec::canonical_eq(self.view(), o.view())
                }
            }
            impl PartialOrd<&$rhs> for $lhs {
                fn partial_cmp(&self, o: &&$rhs) -> Option<Ordering> {
                    skyline::sweep::causal_cmp(self.view().live(), o.view().live())
                }
            }
        )*
    };
}

causal_cmp_impls! {
    Version, Version;
}
