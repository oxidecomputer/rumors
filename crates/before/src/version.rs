//! The interval-tree-clock event tree, [`Version`], and its chaining
//! mutation handle, [`Batch`].

use core::cmp::Ordering;
use core::fmt::Display;
use core::iter::Sum;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Div, DivAssign};

use crate::codec;
use crate::error::{Decode, Parse};
use crate::Party;

mod batch;
mod rank;
mod ranked;
// The skyline coding and its operation kernels: the stored representation
// and every algorithm over it. `pub` where the meter feature re-exports it
// so the resource-envelope suite can pin its internals; crate-private
// otherwise — no public item leaks the representation.
#[cfg(any(test, feature = "meter"))]
pub mod skyline;
#[cfg(not(any(test, feature = "meter")))]
pub(crate) mod skyline;

pub use batch::Batch;
pub use rank::Rank;
pub use ranked::Ranked;

#[cfg(test)]
mod tests;

/// A causal version: an event tree timestamping a [`Party`]'s history.
///
/// Comparison and **join** (`|`) are what give a version meaning;
/// [`tick`](Version::tick) is the only way to change one:
///
/// | Operation                                 | Meaning                                                        |
/// |-------------------------------------------|----------------------------------------------------------------|
/// | `a == b`                                  | identical causal history                                       |
/// | `a < b`, `a <= b`                         | `a` is causally dominated by `b`: every event in `a` is in `b` |
/// | [`a.concurrent(b)`](Version::concurrent)  | incomparable: neither dominates the other                      |
/// | `a \| b`, `a \|= b`                       | the *join* (least upper bound): the combined history of both   |
/// | `a & b`, `a &= b`                         | the *meet* (greatest lower bound): the history common to both  |
/// | [`a.tick(&p)`](Version::tick)             | record one new event for [`Party`] `p`                        |
///
/// Comparison is **partial** ([`PartialOrd`], not [`Ord`]): two distinct
/// versions can be [`concurrent`](Version::concurrent), and then `a < b`,
/// `a == b`, and `a > b` are all false.
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
    /// ```
    /// assert_eq!(before::Version::new().to_string(), "0");
    /// ```
    pub fn new() -> Self {
        let mut bits = codec::Bits::new();
        bits.push(false); // topology: the single leaf
        codec::encode_int(&mut bits, &codec::Base::ZERO); // its absolute height, zero
        Version::from_bits(bits)
    }

    /// Whether this version records no events: equal to [`Version::new`].
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// assert!(v.is_empty());
    /// v.tick(&Party::seed());
    /// assert!(!v.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        // The canonical empty version is exactly the 2-bit stream `01`: a
        // `0` leaf flag, then gamma(0), the single bit `1` (see
        // `Version::new` and `codec::encode_int`). The stored skyline
        // stream is a unique representation, so this O(1) bit test is the
        // whole question — no allocation, no walk.
        let empty = skyline::is_empty_stream(&self.0);
        debug_assert_eq!(
            empty,
            *self == Version::new(),
            "the two-bit emptiness test must agree with comparison against Version::new()",
        );
        empty
    }

    /// Advance the [`Version`] from the perspective of [`Party`].
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut v = Version::new();
    /// v.tick(&Party::seed());
    /// assert_eq!(v.to_string(), "1");
    /// ```
    pub fn tick(&mut self, party: &Party) {
        self.batch().tick(party);
    }

    /// Determine if two [`Version`]s are concurrent, i.e. incomparable.
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
    /// this [`Version`]: the sum of every base in its event tree, saturating
    /// at [`u64::MAX`].
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
    /// ```
    /// use before::Version;
    /// assert_eq!(Version::new().min_ticks(), 0);
    /// assert_eq!(Version::try_from(5).unwrap().min_ticks(), 5);
    /// // Concurrency forces more ticks than the tallest path (1) suggests:
    /// let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    /// assert_eq!(peaks.min_ticks(), 2);
    /// ```
    pub fn min_ticks(&self) -> u64 {
        skyline::query::min_ticks(&self.0)
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
    /// `O(n)` in the event tree.
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
    /// `O(n + m)` in the two event trees.
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
    /// // neither, so the two versions stand a distance of two ticks apart.
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
    /// `O(n + m)` in the two event trees.
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
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let all = Version::join_all([va.clone(), vb.clone()]);
    /// assert!(all >= va && all >= vb);
    /// assert_eq!(Version::join_all(Vec::<Version>::new()), Version::new());
    /// ```
    pub fn join_all<I: IntoIterator<Item = Version>>(iter: I) -> Version {
        // Balanced reduction on a binary-counter stack: an incoming
        // version merges upward while the top entry holds as many inputs
        // as it does, so every input passes through O(log n) joins and no
        // join's operand is more than a bounded factor larger than its
        // partner. A left fold instead joins each input into the whole
        // accumulated union — quadratic scan work on populations whose
        // accumulator never coalesces (interleaved single-tick versions).
        // Associativity makes the two groupings value-identical.
        let mut stack: Vec<(Version, u32)> = Vec::new();
        for v in iter {
            let mut merged = v;
            let mut weight = 0u32;
            while stack.last().is_some_and(|(_, w)| *w == weight) {
                let (top, _) = stack.pop().expect("the loop condition saw a top entry");
                merged = top | merged;
                weight += 1;
            }
            stack.push((merged, weight));
        }
        stack
            .into_iter()
            .fold(Version::new(), |acc, (v, _)| acc | v)
    }

    /// The meet (greatest lower bound) of every version in `iter`, or [`None`]
    /// if it is empty: the history every input shares.
    ///
    /// Unlike [`join_all`](Self::join_all) this returns an [`Option`], because
    /// the meet-semilattice has no identity. The empty meet would be the the
    /// version dominating all others, but no such [`Version`] exists (every
    /// version can [`tick`](Self::tick) higher), so an empty iterator yields
    /// [`None`].
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// let common = Version::meet_all([va.clone(), vb.clone()]).unwrap();
    /// assert!(common <= va && common <= vb);
    /// assert!(Version::meet_all(Vec::<Version>::new()).is_none()); // no top to return
    /// ```
    pub fn meet_all<I: IntoIterator<Item = Version>>(iter: I) -> Option<Version> {
        iter.into_iter().reduce(|acc, v| acc & v)
    }

    /// Begin a batch of operations on this [`Version`].
    ///
    /// [`Batch`] chains sequential operations on one version behind a single
    /// mutable borrow.
    ///
    /// ```
    /// use before::{Party, Version};
    /// let party = Party::seed();
    /// let mut v = Version::new();
    /// v.batch().tick(&party).tick(&party);
    /// assert_eq!(v.to_string(), "2");
    /// ```
    pub fn batch(&mut self) -> Batch<'_> {
        Batch::new(self)
    }

    /// A read-only view of this version's stored skyline stream.
    fn view(&self) -> &codec::Bits {
        &self.0
    }

    /// Encode this [`Version`] to bytes.
    ///
    /// The byte encoding of a [`Clock`](crate::Clock) is not the
    /// concatenation of the encodings of its [`Party`] and [`Version`]; see
    /// [`Clock::encode`](crate::Clock::encode).
    ///
    /// ```
    /// use before::Version;
    /// let v = Version::new();
    /// assert_eq!(Version::decode(&v.encode()[..]).unwrap(), v);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// Encode a [`Version`] to an arbitrary writer.
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

    /// Decode a [`Version`] from a reader of canonical bytes.
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
    /// The
    /// final partial byte is zero-padded in the stored form, so these bytes
    /// are a canonical identity: byte-equal if and only if the versions are
    /// equal, and consistent with [`hash`](core::hash::Hash).
    ///
    /// Their lexicographic order is an arbitrary total order with no causal
    /// meaning; use it only as a deterministic tiebreak between distinct
    /// versions. For causal comparison, use [`PartialOrd`] (`<=`) or
    /// [`concurrent`](Self::concurrent).
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
// [`join_all`](Version::join_all) (the empty case is the empty version); the
// borrowed forms clone each element into the running join. There is
// deliberately no meet counterpart here — the meet has no identity, so its fold
// is the `Option`-returning [`Version::meet_all`].

/// Joins the iterator's versions; the empty sum is [`Version::new`].
impl Sum<Version> for Version {
    fn sum<I: Iterator<Item = Version>>(iter: I) -> Version {
        Version::join_all(iter)
    }
}

/// Joins the iterator's versions; the empty sum is [`Version::new`].
impl<'a> Sum<&'a Version> for Version {
    fn sum<I: Iterator<Item = &'a Version>>(iter: I) -> Version {
        Version::join_all(iter.cloned())
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
impl FromIterator<Version> for Version {
    fn from_iter<I: IntoIterator<Item = Version>>(iter: I) -> Version {
        Version::join_all(iter)
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
impl<'a> FromIterator<&'a Version> for Version {
    fn from_iter<I: IntoIterator<Item = &'a Version>>(iter: I) -> Version {
        Version::join_all(iter.into_iter().cloned())
    }
}

/// Paper notation: `n` leaves, `(n, e1, e2)` nodes. E.g. `(1, 2, (0, (1, 0, 2), 0))`.
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

/// Parse paper notation (`n` or `(n, e1, e2)`), strictly rejecting non-normal-form input.
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

// The join (`|`, `|=`) and meet (`&`, `&=`) matrices across {Version, Batch}²,
// duals of each other, mirroring the comparison matrix below. The
// `binop_matrix!` macro generates every cell of both: 16 value-operator cells
// (lhs × rhs over {Version, &Version, Batch, &Batch}) and 8 assign cells (lhs
// over {Version, Batch}).
//
// A value-operator cell turns its left operand into a fresh owned `Version`
// (`own` moves an owned `Version`, `clone` copies a borrowed one, `snapshot`
// reads a `Batch`), then folds the right operand's view into it; a `Batch`
// read this way is not mutated. An assign cell folds the right operand's view
// into the left operand in place, through a transient `batch()` for a
// `Version` receiver (`batch`) or directly for a `Batch` (`direct`). The two
// families differ only in the view-folding method each cell routes through:
// `Batch::join_view` for join, `Batch::meet_view` for meet.

/// Generates one binary-operator family's full matrix across {Version, Batch}².
///
/// Parameterized over the value operator `$Op::$op` (e.g. `BitOr::bitor`), its
/// assigning form `$Assign::$assign` (e.g. `BitOrAssign::bitor_assign`), and the
/// view-folding method `$view` every cell routes through (`join_view` or
/// `meet_view`). Each strategy — `own`/`clone`/`snapshot` for value cells,
/// `batch`/`direct` for assign cells — has its own `@cell` arm so the receiver
/// `self` is written in the same expansion as the method it belongs to (`self`
/// cannot cross a macro-invocation boundary).
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
                out.batch().$view(r.view());
                out
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, clone) => {
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self.clone();
                out.batch().$view(r.view());
                out
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, snapshot) => {
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self.snapshot();
                out.batch().$view(r.view());
                out
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, batch) => {
        impl $Assign<$rhs> for $lhs {
            fn $assign(&mut self, r: $rhs) {
                self.batch().$view(r.view());
            }
        }
    };
    (@cell $Op:ident::$op:ident, $Assign:ident::$assign:ident, $view:ident, $lhs:ty, $rhs:ty, direct) => {
        impl $Assign<$rhs> for $lhs {
            fn $assign(&mut self, r: $rhs) {
                self.$view(r.view());
            }
        }
    };
}

// The join (`|`, `|=`) family. Routes through `Batch::join_view`.
binop_matrix! {
    BitOr::bitor, BitOrAssign::bitor_assign, join_view;
    // value operator: left operand becomes a fresh owned `Version`
    Version,    Version,    own;
    Version,    &Version,   own;
    Version,    Batch<'_>,  own;
    Version,    &Batch<'_>, own;
    &Version,   Version,    clone;
    &Version,   &Version,   clone;
    &Version,   Batch<'_>,  clone;
    &Version,   &Batch<'_>, clone;
    Batch<'_>,  Version,    snapshot;
    Batch<'_>,  &Version,   snapshot;
    Batch<'_>,  Batch<'_>,  snapshot;
    Batch<'_>,  &Batch<'_>, snapshot;
    &Batch<'_>, Version,    snapshot;
    &Batch<'_>, &Version,   snapshot;
    &Batch<'_>, Batch<'_>,  snapshot;
    &Batch<'_>, &Batch<'_>, snapshot;
    // assign: right operand folded into the left operand in place
    Version,    Version,    batch;
    Version,    &Version,   batch;
    Version,    Batch<'_>,  batch;
    Version,    &Batch<'_>, batch;
    Batch<'_>,  Version,    direct;
    Batch<'_>,  &Version,   direct;
    Batch<'_>,  Batch<'_>,  direct;
    Batch<'_>,  &Batch<'_>, direct;
}

// The meet (`&`, `&=`) family: the dual of the join matrix above, with the
// same cells and strategies, routing through `Batch::meet_view` instead of
// `join_view`.
binop_matrix! {
    BitAnd::bitand, BitAndAssign::bitand_assign, meet_view;
    // value operator: left operand becomes a fresh owned `Version`
    Version,    Version,    own;
    Version,    &Version,   own;
    Version,    Batch<'_>,  own;
    Version,    &Batch<'_>, own;
    &Version,   Version,    clone;
    &Version,   &Version,   clone;
    &Version,   Batch<'_>,  clone;
    &Version,   &Batch<'_>, clone;
    Batch<'_>,  Version,    snapshot;
    Batch<'_>,  &Version,   snapshot;
    Batch<'_>,  Batch<'_>,  snapshot;
    Batch<'_>,  &Batch<'_>, snapshot;
    &Batch<'_>, Version,    snapshot;
    &Batch<'_>, &Version,   snapshot;
    &Batch<'_>, Batch<'_>,  snapshot;
    &Batch<'_>, &Batch<'_>, snapshot;
    // assign: right operand folded into the left operand in place
    Version,    Version,    batch;
    Version,    &Version,   batch;
    Version,    Batch<'_>,  batch;
    Version,    &Batch<'_>, batch;
    Batch<'_>,  Version,    direct;
    Batch<'_>,  &Version,   direct;
    Batch<'_>,  Batch<'_>,  direct;
    Batch<'_>,  &Batch<'_>, direct;
}

// ───────────────────── projection onto a party (`/`, `/=`) ───────────────────
//
// `v / &p` masks `v` to `p`'s id: the value is kept wherever `p` owns the
// region and zeroed everywhere else ("`p`'s contribution to `v`"). It reads
// only the party's id bits (`as_bits`), never consuming or cloning the linear
// `Party`, so it takes `&Party` and leaves it untouched.
//
// Algebraic shape (exercised by `version::tests`): the projection is a
// sub-version (`v/p <= v`) and idempotent (`(v/p)/p == v/p`). It is additive
// across a fork (`v/p == v/p_left | v/p_right` for disjoint halves), and so a
// homomorphism of both join and meet (`(a|b)/p == a/p | b/p`,
// `(a&b)/p == a/p & b/p`); the whole-interval party leaves `v` unchanged.
// Projection can still raise `min_ticks` (carving one broad tick into
// disjoint peaks), so it is not monotone under `<=`.

/// `v / &p` — the part of the [`Version`] `v` contributed within [`Party`]
/// `p`'s id region (zero everywhere `p` does not own). The party is borrowed,
/// not consumed.
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
/// let from_a = &v / a.party();
/// let from_b = &v / b.party();
/// assert!(from_a <= v && from_b <= v);
/// assert_eq!(&from_a | &from_b, v);
/// ```
impl Div<&Party> for &Version {
    type Output = Version;
    fn div(self, party: &Party) -> Version {
        Version::from_bits(skyline::query::project(&self.0, party))
    }
}

impl Div<&Party> for Version {
    type Output = Version;
    fn div(self, party: &Party) -> Version {
        &self / party
    }
}

impl DivAssign<&Party> for Version {
    fn div_assign(&mut self, party: &Party) {
        *self = Version::from_bits(skyline::query::project(&self.0, party));
    }
}

// Causal comparison across {Version, Batch}², reading current state in place.
// All four cells — `Version`/`Version` included — come from this macro, so the
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
    Version, Batch<'_>;
    Batch<'_>, Version;
    Batch<'_>, Batch<'_>;
}
