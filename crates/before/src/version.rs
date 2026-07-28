//! The interval-tree-clock event tree, [`Version`].

use core::cmp::Ordering;
use core::fmt::Display;
use core::iter::Sum;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Div};

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
    /// `O(1)` time and space.
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
    /// `O(1)` time; no allocation.
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
    /// `O(|v|)` time and space.
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
    /// `O(|v|)` time and space; the returned rank's numeric size (see
    /// [`Rank`]) is itself `O(|v|)`.
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
    /// # Complexity
    ///
    /// `O(|a| + |b|)` time and space: one fused sweep over the two
    /// packed streams integrates the height difference directly, each
    /// step paid for by the codes it consumes.
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
    /// `O(|a| + |b|)` time and space.
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
    /// # Complexity
    ///
    /// `O(D log k)` time and `O(D)` space, where `D` is the inputs' total
    /// packed size and `k` their number: the fold is a balanced reduction,
    /// so every input passes through `O(log k)` joins of similarly sized
    /// operands.
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
    /// the meet-semilattice has no identity. The empty meet would be the
    /// version dominating all others, but no such [`Version`] exists (every
    /// version can [`tick`](Self::tick) higher), so an empty iterator yields
    /// [`None`].
    ///
    /// # Complexity
    ///
    /// `O(D)` time and space, the inputs' total packed size: a meet only
    /// shrinks, so each step of the fold is bounded by its smaller
    /// operand.
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

    /// A read-only view of this version's stored skyline stream.
    fn view(&self) -> &codec::Bits {
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
    /// [`Version::new`] (`join_all`, `Sum`) hit it on their first join.
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
    /// `O(1)` time; no allocation.
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
    /// `O(1)` time: a borrow, no copy.
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
///
/// # Complexity
///
/// `O(D log k)` time and `O(D)` space, as [`Version::join_all`], the fold
/// it is.
impl Sum<Version> for Version {
    fn sum<I: Iterator<Item = Version>>(iter: I) -> Version {
        Version::join_all(iter)
    }
}

/// Joins the iterator's versions; the empty sum is [`Version::new`].
///
/// # Complexity
///
/// `O(D log k)` time and `O(D)` space, as [`Version::join_all`], plus one
/// clone of each element.
impl<'a> Sum<&'a Version> for Version {
    fn sum<I: Iterator<Item = &'a Version>>(iter: I) -> Version {
        Version::join_all(iter.cloned())
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
///
/// # Complexity
///
/// `O(D log k)` time and `O(D)` space, as [`Version::join_all`], the fold
/// it is.
impl FromIterator<Version> for Version {
    fn from_iter<I: IntoIterator<Item = Version>>(iter: I) -> Version {
        Version::join_all(iter)
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
///
/// # Complexity
///
/// `O(D log k)` time and `O(D)` space, as [`Version::join_all`], plus one
/// clone of each element.
impl<'a> FromIterator<&'a Version> for Version {
    fn from_iter<I: IntoIterator<Item = &'a Version>>(iter: I) -> Version {
        Version::join_all(iter.into_iter().cloned())
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
/// `O(1)` time and space (the value is word-sized).
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
