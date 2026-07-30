//! The interval-tree-clock event tree, [`Version`].

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt::Display;
use core::iter::Sum;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Div};

use crate::causally;
use crate::codec;
use crate::error::{Decode, Parse};
use crate::Party;

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
pub use ticks::Ticks;

#[cfg(test)]
mod tests;

/// A causal version: an event tree timestamping a [`Party`]'s history.
///
/// Comparison and **join** (`|`) are what give a version meaning;
/// [`tick`](Version::tick) is the only operation that records *new*
/// history (a join or meet only combines histories already recorded):
///
/// | Operation                                 | Meaning                                                        |
/// |-------------------------------------------|----------------------------------------------------------------|
/// | `a == b`                                  | identical causal history                                       |
/// | `a < b`, `a <= b`                         | `a` is causally dominated by `b`: every event in `a` is in `b` |
/// | [`a.concurrent(b)`](Version::concurrent)  | incomparable: neither dominates the other                      |
/// | `a \| b`, `a \|= b`                       | the *join* (least upper bound): the combined history of both   |
/// | `a & b`, `a &= b`                         | the *meet* (greatest lower bound): the history common to both  |
/// | [`a.tick(&p)`](Version::tick)             | record one new event for [`Party`] `p`                        |
/// | [`a.ticks(&p, n)`](Version::ticks)        | record `n` new events for [`Party`] `p`, in one pass          |
///
/// Comparison is **partial** ([`PartialOrd`], not [`Ord`]): two distinct
/// versions can be [`concurrent`](Version::concurrent), and then `a < b`,
/// `a == b`, and `a > b` are all false.
///
/// # Complexity
///
/// A version's *packed size* `|v|` is the length of
/// [`encode`](Version::encode)'s bytes (borrowable without copying as
/// [`as_bytes`](Version::as_bytes); exact to the bit as
/// [`encoded_bits`](Version::encoded_bits)); every `# Complexity` section
/// on this type's operations is denominated in packed sizes. Every
/// comparison cell — `==`, `<`, `<=`,
/// [`partial_cmp`](PartialOrd::partial_cmp),
/// [`concurrent`](Version::concurrent) — and every join (`|`, `|=`) and
/// meet (`&`, `&=`) cell, over owned or borrowed operands,
/// is `O(|a| + |b|)` time and space, and a join or meet result
/// is `O(|a| + |b|)` bytes itself. Hashing is `O(|v|)`. The costs that
/// differ live on their operations: the projection `/` (an `O(1)` view
/// whose explicit materialization,
/// [`OwnVersion::to_version`](crate::OwnVersion::to_version), can outgrow
/// its operands), the n-ary folds
/// ([`join_all`](Version::join_all)), and the text conversions
/// (`Display` and `FromStr`, documented on their impls).
///
/// **Complexity**: every comparison, join, and meet `O(a + b)`; hashing `O(n)`.
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
// At rest, a `Version` is its canonical skyline stream in a
// length-carrying container ([`codec::Bits`]): the raw byte slice IS the
// wire encoding (`from_bits` zeroes the dead pad bits at the seam), and
// the live bit length is a cached parse product the wire legitimately
// omits — the stream is self-delimiting at the bit level. Canonical
// uniqueness makes byte equality exactly causal equality; `PartialEq` is
// the macro's byte-level stream compare (see `causal_cmp_impls!` and
// `codec::canonical_eq`), and the manual `Hash` below reads the same
// (raw bytes, live length) pair, so their consistency holds by
// construction.
#[derive(Clone, Eq)]
pub struct Version(codec::Bits);

impl core::hash::Hash for Version {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        codec::canonical_hash(&self.0, state);
    }
}

impl Version {
    /// The empty [`Version`], representing no [`tick`](Version::tick)s.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    ///
    /// ```
    /// assert_eq!(before::Version::new().to_string(), "0");
    /// ```
    pub fn new() -> Self {
        let mut bits = codec::Bits::new();
        bits.push(true); // topology: the single leaf
        codec::encode_int(&mut bits, &codec::Base::ZERO); // its absolute height, zero
        Version::from_bits(bits)
    }

    /// Whether this version records no events: equal to [`Version::new`].
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// assert!(v.is_empty());
    /// v.tick(&Party::seed());
    /// assert!(!v.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        // The canonical empty version is exactly the 2-bit stream `11`: a
        // `1` leaf flag, then gamma(0), the single bit `1` (see
        // `Version::new` and `codec::encode_int`). The stored skyline
        // stream is a unique representation, so this O(1) bit test is the
        // whole question — no allocation, no walk.
        skyline::is_empty_stream(&self.0)
    }

    /// Advances this version by one event for `party`.
    ///
    /// # Complexity
    ///
    /// `O(|v| + |p|)` time and space, the packed sizes of the version and
    /// the party. The bound holds per call, wide values included:
    /// recording one event can re-code a value as wide as the operands
    /// spell, but that width arrives in the packed operand carrying it and
    /// is paid at most a constant number of times.
    ///
    /// **Complexity**: `O(a + b)`.
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// v.tick(&Party::seed());
    /// assert_eq!(v.to_string(), "1");
    /// ```
    pub fn tick(&mut self, party: &Party) {
        *self = Version::from_bits(skyline::fill::tick(&self.0, party));
    }

    /// Advances this version by `n` events for `party`: byte-identical to
    /// `n` sequential [`tick`](Self::tick)s, computed in a bounded number
    /// of passes rather than `n`.
    ///
    /// `n` is any count — an unsigned integer literal converts in place,
    /// and a [`Ticks`] carries counts wider than any machine integer —
    /// and `n = 0` is the identity (the empty run), so replay drivers and
    /// folds can pass whatever count they hold.
    ///
    /// # Complexity
    ///
    /// `O(|v| + |p| + log n)` time and space [measured: the ticks rows of
    /// the resource-envelope suite pin the constants, and the flatness
    /// pin holds the whole `n`-dependence to the boundary codes' gamma
    /// width — at most two fused walks and one splice at any count, so
    /// skipping by `n` costs what one tick costs plus the width of `n`,
    /// never `n` walks].
    ///
    /// **Complexity**: `O(a + b + log m)`, `m` the tick count.
    ///
    /// ```
    /// use before::{Party, Ticks, Version};
    /// let party = Party::seed();
    /// let mut v = Version::new();
    /// v.ticks(&party, 5u64);
    /// assert_eq!(v.to_string(), "5");
    /// // One call skips forward by a count no iteration could reach.
    /// let wide: Ticks = "100000000000000000000000000".parse().unwrap();
    /// v.ticks(&party, wide);
    /// assert_eq!(v.to_string(), "100000000000000000000000005");
    /// ```
    pub fn ticks(&mut self, party: &Party, n: impl Into<Ticks>) {
        let n = n.into();
        *self = Version::from_bits(skyline::fill::ticks(&self.0, party, &n.0));
    }

    /// Tests whether two [`Version`]s are concurrent (incomparable).
    ///
    /// # Complexity
    ///
    /// One causal comparison: `O(|a| + |b|)` time and space.
    ///
    /// **Complexity**: `O(a + b)`.
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
    /// this [`Version`]: the sum of every base in its event tree, as an
    /// exact [`Ticks`] count at any magnitude.
    ///
    /// This is a floor over all causal histories: every sequence of
    /// [`fork`](crate::Clock::fork), `tick`, and
    /// [`join`](crate::Clock::join) that yields this version performs at
    /// least this many ticks, and some history achieves it exactly (for a
    /// leaf, a single [`Party`] ticking in a line). The floor exceeds the
    /// tallest root-to-leaf path sum whenever the history forked:
    /// `(0, (0,1,0), (0,0,1))` has no path taller than `1`, but its two
    /// peaks over disjoint regions force two independent ticks.
    ///
    /// There is no corresponding maximum. For any nonempty version the tick
    /// count is unbounded above: an increment over an interval can always be
    /// refined into two concurrent increments over its halves (forked,
    /// ticked, rejoined), producing the same version with one more tick.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(n)`.
    ///
    /// ```
    /// use before::{Ticks, Version};
    /// assert_eq!(Version::new().min_ticks(), Ticks::ZERO);
    /// assert_eq!(Version::try_from(5).unwrap().min_ticks(), Ticks::from(5u64));
    /// // Concurrency forces more ticks than the tallest path (1) suggests:
    /// let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    /// assert_eq!(peaks.min_ticks(), Ticks::from(2u64));
    /// ```
    pub fn min_ticks(&self) -> Ticks {
        Ticks(skyline::query::min_ticks(&self.0))
    }

    /// This [`Version`]'s exact causal [`Rank`], strictly monotone: `v < w`
    /// implies `v.rank() < w.rank()`, so equal ranks are never causally ordered
    /// (same version, or concurrent).
    ///
    /// Sorting by `(rank, some-total-tiebreak)`
    /// therefore yields a linear extension of the causal order: causes always
    /// sort before their effects. See [`Rank`] for the measure itself and why
    /// strictness holds.
    ///
    /// # Complexity
    ///
    /// `O(|v|)` space; the returned rank's numeric size (see
    /// [`Rank`]) is itself `O(|v|)`. Time, in three parts:
    ///
    /// - `O(M(|v|) · log |v|)` in the worst case, `M` the
    ///   integer-multiplication bound of the arithmetic backend: the
    ///   fold's only superlinear work is its settle products — parked
    ///   drift times a dense interval mass, the wide × dense shape at
    ///   either of the fold's two settle sites — and each is delegated
    ///   whole to the backend's multiplication, re-associated through
    ///   a mass-balanced product tree. The log factor is the tree's
    ///   depth, and it is absorbed geometrically whenever the products
    ///   run in a power-law tier of the backend's multiplication —
    ///   every tier below its quasilinear threshold (4,000-word
    ///   operand sides; no product's smaller side clears it before
    ///   the packed input is ~64 KiB) — so such inputs run in
    ///   `O(M(|v|))`, as does an input of any size that re-arms the
    ///   fold's parked drift `O(1)` times (both committed wide × dense
    ///   witness families are single re-armings).
    /// - `Ω(M(|v|))` on adversarial inputs: a version of
    ///   `Θ(bits(x) + bits(y))` stored bits can *embed* the product
    ///   of two arbitrary integers in its exact rank (numerator
    ///   `2·x·y + 1`), so a fold that answers exactly multiplies two
    ///   input-funded factors at linear overhead, and no fold goes
    ///   below the cost of one multiplication — the worst case
    ///   cannot reach `O(|v|)` while integer multiplication is
    ///   superlinear.
    /// - `O(|v| log |v|)` for streams whose parked drifts stay a
    ///   bounded number of digits wide — every committed board family,
    ///   dense trailing regions and many re-armings included: the fold
    ///   settles its re-armings once, through the product tree,
    ///   re-reading no width or density more times than the product
    ///   tree's depth, which stays logarithmic in `|v|`.
    ///
    /// The gap between `O(M(|v|) · log |v|)` and `Ω(M(|v|))` is not
    /// contractual: a future release may close the tree-depth factor,
    /// and none may do better than one multiplication on the
    /// embedded-product inputs.
    ///
    /// **Complexity**: `O(n)` space; time `O(M(n) · log n)` worst case, `Ω(M(n))` mandatory, `O(n log n)` with width-bounded parked drifts.
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
        skyline::query::rank(&self.0)
    }

    /// Views this version by its causal rank: the borrowing [`Ranked`]
    /// view, which compares by [`rank`](Self::rank) without
    /// materializing one and keys by the composite rank-then-version
    /// encoding.
    ///
    /// Equal to `Ranked::from(&self)` — this is the method spelling
    /// for chained call sites. Construction is `O(1)` and runs no
    /// fold; see [`Ranked`] for what the view's comparisons mean
    /// (rank order completed by a deterministic tiebreak, equality as
    /// version identity) and when to materialize a [`Rank`] instead.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    ///
    /// ```
    /// use before::{Ranked, Version};
    /// let half: Version = "(0, 1, 0)".parse().unwrap();
    /// let one = Version::try_from(1).unwrap();
    /// // Compare by rank; no Rank is materialized on either side.
    /// assert!(half.ranked() < one.ranked());
    /// // The method is exactly the borrowing view conversion.
    /// assert!(half.ranked() == Ranked::from(&half));
    /// assert_eq!(half.ranked().version(), &half);
    /// ```
    pub fn ranked(&self) -> Ranked<'_> {
        Ranked::from(self)
    }

    /// The causal distance between two versions: the [`Rank`] of their
    /// symmetric difference, `rank(self | other) - rank(self & other)`.
    ///
    /// This is a metric on the version lattice. [`Rank`] is the area under the
    /// event tree, so it is a *valuation*: `rank(a | b) + rank(a & b) ==
    /// rank(a) + rank(b)`. A strictly monotone valuation on a distributive
    /// lattice induces a metric, so `distance` is symmetric, zero only between
    /// equal versions, and obeys the triangle inequality. It measures how much
    /// history two replicas would have to exchange to converge: zero when they
    /// agree, growing with every event neither shares.
    ///
    /// # Complexity
    ///
    /// `O(|a| + |b|)` space: one fused sweep over the two packed
    /// streams integrates the height difference directly, each step
    /// paid for by the codes it consumes. Time is exactly
    /// [`rank`](Self::rank)'s, in its three parts (the sweeps share
    /// one integral): `O(M(|a| + |b|) · log (|a| + |b|))` in the worst
    /// case, `M` the integer-multiplication bound of the arithmetic
    /// backend — the settle products (wide parked drift times a dense
    /// interval mass, with or without any re-arming) ride the
    /// backend's multiplication through the mass-balanced product
    /// tree, and the tree-depth log is absorbed below the backend's
    /// quasilinear threshold and on `O(1)`-re-arming pairs of any
    /// size, where the bound is `O(M(|a| + |b|))`; `Ω(M(|a| + |b|))`
    /// on adversarial inputs (the answer-embedded product floors
    /// every fold); and `O((|a| + |b|) log (|a| + |b|))` for every
    /// committed board family and any pair whose parked drifts stay a
    /// bounded number of digits wide. The gap above `Ω(M(|a| + |b|))`
    /// is not contractual: a future release may close the tree-depth
    /// factor.
    ///
    /// **Complexity**: `O(a + b)` space; time `O(M(a + b) · log (a + b))` worst case, `Ω(M(a + b))` mandatory, `O((a + b) log (a + b))` with width-bounded parked drifts.
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
        skyline::query::distance(&self.0, &other.0)
    }

    /// How far `self` lags behind `other`: the [`Rank`] of the history `other`
    /// records that `self` does not, `rank(self | other) - rank(self)`.
    ///
    /// The directed half of [`distance`](Self::distance): exactly the size of
    /// the [`causally::delta`](crate::causally::delta) a replica at `self` must
    /// receive to reach `self | other`. Zero when `other <= self` (`self`
    /// already knows everything `other` does), and the two directions sum to
    /// the symmetric distance: `a.lag(b) + b.lag(a) == a.distance(b)`.
    ///
    /// # Complexity
    ///
    /// `O(|a| + |b|)` space. Time is exactly
    /// [`distance`](Self::distance)'s, in its three parts (one shared
    /// co-sweep integrates both measures' functionals):
    /// `O(M(|a| + |b|) · log (|a| + |b|))` in the worst case (`M` the
    /// integer-multiplication bound of the arithmetic backend, the
    /// tree-depth log absorbed below the backend's quasilinear
    /// threshold and on `O(1)`-re-arming pairs), `Ω(M(|a| + |b|))` on
    /// adversarial inputs, and `O((|a| + |b|) log (|a| + |b|))` when
    /// parked drifts stay a bounded number of digits wide; the same
    /// gap above the multiplication bound is likewise not
    /// contractual.
    ///
    /// **Complexity**: `O(a + b)` space; time `O(M(a + b) · log (a + b))` worst case, `Ω(M(a + b))` mandatory, `O((a + b) log (a + b))` with width-bounded parked drifts.
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
        skyline::query::lag(&self.0, &other.0)
    }

    /// The join (least upper bound) of every version in `iter`, or the empty
    /// [`Version`] for an empty iterator.
    ///
    /// The join-semilattice is a commutative idempotent monoid under `|` with
    /// identity [`Version::new`], so folding any collection is well defined and
    /// order-independent. The merged version dominates every input: it is the
    /// combined causal history of all of them.
    ///
    /// The meet has no such fold: the lattice has no top element (a version can
    /// always [`tick`](Self::tick) higher), so the empty meet has no value; see
    /// [`meet_all`](Self::meet_all), which returns [`Option`] for that reason.
    ///
    /// The items may be owned versions or references — anything that
    /// [borrows](Borrow) as a [`Version`]. Borrowed operands are read in
    /// place, allocating only results, so folding a collection you keep
    /// never copies it into an accumulator.
    ///
    /// # Complexity
    ///
    /// `O(D log k)` time and `O(D)` space, where `D` is the inputs' total
    /// packed size and `k` their number: the fold is a balanced reduction,
    /// so every input passes through `O(log k)` joins of similarly sized
    /// operands.
    ///
    /// **Complexity**: `O(D log k)` time, `O(D)` space.
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let all = Version::join_all([&va, &vb]); // by reference: both stay usable
    /// assert!(all >= va && all >= vb);
    /// assert_eq!(Version::join_all(Vec::<Version>::new()), Version::new());
    /// ```
    pub fn join_all<I>(iter: I) -> Version
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        Self::balanced_fold(iter, Self::join_refs, Self::join_view).unwrap_or_default()
    }

    /// The meet (greatest lower bound) of every version in `iter`, or [`None`]
    /// if it is empty: the history every input shares.
    ///
    /// Unlike [`join_all`](Self::join_all) this returns an [`Option`], because
    /// the meet-semilattice has no identity. The empty meet would be the
    /// version dominating all others, but no such [`Version`] exists (every
    /// version can [`tick`](Self::tick) higher), so an empty iterator yields
    /// [`None`].
    ///
    /// As with [`join_all`](Self::join_all), the items may be owned
    /// versions or references — anything that [borrows](Borrow) as a
    /// [`Version`] — and borrowed operands are read in place.
    ///
    /// # Complexity
    ///
    /// `O(D log k)` time and `O(D)` space, where `D` is the inputs' total
    /// packed size and `k` their number: the same balanced reduction as
    /// [`join_all`](Self::join_all), so every input passes through
    /// `O(log k)` meets of similarly sized operands. The balance is what
    /// bounds the worst case — a meet shrinks the running result's
    /// *value*, never necessarily its packed size, so a population that
    /// keeps it full-size (one deep version among operands that dominate
    /// it) re-walks that result once per level, never once per operand.
    ///
    /// **Complexity**: `O(D log k)` time, `O(D)` space.
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let common = Version::meet_all([&va, &vb]).unwrap(); // by reference
    /// assert!(common <= va && common <= vb);
    /// assert!(Version::meet_all(Vec::<Version>::new()).is_none()); // no top to return
    /// ```
    pub fn meet_all<I>(iter: I) -> Option<Version>
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        Self::balanced_fold(iter, Self::meet_refs, Self::meet_view)
    }

    /// The causal span from this version to `other`: the tightest
    /// [`Span`](crate::causally::Span) containing both —
    /// `[self & other, self | other]`, the pair's lattice hull.
    ///
    /// The binary form of [`span_all`](Self::span_all), and *total*
    /// where the validating door
    /// ([`Span::new`](crate::causally::Span::new)) must reject a pair
    /// no chain connects. On comparable versions the hull *is* the
    /// pair, reordered (the meet and join of comparable versions are
    /// the smaller and the larger), so `span` subsumes the flip repair
    /// `new` declines to perform — and it is deliberately the only
    /// repair offered: silently reordering a caller's *stated*
    /// endpoints would hide the caller bug
    /// [`Crossed`](crate::error::Crossed) surfaces, and no reordering
    /// exists for a concurrent pair, whose hull's endpoints are fresh
    /// versions bracketing both inputs.
    ///
    /// Both operands are read in place; the endpoints are minted
    /// owned, so the span borrows neither. The `span_is_the_pair_hull`
    /// law in [`laws`](crate::laws) pins the door: the endpoints are
    /// definitionally the pair's meet and join, operand order is
    /// irrelevant, a comparable pair's hull is its validated span
    /// either way around, and the n-ary form agrees at its edges.
    ///
    /// # Complexity
    ///
    /// One meet and one join over the pair.
    ///
    /// **Complexity**: `O(a + b)`.
    ///
    /// ```
    /// use before::{Clock, causally::{Placement, Span}};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let va2 = a.tick().clone();
    /// let vb = b.tick().clone(); // concurrent to alice's line
    ///
    /// // Comparable versions: the hull is the pair, reordered —
    /// // the flip repair, from either operand order.
    /// assert_eq!(va2.span(&va), Span::new(&va, &va2).unwrap());
    /// assert_eq!(va.span(&va2), Span::new(&va, &va2).unwrap());
    /// // A concurrent pair has no reordering to repair, but it has a
    /// // hull: both inputs sit strictly inside it.
    /// let hull = va.span(&vb);
    /// assert_eq!(hull.place(&va), Placement::Between);
    /// assert_eq!(hull.place(&vb), Placement::Between);
    /// ```
    pub fn span(&self, other: &Version) -> causally::Span<'static> {
        causally::Span::owned(Self::meet_refs(self, other), Self::join_refs(self, other))
    }

    /// The causal span of this version and every version in `others`:
    /// the tightest [`Span`](crate::causally::Span) containing them all
    /// — `[⋀ ({self} ∪ others), ⋁ ({self} ∪ others)]`, the lattice
    /// hull.
    ///
    /// The receiver is the guaranteed first element, and that is what
    /// makes the construction *total*: the lattice has no top, so the
    /// hull of *nothing* has no value
    /// ([`meet_all`](Self::meet_all) returns [`Option`] for exactly
    /// that reason) — seeded with `self`, the folds are never empty,
    /// and an empty iterator yields the coincident `[self, self]`.
    ///
    /// The items may be owned versions or references — anything that
    /// [borrows](Borrow) as a [`Version`], the
    /// [`join_all`](Self::join_all) calling convention. Borrowed
    /// operands are read in place; the endpoints are minted owned, so
    /// the span borrows nothing from the collection.
    ///
    /// The `span_all_is_the_lattice_hull` and `span_is_the_pair_hull`
    /// laws in [`laws`](crate::laws) pin the door: the endpoints are
    /// definitionally [`meet_all`](Self::meet_all) and
    /// [`join_all`](Self::join_all) over `{self} ∪ others`, which
    /// element rides as the receiver is irrelevant and so is item
    /// order, every input places within the hull (never
    /// [`Before`](crate::causally::Placement::Before) or
    /// [`After`](crate::causally::Placement::After)), the empty
    /// iterator yields `[self, self]`, and at one item the n-ary form
    /// is [`span`](Self::span).
    ///
    /// # Complexity
    ///
    /// One balanced fold over `{self} ∪ others`, the accumulator
    /// carrying both hull endpoints through a single pass — the
    /// iterator is never buffered, and a leaf combine derives its
    /// pair hull directly — with `D` the inputs' total packed size
    /// and `k` their number.
    ///
    /// **Complexity**: `O(D log k)` time, `O(D)` space.
    ///
    /// ```
    /// use before::{Clock, Version, causally::{Placement, Span}};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let va2 = a.tick().clone();
    ///
    /// // The hull of a collection: every input places within it.
    /// let hull = va.span_all([&vb, &va2]);
    /// for v in [&va, &vb, &va2] {
    ///     assert!(!matches!(hull.place(v), Placement::Before | Placement::After));
    /// }
    /// // The empty iterator is the coincident single-version span:
    /// // the receiver keeps the hull total.
    /// assert_eq!(
    ///     va.span_all(Vec::<Version>::new()),
    ///     Span::new(&va, &va).unwrap(),
    /// );
    /// ```
    pub fn span_all<I>(&self, others: I) -> causally::Span<'static>
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        // One balanced fold, two-sided accumulator: the hull needs both
        // lattice directions, and carrying `(lo, hi)` through the
        // counter feeds them from a single pass — the caller's iterator
        // is never buffered, and each input is read at its combines
        // alone. A leaf combine (two raw inputs) derives the pair hull;
        // an interior combine folds per side, because its two legs read
        // *different* operand pairs (`lo₁ ∧ lo₂` and `hi₁ ∨ hi₂`) — no
        // shared pair walk exists there to fuse.
        let inputs = core::iter::once(SpanInput::Receiver(self))
            .chain(others.into_iter().map(SpanInput::Item))
            .map(Hull::Input);
        let group = crate::fold::balanced_reduce(inputs, |a, b| {
            let (lo, hi) = match (a, b) {
                // A leaf combine: two raw inputs derive their pair hull.
                (Hull::Input(a), Hull::Input(b)) => {
                    let (a, b) = (a.version(), b.version());
                    (Self::meet_refs(a, b), Self::join_refs(a, b))
                }
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
                // weight-0 lone input never sits below a merged group in
                // the closing drain), but the match stays total rather
                // than asserting: both sides' combiners are commutative,
                // so folding the raw input into the owned hull is
                // value-identical.
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
            // The receiver alone (an empty iterator): the coincident
            // span, the one place an input itself becomes the hull.
            Hull::Input(input) => {
                let v = input.version();
                causally::Span::owned(v.clone(), v.clone())
            }
            Hull::Merged { lo, hi } => causally::Span::owned(lo, hi),
        }
    }

    /// The balanced reduction under [`join_all`](Self::join_all) and
    /// [`meet_all`](Self::meet_all): fold items that [borrow](Borrow) as
    /// [`Version`] through [`crate::fold::balanced_reduce`] without
    /// cloning them on entry.
    ///
    /// Each combiner comes in the two forms ownership demands — `refs`
    /// combines two borrowed inputs into a fresh owned result
    /// ([`join_refs`](Self::join_refs)/[`meet_refs`](Self::meet_refs)),
    /// and `view` folds a borrowed stream into an owned group in place
    /// ([`join_view`](Self::join_view)/[`meet_view`](Self::meet_view)) —
    /// and [`Group`] carries which form each operand needs. `None` is the
    /// empty fold; a lone input is cloned, the one place an input itself
    /// must become an owned result.
    fn balanced_fold<I>(
        iter: I,
        refs: fn(&Version, &Version) -> Version,
        view: fn(&mut Version, &codec::Bits),
    ) -> Option<Version>
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        let group = crate::fold::balanced_reduce(iter.into_iter().map(Group::Input), |a, b| {
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
                // weight-0 lone input never sits below a merged group in
                // the closing drain), but the match stays total rather
                // than asserting: both combiners are commutative, so
                // folding the borrowed side into the owned side is
                // value-identical.
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

    /// A read-only view of this version's stored skyline stream.
    pub(crate) fn view(&self) -> &codec::Bits {
        &self.0
    }

    /// The view-taking join core: fold an arbitrary skyline stream into
    /// this version in place.
    ///
    /// Every `|`/`|=` cell routes through here, so owned and borrowed
    /// operands join without transcoding.
    ///
    /// Before the merge sweep, two `O(1)` short-circuits settle the cases
    /// canonical form makes immediate: trivial equality (`a ∨ a = a`, a
    /// no-op, decided by a byte compare of the two unique streams) and the
    /// lattice identity `0 ∨ v = v` — an empty incoming leaves the current
    /// tree untouched, and an empty current adopts the incoming stream
    /// wholesale (a copy, byte-identical to what the merge would emit). The
    /// identity path is the common seed pattern: folds seeded with
    /// [`Version::new`] (the `|=` accumulation shape) hit it on their
    /// first join.
    fn join_view(&mut self, incoming: &codec::Bits) {
        if codec::canonical_eq(&self.0, incoming) {
            return; // a ∨ a = a
        }
        if skyline::is_empty_stream(incoming) {
            return; // v ∨ 0 = v: nothing to fold in
        }
        if skyline::is_empty_stream(&self.0) {
            // 0 ∨ v = v: adopt the incoming stream wholesale. Both streams
            // are canonical, so the copy equals the merge byte for byte.
            *self = Version::from_bits(incoming.clone());
            return;
        }
        *self = Version::from_bits(skyline::emit::join(&self.0, incoming));
    }

    /// The borrowed-operands join: `a ∨ b` as a fresh [`Version`],
    /// reading both operands in place.
    ///
    /// [`join_view`](Self::join_view) for the case where neither operand
    /// is owned — the same short-circuits in the same order (keep the two
    /// in lockstep), with each hand-back arm cloning the operand that is
    /// itself the answer. The general path emits the merged stream
    /// straight from the two views, so borrowing costs no clone and no
    /// transcoding.
    fn join_refs(a: &Version, b: &Version) -> Version {
        if codec::canonical_eq(&a.0, &b.0) {
            return a.clone(); // a ∨ a = a
        }
        if skyline::is_empty_stream(&b.0) {
            return a.clone(); // v ∨ 0 = v
        }
        if skyline::is_empty_stream(&a.0) {
            return b.clone(); // 0 ∨ v = v
        }
        Version::from_bits(skyline::emit::join(&a.0, &b.0))
    }

    /// The view-taking meet core, the dual of
    /// [`join_view`](Self::join_view): meet an arbitrary skyline stream
    /// into this version in place.
    ///
    /// The `&`/`&=` matrix routes through here just as the `|`/`|=` matrix
    /// routes through `join_view`.
    ///
    /// The dual short-circuits apply: trivial equality (`a ∧ a = a`), and the
    /// empty version as the *absorbing* element, `0 ∧ v = 0` — an empty
    /// current is already the answer, and an empty incoming makes the result
    /// the empty version outright, no merge sweep either way.
    fn meet_view(&mut self, incoming: &codec::Bits) {
        if codec::canonical_eq(&self.0, incoming) {
            return; // a ∧ a == a
        }
        if skyline::is_empty_stream(&self.0) {
            return; // 0 ∧ v = 0: already empty, nothing can shrink it
        }
        if skyline::is_empty_stream(incoming) {
            // v ∧ 0 = 0: the result is the empty version, whatever `v` was.
            *self = Version::new();
            return;
        }
        *self = Version::from_bits(skyline::emit::meet(&self.0, incoming));
    }

    /// The borrowed-operands meet: `a ∧ b` as a fresh [`Version`],
    /// reading both operands in place.
    ///
    /// [`meet_view`](Self::meet_view) for the case where neither operand
    /// is owned, exactly as [`join_refs`](Self::join_refs) mirrors
    /// [`join_view`](Self::join_view): the same short-circuits in the
    /// same order — keep the two in lockstep.
    fn meet_refs(a: &Version, b: &Version) -> Version {
        if codec::canonical_eq(&a.0, &b.0) {
            return a.clone(); // a ∧ a = a
        }
        if skyline::is_empty_stream(&a.0) {
            return a.clone(); // 0 ∧ v = 0: `a` is already the answer
        }
        if skyline::is_empty_stream(&b.0) {
            return Version::new(); // v ∧ 0 = 0, whatever `v` was
        }
        Version::from_bits(skyline::emit::meet(&a.0, &b.0))
    }

    /// Encodes this [`Version`] to bytes.
    ///
    /// A [`Clock`](crate::Clock)'s encoding is the byte-level concatenation
    /// of its [`Party`]'s and [`Version`]'s encodings; see
    /// [`Clock::encode`](crate::Clock::encode) for the framing rule.
    ///
    /// # Complexity
    ///
    /// `O(|v|)` time and space: one copy of the stored bytes
    /// ([`as_bytes`](Self::as_bytes) borrows the same bytes without
    /// copying).
    ///
    /// **Complexity**: `O(n)`.
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
    /// `O(|v|)` time: one write of the stored bytes, plus whatever the
    /// writer itself costs.
    ///
    /// **Complexity**: `O(n)`.
    ///
    /// ```
    /// use before::Version;
    /// let mut buf = Vec::new();
    /// Version::new().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Version::new().encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(self.as_bytes())
    }

    /// Decodes a [`Version`] from a reader of canonical bytes.
    ///
    /// # Complexity
    ///
    /// `O(n)` time and space in the bytes read, accepted or rejected:
    /// strict validation is one pass over the stream, and the result
    /// reuses the read buffer.
    ///
    /// **Complexity**: `O(n)`.
    ///
    /// ```
    /// use before::Version;
    /// let bytes = Version::new().encode();
    /// assert_eq!(Version::decode(&bytes[..]).unwrap(), Version::new());
    /// ```
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Self, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        let end = {
            let bits = codec::bytes_as_bits(&buf);
            let end = skyline::validate_prefix(bits)?;
            codec::require_zero_padding(bits, end)?;
            end
        };
        // Reuse the read buffer as the result's backing store (offset-0,
        // canonical up to `end`), so decoding allocates no more than before.
        let mut bits = codec::Bits::from_vec(buf);
        bits.truncate(end);
        Ok(Version::from_bits(bits))
    }

    /// The exact length in bits of [`encode`](Self::encode) before its zero-pad
    /// to a byte boundary.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    ///
    /// ```
    /// use before::Version;
    /// // The empty version is a single `0` leaf: a flag bit plus a value bit.
    /// assert_eq!(Version::new().encoded_bits(), 2);
    /// ```
    pub fn encoded_bits(&self) -> usize {
        self.0.len()
    }

    /// The canonical packed bytes of this [`Version`]: what
    /// [`encode`](Self::encode) produces, borrowed without copying.
    ///
    /// The final partial byte is zero-padded in the stored form, so these
    /// bytes are a canonical identity: byte-equal if and only if the versions
    /// are equal, and consistent with [`hash`](core::hash::Hash).
    ///
    /// Their lexicographic order is an arbitrary total order with no causal
    /// meaning; use it only as a deterministic tiebreak between distinct
    /// versions. For causal comparison, use [`PartialOrd`] (`<=`) or
    /// [`concurrent`](Self::concurrent).
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    ///
    /// ```
    /// use before::Version;
    /// let v = Version::try_from((1, 0, 1)).unwrap();
    /// assert_eq!(v.as_bytes(), v.encode().as_slice());
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        debug_assert!(
            codec::dead_bits_are_zero(&self.0),
            "non-canonical Version storage: dead bits past the live length must be zero",
        );
        self.0.as_raw_slice()
    }

    /// The stored skyline stream, borrowed as live bits.
    ///
    /// Test- and meter-only: the meter surface's `skyline::encode` and
    /// the differential bridges read it; production code goes through
    /// [`Self::as_bytes`] or the crate-internal `view`.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn as_bits(&self) -> &codec::BitsSlice {
        &self.0
    }

    /// Wrap a normal-form skyline bit stream as a `Version`, canonicalizing
    /// its storage. The single gate every built/parsed `Version` passes
    /// through.
    ///
    /// Callers guarantee canonical skyline form; this zeroes the dead bits
    /// past the live length so the stored bytes are canonical (see
    /// [`codec::zero_dead_bits`]).
    pub(crate) fn from_bits(mut bits: codec::Bits) -> Self {
        codec::zero_dead_bits(&mut bits);
        Version(bits)
    }
}

/// One group in [`Version::balanced_fold`]'s counter: an input exactly as
/// the caller supplied it (owned or borrowed through [`Borrow`], never
/// cloned on entry), or the owned version a combine produced.
///
/// Weight-0 counter entries are always lone [`Input`](Group::Input)s and
/// every combine's output is [`Merged`](Group::Merged), so the
/// distinction lets each combine pick the operand form its combiner
/// needs — in place for borrowed inputs, by view-fold for owned groups.
enum Group<B> {
    /// An input the fold has not yet combined, still in the caller's form.
    Input(B),
    /// The owned running result of one or more combines.
    Merged(Version),
}

/// One raw input to [`Version::span_all`]'s fold: the receiver rides by
/// borrow, the caller's items in their own form (owned or borrowed
/// through [`Borrow`], never cloned on entry).
enum SpanInput<'r, B> {
    /// The receiver: the guaranteed first element that keeps the hull
    /// total.
    Receiver(&'r Version),
    /// One of the caller's items.
    Item(B),
}

impl<B: Borrow<Version>> SpanInput<'_, B> {
    /// The borrowed version this input contributes.
    fn version(&self) -> &Version {
        match self {
            SpanInput::Receiver(v) => v,
            SpanInput::Item(b) => b.borrow(),
        }
    }
}

/// One group in [`Version::span_all`]'s counter: [`Group`]'s shape with
/// the two-sided hull accumulator, so one balanced fold carries both
/// lattice directions.
enum Hull<'r, B> {
    /// An input the fold has not yet combined.
    Input(SpanInput<'r, B>),
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
// over an `Iterator` and `.collect()` into a `Version`. Both are
// [`join_all`](Version::join_all) (the empty case is the empty version), which
// takes the borrowed forms' references as they come. There is deliberately no
// meet counterpart here — the meet has no identity, so its fold is the
// `Option`-returning [`Version::meet_all`].

/// Joins the iterator's versions; the empty sum is [`Version::new`].
///
/// # Complexity
///
/// `O(D log k)` time and `O(D)` space, as [`Version::join_all`], the fold
/// it is.
///
/// **Complexity**: `O(D log k)` time, `O(D)` space.
impl Sum<Version> for Version {
    fn sum<I: Iterator<Item = Version>>(iter: I) -> Version {
        Version::join_all(iter)
    }
}

/// Joins the iterator's versions; the empty sum is [`Version::new`].
///
/// # Complexity
///
/// `O(D log k)` time and `O(D)` space, as [`Version::join_all`], the fold
/// it is: the borrowed elements are read in place, not cloned.
///
/// **Complexity**: `O(D log k)` time, `O(D)` space.
impl<'a> Sum<&'a Version> for Version {
    fn sum<I: Iterator<Item = &'a Version>>(iter: I) -> Version {
        Version::join_all(iter)
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
///
/// # Complexity
///
/// `O(D log k)` time and `O(D)` space, as [`Version::join_all`], the fold
/// it is.
///
/// **Complexity**: `O(D log k)` time, `O(D)` space.
impl FromIterator<Version> for Version {
    fn from_iter<I: IntoIterator<Item = Version>>(iter: I) -> Version {
        Version::join_all(iter)
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
///
/// # Complexity
///
/// `O(D log k)` time and `O(D)` space, as [`Version::join_all`], the fold
/// it is: the borrowed elements are read in place, not cloned.
///
/// **Complexity**: `O(D log k)` time, `O(D)` space.
impl<'a> FromIterator<&'a Version> for Version {
    fn from_iter<I: IntoIterator<Item = &'a Version>>(iter: I) -> Version {
        Version::join_all(iter)
    }
}

/// Paper notation: `n` leaves, `(n, e1, e2)` nodes. E.g. `(1, 2, (0, (1, 0, 2), 0))`.
///
/// # Complexity
///
/// `O(|v| + t)` space, the packed version plus the `t` rendered text
/// bytes. Time is **superlinear** in the worst case, on two counts: each
/// value wider than a machine word pays binary-to-decimal conversion,
/// superlinear (though subquadratic) in its width; and a deep tree of
/// wide interior values additionally pays a summary-merge cost that grows
/// faster than the operand. The merge cost is the renderer's, not the
/// format's — parsing the same text back pays only the conversion — and
/// is not contractual: a future release may render in time linear but
/// for conversion.
///
/// **Complexity**: `O(n + t)` space; time superlinear in the spelled value widths (decimal conversion plus the render merge).
///
/// ```
/// use before::Version;
/// let v: Version = "(1, 2, (0, (1, 0, 2), 0))".parse().unwrap();
/// assert_eq!(v.to_string(), "(1, 2, (0, (1, 0, 2), 0))");
/// ```
impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&skyline::text::render(&self.0))
    }
}

/// The same format as `Display`.
///
/// ```
/// assert_eq!(format!("{:?}", before::Version::new()), "0");
/// ```
impl core::fmt::Debug for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

/// Parses paper notation (`n` or `(n, e1, e2)`), strictly rejecting
/// non-normal-form input.
///
/// # Complexity
///
/// `O(t + |v|)` time and space, the input text plus the packed version
/// produced — accepted or rejected — except that each spelled value wider
/// than a machine word pays decimal-to-binary conversion, superlinear
/// (though subquadratic) in that value's width.
///
/// **Complexity**: `O(t + n)` space; time superlinear in the spelled value widths (decimal-to-binary conversion).
///
/// ```
/// use before::Version;
/// let v: Version = "(1, 0, 1)".parse().unwrap();
/// assert_eq!(v.to_string(), "(1, 0, 1)");
/// ```
impl core::str::FromStr for Version {
    type Err = Parse;
    fn from_str(s: &str) -> Result<Self, Parse> {
        Ok(Version::from_bits(skyline::text::parse(s)?))
    }
}

/// An event leaf from its base value, e.g. `Version::try_from(3u64)`.
///
/// # Complexity
///
/// **Complexity**: `O(1)`.
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

/// An event node from an `(n, left, right)` literal, e.g.
/// `Version::try_from((1u64, 0u64, (2u64, 0u64, 1u64)))`. Rejects non-normal-form nodes
/// (no zero-base child, or a collapsible `(n, m, m)`).
///
/// # Complexity
///
/// `O(|v|)` time and space in the version built.
///
/// **Complexity**: `O(n)`.
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
        Ok(Version::from_bits(skyline::literal::node(n, &l.0, &r.0)?))
    }
}

// The join (`|`, `|=`) and meet (`&`, `&=`) matrices over owned and
// borrowed `Version` operands, duals of each other, mirroring the
// comparison matrix below. The `binop_matrix!` macro generates every cell
// of both families: four value-operator cells (lhs × rhs over
// {Version, &Version}) and two assign cells (rhs over {Version, &Version}).
//
// A value-operator cell turns its left operand into a fresh owned `Version`
// (`own` moves an owned `Version`, `clone` copies a borrowed one), then
// folds the right operand's view into it. An assign cell folds the right
// operand's view into the receiver in place. The two families differ only
// in the view-folding method each cell routes through: `Version::join_view`
// for join, `Version::meet_view` for meet.

/// Generates one binary-operator family's full matrix over owned and
/// borrowed `Version` operands.
///
/// Parameterized over the value operator `$Op::$op` (e.g. `BitOr::bitor`), its
/// assigning form `$Assign::$assign` (e.g. `BitOrAssign::bitor_assign`), and the
/// view-folding method `$view` every cell routes through (`join_view` or
/// `meet_view`). Each strategy — `own`/`clone` for value cells, `assign` for
/// assign cells — has its own `@cell` arm so the receiver `self` is written
/// in the same expansion as the method it belongs to (`self` cannot cross a
/// macro-invocation boundary).
macro_rules! binop_matrix {
    ($Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident;
     $($lhs:ty, $rhs:ty, $strat:tt);* $(;)?
    ) => {
        $( binop_matrix!(@cell $Op::$op, $Assign::$assign, $view, $lhs, $rhs, $strat); )*
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, own) => {
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self;
                out.$view(r.view());
                out
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, clone) => {
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self.clone();
                out.$view(r.view());
                out
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, assign) => {
        impl $Assign<$rhs> for $lhs {
            fn $assign(&mut self, r: $rhs) {
                self.$view(r.view());
            }
        }
    };
}

// The join (`|`, `|=`) family. Routes through `Version::join_view`.
binop_matrix! {
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

/// `&v / &p` — the part of the [`Version`] `v` contributed within
/// [`Party`] `p`'s id region (zero everywhere `p` does not own), as the
/// lazy [`OwnVersion`] view. Both operands are borrowed, not consumed.
///
/// The view compares directly (against a [`Version`] or another view);
/// the projected [`Version`] itself exists only through the explicit
/// [`OwnVersion::to_version`], whose result can outgrow its operands.
///
/// # Complexity
///
/// `O(1)` time and space: the view borrows its operands. Every cost lives
/// on the view's operations ([`OwnVersion`]'s doc carries them).
///
/// **Complexity**: `O(1)`.
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

// Causal comparison over owned and borrowed `Version` operands, reading
// current state in place. Every cell comes from this macro, so the
// comparison matrix reads as a matrix. Each ordering cell delegates to the
// skyline comparison sweep; each equality cell is a byte compare of the two
// stored streams (`codec::canonical_eq`) — the skyline coding is a canonical
// unique representation, so byte equality is exactly causal equality. The
// `Version` derive list deliberately omits `PartialEq`/`PartialOrd` so the
// macro is the single source of both (see the note on the derive above).
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
                    skyline::sweep::causal_cmp(self.view(), o.view())
                }
            }
            impl PartialEq<$rhs> for &$lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    codec::canonical_eq(self.view(), o.view())
                }
            }
            impl PartialOrd<$rhs> for &$lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    skyline::sweep::causal_cmp(self.view(), o.view())
                }
            }
            impl PartialEq<&$rhs> for $lhs {
                fn eq(&self, o: &&$rhs) -> bool {
                    codec::canonical_eq(self.view(), o.view())
                }
            }
            impl PartialOrd<&$rhs> for $lhs {
                fn partial_cmp(&self, o: &&$rhs) -> Option<Ordering> {
                    skyline::sweep::causal_cmp(self.view(), o.view())
                }
            }
        )*
    };
}

causal_cmp_impls! {
    Version, Version;
}
