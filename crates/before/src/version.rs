//! The interval-tree-clock event tree, [`Version`].

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt::Debug;
use core::hash::Hash;
use core::iter::Sum;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Div};
use std::borrow::Cow;
use std::io::{Read, Result as IoResult, Write};

use crate::bits::Bits;
use crate::error::{Decode, ParseValue};
use crate::span::Span;
use crate::{text, Count, Party};

#[cfg(any(test, feature = "meter"))]
pub(crate) mod instrument;
pub(crate) mod io;
mod lattice;
mod measure;
mod order;
pub(crate) mod overlay;
mod own;
pub(crate) mod place;
mod projection;
mod range_minima;
pub(crate) mod shape;
pub(crate) mod tick;

pub use own::OwnVersion;

use crate::{Rank, Ranked};
use lattice::Extreme;

#[cfg(test)]
mod tests;

/// A timestamp representing causal history.
///
/// [`Version::tick`] and [`ticks`](Version::ticks) record new events for
/// a [`Party`]. Comparison asks whether one [`Version`] contains all events in
/// another: `a < b` means every event in `a` is present in `b`, while
/// [`Version::concurrent`] means neither contains the other.
/// [`Version::join`] (`|`) combines histories, and
/// [`meet`](Version::meet) (`&`) retains only their common history. Neither
/// lattice operation records a new event.
///
/// | Operation                                | Meaning                                                        |
/// |------------------------------------------|----------------------------------------------------------------|
/// | `a == b`                                 | identical causal history                                       |
/// | `a < b`, `a <= b`                        | every event in `a` is present in `b`                            |
/// | [`a.concurrent(b)`](Version::concurrent) | neither version contains the other                              |
/// | `a \| b`, `a \|= b`                      | join: the least version containing both histories               |
/// | `a & b`, `a &= b`                        | meet: the greatest history common to both                       |
/// | [`a.tick(&p)`](Version::tick)            | record one event for [`Party`] `p`                              |
/// | [`a.ticks(&p, k)`](Version::ticks)       | record `k` events for `p` in one pass                           |
///
/// Comparison is **partial** ([`PartialOrd`], not [`Ord`]): two distinct
/// versions can be [`concurrent`](Version::concurrent), and then `a < b`, `a ==
/// b`, and `a > b` are all false.
///
/// [`Display`](core::fmt::Display) and [`FromStr`](core::str::FromStr) use the
/// lowercase hexadecimal form of the canonical bytes. Parsing also accepts
/// uppercase hexadecimal letters.
///
/// # Complexity
///
/// All comparisons are linear in the combined input size, but they need not
/// read the same amount. [`partial_cmp`](PartialOrd::partial_cmp) must
/// distinguish all four outcomes — less, equal, greater, or concurrent — so
/// it continues until the full causal relation is known. A directional
/// operator such as `<=` asks only whether one history is contained in the
/// other and stops at the first counterexample. Equality compares the
/// canonical bytes and stops at the first difference (each version has one
/// byte representation, so byte equality is exactly causal equality):
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_cmp.html")))]
#[cfg_attr(
    not(doc),
    doc = "full relation: `O(n)` in total input bytes; `O(|a| + |b|)`"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_order.html")))]
#[cfg_attr(
    not(doc),
    doc = "directional (`<=`): `O(n)` in total input bytes; `O(|a| + |b|)`; stops when this direction is disproved"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_eq.html")))]
#[cfg_attr(
    not(doc),
    doc = "equality: `O(n)` in total input bytes; `O(|a| + |b|)`: canonical byte compare, measured on equal pairs"
)]
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
// A `Version` is always represented by one canonical preorder stream
// ([`Bits`]); its stored bytes are also its wire encoding.
//
// Canonical uniqueness makes byte equality exactly causal equality; `PartialEq`
// is the byte-level stream compare below, and the
// manual `Hash` below reads the same bytes, so their consistency holds by
// construction. The
// container's backing store is refcounted (`bytes::Bytes`), which is what makes
// the derived `Clone` `O(1)`: a clone shares the buffer, and
// `Bits::eq` recognizes shared storage before comparing bytes.
#[derive(Clone, Eq)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct Version(Bits);

/// Renders the version's canonical bytes as lowercase hexadecimal.
///
/// Takes `O(n)` time and output space for `n` canonical bytes.
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_display.html")))]
#[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        text::write_hex(self.as_bytes(), f)
    }
}

/// Parses the hexadecimal form produced by [`Display`](core::fmt::Display).
/// Hexadecimal letters may use either case; prefixes and whitespace are not
/// accepted.
///
/// # Errors
///
/// Returns [`ParseValue::InvalidSyntax`] for malformed hexadecimal and
/// [`ParseValue::InvalidEncoding`] when the decoded bytes are not a canonical
/// [`Version`].
///
/// Takes `O(n)` time and `O(n)` space for `n` text bytes.
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_parse.html")))]
#[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(n)` in text bytes")]
impl core::str::FromStr for Version {
    type Err = ParseValue;

    fn from_str(text: &str) -> Result<Self, ParseValue> {
        text::decode_hex(text, |bytes| Version::decode_bytes(bytes.into()))
    }
}

/// Hashes exactly as its byte view, [`as_bytes`](Version::as_bytes), does.
///
/// Under any hasher, a version and its canonical bytes feed the same data,
/// so they hash equally. This is consistent with `Eq`, which compares those
/// bytes.
///
/// # Complexity
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_hash.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n)` in total input bytes; `O(|self|)`: one pass over the canonical bytes"
)]
impl Hash for Version {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Version {
    /// Adopt a canonical [`Version`] representation produced or validated by
    /// its I/O boundary.
    pub(crate) fn from_canonical(bits: Bits) -> Self {
        Version(bits)
    }

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
        // Encoding tests pin the constant against the built form.
        static EMPTY_STREAM: &[u8] = &[0b1110_0000];
        Version::from_canonical(Bits::from_canonical(bytes::Bytes::from_static(
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
        // Canonical storage gives the empty version exactly one byte: the leaf
        // flag and gamma(0), `11`, followed by the marker and zero padding.
        // Comparing that byte avoids turning a representation predicate into a
        // metered tree read.
        self.0.as_raw_slice() == [0b1110_0000]
    }

    /// Advances this version by one event for `party`.
    ///
    /// The result strictly dominates the previous version. The new event
    /// changes history only within `party`'s region; projecting onto any
    /// disjoint party gives the same history as before. Consequently, ticking
    /// two disjoint parties from the same version produces distinct versions.
    ///
    /// Dealing directly with a [`Party`] and a [`Version`] permits one version
    /// to be [`tick`](Version::tick)ed by many parties, or one [`Party`] to
    /// [`tick`](Party::tick) many [`Version`]s; this is in contrast to a
    /// [`Clock`](crate::Clock), which binds the two together.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_tick.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |party|)`")]
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
        *self = tick::TickWalk::tick(self, party);
    }

    /// Advances this version by `k` events for `party`.
    ///
    /// This is identical to `k` sequential [`tick`](Self::tick)s, but computed
    /// much more efficiently.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_ticks.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(|self| + |party| + log k)`"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Version};
    /// let party = Party::seed();
    /// let mut v = Version::new();
    /// v.ticks(&party, 5u64);
    /// let mut w = Version::new();
    /// for _ in 0..5 {
    ///     w.tick(&party);
    /// }
    /// assert_eq!(v, w); // one call, same version as five sequential ticks
    /// ```
    pub fn ticks(&mut self, party: &Party, k: impl Into<Count>) {
        let k = k.into();
        // The empty run is the identity, settled without re-freezing the stream.
        if k.0.bits() == 0 {
            return;
        }
        *self = tick::TickWalk::ticks(self, party, &k.0);
    }

    /// Tests whether two [`Version`]s are concurrent (incomparable).
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_concurrent.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |version|)`")]
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
    /// this [`Version`], as an exact [`Count`] at any magnitude.
    ///
    /// This is a floor over all causal histories: every sequence of
    /// [`fork`](crate::Clock::fork), `tick`, and [`join`](crate::Clock::join)
    /// that could have yielded this version must have performed at least this
    /// many ticks. The true history could have performed arbitrarily many more.
    ///
    /// There is no corresponding maximum. For any nonempty version, an
    /// increment over an interval can instead be performed concurrently over
    /// its two halves. Rejoining them produces the same version from one
    /// additional tick.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_min_ticks.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Count, Version};
    /// assert_eq!(Version::new().min_ticks(), Count::ZERO);
    /// let mut p = Party::seed();
    /// let mut v = Version::new();
    /// v.ticks(&p, 5u64);
    /// assert_eq!(v.min_ticks(), Count::from(5u64));
    /// let mut q = p.fork();
    /// let _ = p.fork();
    /// let r = q.fork();
    /// let mut peaks = Version::new();
    /// peaks.tick(&p);
    /// peaks.tick(&r);
    /// assert_eq!(peaks.min_ticks(), Count::from(2u64));
    /// ```
    pub fn min_ticks(&self) -> Count {
        Count::min_ticks_for(self)
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_rank.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
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
    /// assert!(va.rank() < joined.rank());
    /// assert!(b.version().rank() < joined.rank());
    /// ```
    pub fn rank(&self) -> Rank {
        Rank::of_version(self)
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

    /// The *causal distance* between two [`Version`]s.
    ///
    /// This measures how much history two replicas would have to exchange to
    /// converge: zero when they agree, growing with every event neither shares.
    /// It equals `(self | other).rank() - (self & other).rank()`, but is
    /// computed in one traversal.
    ///
    /// Distance is symmetric, zero only between equal versions, and obeys the
    /// triangle inequality.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_distance.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Rank};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// assert_eq!(va.distance(&va), Rank::ZERO);
    /// assert_eq!(va.distance(&vb), vb.distance(&va));
    /// assert_eq!(va.distance(&vb).to_string(), "1");
    /// ```
    pub fn distance(&self, other: &Version) -> Rank {
        if self == other {
            Rank::ZERO
        } else {
            Rank::distance_between(self, other)
        }
    }

    /// How far `self` lags behind `other`.
    ///
    /// This computes the [`Rank`] of the history `other` records that `self`
    /// does not, `(self | other).rank() - self.rank()`, in one traversal. It is
    /// zero when `other <= self`, and the two directions sum to the symmetric
    /// distance: `a.lag(b) + b.lag(a) == a.distance(b)`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_lag.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Rank};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let vb = b.tick().clone();
    /// assert_eq!(va.lag(&va), Rank::ZERO);
    /// assert!(va.lag(&vb) > Rank::ZERO);
    /// assert_eq!(va.lag(&vb) + vb.lag(&va), va.distance(&vb));
    /// ```
    pub fn lag(&self, other: &Version) -> Rank {
        if self == other {
            Rank::ZERO
        } else {
            Rank::lag_between(self, other)
        }
    }

    /// The join (least upper bound) of this [`Version`] and `other`: their
    /// combined causal history.
    ///
    /// Identical to the operator form `self | other`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// The result's canonical encoding is no longer than the two operands'
    /// encodings together.
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
    #[must_use = "`Version::join` does not modify `self` or `other`; discarding its result means that it has no effect"]
    pub fn join(&self, other: &Version) -> Version {
        self.extreme(Extreme::Higher, other)
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
    )]
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
    #[must_use = "`Version::join_all` does not modify `self`; discarding its result means that it has no effect"]
    pub fn join_all<I>(&self, iter: I) -> Version
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        Self::balanced_fold(self.with_items(iter), Extreme::Higher)
            .expect("the fold is seeded with the receiver: never empty")
    }

    /// The meet (greatest lower bound) of this version and `other`: the
    /// history the two share.
    ///
    /// Identical to the operator form `self & other`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_meet.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// The result's canonical encoding is no longer than the two operands'
    /// encodings together.
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
    #[must_use = "`Version::meet` does not modify `self` or `other`; discarding its result means that it has no effect"]
    pub fn meet(&self, other: &Version) -> Version {
        self.extreme(Extreme::Lower, other)
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_meet_all.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
    )]
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
    #[must_use = "`Version::meet_all` does not modify `self`; discarding its result means that it has no effect"]
    pub fn meet_all<I>(&self, iter: I) -> Version
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        Self::balanced_fold(self.with_items(iter), Extreme::Lower)
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_span.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Placement, Span};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let va = a.tick().clone();
    /// let va2 = a.tick().clone();
    /// let vb = b.tick().clone(); // concurrent with `a`'s line
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
        let (lo, hi) = self.hull(other);
        Span::owned(lo, hi)
    }

    /// The causal [`Span`] enclosing `self` and all the [`Version`]s in `iter`.
    ///
    /// This computes the tightest [`Span`] which encloses all [`Version`]s `v`
    /// such that `self.meet_all(iter) <= v <= self.join_all(iter)`.
    ///
    /// Prefer this to repeatedly calling [`span`](Version::span) one pair at a
    /// time, as it is more efficient.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_span_all.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
    )]
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
        // before the counter reads them: both hull directions are idempotent,
        // so a run of one shared buffer is one input.
        let inputs = crate::fold::dedup_runs(self.with_items(iter), same_buffer).map(Hull::Input);
        let group = crate::fold::balanced_reduce(inputs, |a, b| {
            let (lo, hi) = match (a, b) {
                // A leaf combine: two raw inputs derive their pair hull in one
                // fused walk.
                (Hull::Input(a), Hull::Input(b)) => a.version().hull(b.version()),
                (Hull::Merged { mut lo, mut hi }, Hull::Input(b)) => {
                    let b = b.version();
                    lo &= b;
                    hi |= b;
                    (lo, hi)
                }
                (
                    Hull::Merged {
                        lo: mut a_lo,
                        hi: mut a_hi,
                    },
                    Hull::Merged { lo: b_lo, hi: b_hi },
                ) => {
                    a_lo &= &b_lo;
                    a_hi |= &b_hi;
                    (a_lo, a_hi)
                }
                // Unreachable through the counter's weight discipline (a
                // weight-0 lone input never sits below a merged group in the
                // closing drain), but the match stays total rather than
                // asserting: both sides' combiners are commutative, so folding
                // the raw input into the owned hull is value-identical.
                (Hull::Input(a), Hull::Merged { mut lo, mut hi }) => {
                    let a = a.version();
                    lo &= a;
                    hi |= a;
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
        OwnVersion {
            party,
            version: self,
        }
    }

    /// The version's shape.
    ///
    /// The iterator yields its step function over the interval `[0, 1)`
    /// as an iterator of [`Plateau`](crate::shape::Plateau)s, left to right.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_shape.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)` to drain")]
    ///
    /// Draining the iterator is linear in the version's size:
    /// each plateau costs `O(1)` plus its own rise's bit width, and
    /// the walk itself performs no arithmetic.
    ///
    /// Arithmetic performed by the caller is separate from the cost of the
    /// walk. [`Count`] addition costs the operands' widths, so reconstructing
    /// every absolute height by repeatedly adding rises can be quadratic in the
    /// worst case even though draining the iterator itself is linear.
    ///
    /// # Example
    ///
    /// ```
    /// use before::shape::{Plateau, Rise};
    /// use before::{Clock, Count};
    ///
    /// let mut left = Clock::seed();
    /// let mut right = left.fork();
    /// left.tick();
    /// left.tick();
    /// right.tick();
    /// left.sync(&mut right).unwrap();
    /// let version = left.version();
    /// let plateaus: Vec<Plateau> = version.shape().collect();
    /// assert_eq!(
    ///     plateaus,
    ///     vec![
    ///         // The left half is at height 2; the first rise is absolute.
    ///         Plateau { rise: Some(Rise::Up(Count::from(2u64))), depth: 1 },
    ///         // The right half is at height 1.
    ///         Plateau { rise: Some(Rise::Down(Count::from(1u64))), depth: 1 },
    ///     ],
    /// );
    /// // Widths tile the unit interval: 1/2 + 1/2 = 1.
    /// let total: f64 = plateaus.iter().map(|p| 0.5f64.powi(p.depth as i32)).sum();
    /// assert_eq!(total, 1.0);
    /// ```
    pub fn shape(&self) -> crate::shape::Plateaus<'_> {
        crate::shape::Plateaus::of_version(self)
    }

    /// Reduce borrowed or owned versions to their pointwise `extreme` with a
    /// balanced fold: the join for [`Extreme::Higher`], the meet for
    /// [`Extreme::Lower`].
    ///
    /// [`Group`] distinguishes an untouched input, still in the caller's form,
    /// from an owned intermediate result. Two untouched inputs combine into a
    /// new value, and every later combine updates an owned result in place, so
    /// no input's bytes are copied. Empty input returns `None`; one input
    /// returns a shared-buffer clone so the result is owned.
    ///
    /// [`dedup_runs`](crate::fold::dedup_runs) removes adjacent inputs sharing
    /// the same buffer. Join and meet are idempotent, so retaining one from
    /// each run preserves the result.
    fn balanced_fold<I>(iter: I, extreme: Extreme) -> Option<Version>
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        let inputs = crate::fold::dedup_runs(iter, same_buffer);
        let group = crate::fold::balanced_reduce(inputs.map(Group::Input), |a, b| {
            Group::Merged(match (a, b) {
                (Group::Input(a), Group::Input(b)) => a.borrow().extreme(extreme, b.borrow()),
                (Group::Merged(mut a), Group::Input(b)) => {
                    a.assign_extreme(extreme, b.borrow());
                    a
                }
                (Group::Merged(mut a), Group::Merged(b)) => {
                    a.assign_extreme(extreme, b);
                    a
                }
                // Commutativity lets either operand supply the owned result.
                (Group::Input(a), Group::Merged(mut b)) => {
                    b.assign_extreme(extreme, a.borrow());
                    b
                }
            })
        })?;
        Some(match group {
            Group::Input(input) => input.borrow().clone(),
            Group::Merged(version) => version,
        })
    }

    /// Prepend this version to the input iterator, ensuring a nonempty fold.
    fn with_items<I>(&self, iter: I) -> impl Iterator<Item = FoldInput<'_, I::Item>>
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        core::iter::once(FoldInput::Receiver(self)).chain(iter.into_iter().map(FoldInput::Item))
    }

    /// Return the meet and join of two versions as their enclosing endpoints.
    ///
    /// Equality and empty inputs determine both endpoints immediately. If the
    /// inputs are causally comparable, their meet is the lesser input and their
    /// join the greater, so a comparison allows both buffers to be shared
    /// without building either endpoint. Equal endpoints share one buffer too.
    ///
    /// Concurrent inputs require new endpoints. The comparison stops as soon
    /// as it sees a region supporting each direction of order; a second,
    /// complete walk then builds the meet and join together, decoding each
    /// input once for both outputs.
    pub(crate) fn hull(&self, other: &Version) -> (Version, Version) {
        if self == other {
            return (self.clone(), self.clone());
        }
        if self.is_empty() {
            // The empty version is the meet and the other version is the join.
            return (self.clone(), other.clone());
        }
        if other.is_empty() {
            return (Version::new(), self.clone());
        }
        match self.partial_cmp(other) {
            Some(Ordering::Less) => {
                return (self.clone(), other.clone());
            }
            Some(Ordering::Greater) => {
                return (other.clone(), self.clone());
            }
            Some(Ordering::Equal) => unreachable!(
                "equal versions have byte-equal canonical streams, handled by the initial equality check"
            ),
            None => {}
        }
        let hull = self.hull_bits(other);
        // The emitting walk also computes causal order from comparisons needed
        // for its outputs, providing an independent check of the earlier result.
        debug_assert!(
            hull.relation.is_none(),
            "only concurrent pairs reach the endpoint-building walk"
        );
        (hull.lo, hull.hi)
    }

    /// Encodes this [`Version`] to bytes.
    ///
    /// Prefer [`as_bytes`](Version::as_bytes) to get a reference to the
    /// underlying encoding without cloning it.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_encode.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
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
    /// # Errors
    ///
    /// Returns any error reported by the writer.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_encode.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Version;
    /// let mut buf = Vec::new();
    /// Version::new().encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Version::new().encode());
    /// ```
    pub fn encode_to<W: Write>(&self, writer: &mut W) -> IoResult<()> {
        writer.write_all(self.as_bytes())
    }

    /// Encodes this [`Version`]'s [`Rank`] to bytes.
    ///
    /// This produces the same bytes as `self.rank().encode()` and
    /// `self.ranked().encode_rank()`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_encode_rank.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
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
    /// This writes the same bytes as `self.rank().encode_to(writer)` and
    /// `self.ranked().encode_rank_to(writer)`.
    /// The encoded output is written incrementally rather than buffered in
    /// full.
    ///
    /// # Errors
    ///
    /// Returns any error reported by the writer.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_encode_rank.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::Version;
    /// let mut buf = Vec::new();
    /// Version::new().encode_rank_to(&mut buf).unwrap();
    /// assert_eq!(buf, Version::new().rank().encode());
    /// ```
    pub fn encode_rank_to<W: Write>(&self, writer: &mut W) -> IoResult<()> {
        self.ranked().encode_rank_to(writer)
    }

    /// Decodes one [`Version`] from a reader.
    ///
    /// A successful decode requires exactly one canonical encoding; bytes after
    /// its padding are an error.
    ///
    /// # Errors
    ///
    /// - [`Decode::Truncated`] if the tree, an integer, or the padding is
    ///   incomplete;
    /// - [`Decode::TrailingBits`] if the padding is malformed or followed by
    ///   more bytes;
    /// - [`Decode::NotCanonical`] if the tree is not in normal form;
    /// - [`Decode::Io`] if the reader fails.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_decode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(n)`, `n` the bytes read, accepted or rejected"
    )]
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
        Version::decode_bytes(buf.into())
    }

    /// The exact length in bits of [`encode`](Self::encode) before its
    /// padding — the marker bit and zero-pad to the byte boundary, so
    /// `encode().len()` is `(encoded_bits() + 1).div_ceil(8)`.
    ///
    /// This method is available under the `meter` feature for exact
    /// representation measurements. Applications ordinarily want
    /// [`as_bytes`](Self::as_bytes)`.len()`, the number of bytes written to the
    /// wire.
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
        self.0.reader().len()
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
            self.0.has_canonical_padding(),
            "non-canonical Version storage: the bytes must end in the `1 0*` padding",
        );
        self.0.as_raw_slice()
    }
}

/// Whether two fold inputs contribute one shared stored buffer.
///
/// Join and meet are idempotent, so a run of clones contributes only one
/// operand. A clone shares its source's buffer, so comparing buffer address
/// and length (`ptr_eq`) identifies it in `O(1)` without reading either
/// stream. Equal addresses prove shared storage, and so equal values, only
/// while both buffers are alive, because a new buffer can reuse a freed one's
/// address; [`dedup_runs`](crate::fold::dedup_runs) compares only items it
/// still holds.
fn same_buffer<B: Borrow<Version>>(a: &B, b: &B) -> bool {
    a.borrow().ptr_eq(b.borrow())
}

/// One entry in [`Version::balanced_fold`]: either an untouched input or an
/// owned intermediate result.
///
/// Untouched inputs retain the caller's owned or borrowed form. Combined
/// entries own their result, allowing later steps to update that allocation in
/// place.
enum Group<B> {
    /// An input the fold has not yet combined, still in the caller's form.
    Input(B),
    /// The owned running result of one or more combines.
    Merged(Version),
}

/// One input to a receiver-seeded fold.
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

/// One entry in [`Version::span_all`]'s balanced fold.
///
/// Intermediate entries carry both endpoints so one reduction computes the
/// meet and join together.
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
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
)]
///
/// Auxiliary space is `O(|iter|)`.
impl Sum<Version> for Version {
    fn sum<I: Iterator<Item = Version>>(iter: I) -> Version {
        Version::balanced_fold(iter, Extreme::Higher).unwrap_or_default()
    }
}

/// Joins the iterator's versions; the empty sum is [`Version::new`].
///
/// # Complexity
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
)]
///
/// Auxiliary space is `O(|iter|)`.
impl<'a> Sum<&'a Version> for Version {
    fn sum<I: Iterator<Item = &'a Version>>(iter: I) -> Version {
        Version::balanced_fold(iter, Extreme::Higher).unwrap_or_default()
    }
}

/// Collects by joining; the empty collection is [`Version::new`].
///
/// # Complexity
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
)]
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
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join_all.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
)]
///
/// Auxiliary space is `O(|iter|)`.
impl<'a> FromIterator<&'a Version> for Version {
    fn from_iter<I: IntoIterator<Item = &'a Version>>(iter: I) -> Version {
        iter.into_iter().sum()
    }
}

/// Shows the version as `Version(0b…)` using its binary encoding.
impl Debug for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Version({:#b})", self.0)
    }
}

// The join (`|`, `|=`) and meet (`&`, `&=`) matrices over owned and borrowed
// `Version` operands, duals of each other, mirroring the comparison matrix
// below. Each family has four value-operator cells (lhs × rhs over {Version,
// &Version}) and two assign cells (rhs over {Version, &Version}).
//
// Every cell delegates to `Outcome::of`, which holds the one short-circuit
// ladder and the one call into the lattice kernel for both families, as do the
// named methods `Version::join` and `Version::meet` and the balanced folds
// (`Version::balanced_fold`). A cell with an owned left operand updates it in
// place through `Version::assign_extreme`: the assign cells and the `own` value
// cells, which move their left operand. A cell with a borrowed left operand
// (`read`) builds a new value through `Version::extreme`. Both take the right
// operand in the caller's form, so when the result is the right operand, an
// owned one moves into the result and only a borrowed one is cloned. The
// borrowed assign cell (`Version |= &Version`, `Version &= &Version`) is
// written out by hand; the `binop_matrix!` macro generates the rest.

/// Generates one binary-operator family's cells over owned and borrowed
/// `Version` operands, apart from the hand-written borrowed assignment.
///
/// Parameterized over the value operator `$Op::$op` (e.g. `BitOr::bitor`), its
/// assigning form `$Assign::$assign` (e.g. `BitOrAssign::bitor_assign`), and
/// the family's pointwise [`Extreme`]. Each strategy — `own`/`read` for value
/// cells and `assign` for an owned right-hand side — has its own `@cell` arm so
/// the receiver `self` is written in the same expansion as the method it
/// belongs to (`self` cannot cross a macro-invocation boundary).
macro_rules! binop_matrix {
    ($island:literal, $contract:literal, $opdoc:literal, $Op:ident::$op:ident, $Assign:ident::$assign:ident, $extreme:ident;
     $($lhs:ty, $rhs:ty, $strat:tt);* $(;)?
    ) => {
        $( binop_matrix!(@cell $island, $contract, $opdoc, $Op::$op, $Assign::$assign, $extreme, $lhs, $rhs, $strat); )*
    };
    (@cell $island:literal, $contract:literal, $opdoc:literal, $Op:ident::$op:ident, $Assign:ident::$assign:ident, $extreme:ident, $lhs:ty, $rhs:ty, own) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html")))]
        #[cfg_attr(not(doc), doc = $contract)]
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                let mut out: Version = self;
                out.assign_extreme(Extreme::$extreme, r);
                out
            }
        }
    };
    (@cell $island:literal, $contract:literal, $opdoc:literal, $Op:ident::$op:ident, $Assign:ident::$assign:ident, $extreme:ident, $lhs:ty, $rhs:ty, read) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html")))]
        #[cfg_attr(not(doc), doc = $contract)]
        impl $Op<$rhs> for $lhs {
            type Output = Version;
            fn $op(self, r: $rhs) -> Version {
                self.extreme(Extreme::$extreme, r)
            }
        }
    };
    (@cell $island:literal, $contract:literal, $opdoc:literal, $Op:ident::$op:ident, $Assign:ident::$assign:ident, $extreme:ident, $lhs:ty, $rhs:ty, assign) => {
        #[doc = $opdoc]
        #[doc = ""]
        #[doc = "# Complexity"]
        #[doc = ""]
        #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html")))]
        #[cfg_attr(not(doc), doc = $contract)]
        impl $Assign<$rhs> for $lhs {
            fn $assign(&mut self, r: $rhs) {
                self.assign_extreme(Extreme::$extreme, r);
            }
        }
    };
}

/// Assigns the causal join with a borrowed version.
///
/// Empty and equal inputs reuse an existing buffer. Otherwise one pass emits
/// the joined version directly from the two inputs.
impl BitOrAssign<&Version> for Version {
    fn bitor_assign(&mut self, incoming: &Version) {
        self.assign_extreme(Extreme::Higher, incoming);
    }
}

/// Assigns the causal meet with a borrowed version.
///
/// Empty and equal inputs reuse an existing buffer. Otherwise one pass emits
/// the met version directly from the two inputs.
impl BitAndAssign<&Version> for Version {
    fn bitand_assign(&mut self, incoming: &Version) {
        self.assign_extreme(Extreme::Lower, incoming);
    }
}

// The remaining join (`|`, `|=`) cells.
binop_matrix! {
    "version_join",
    "`O(n)` in total input bytes; `O(|self| + |other|)`",
    "`a | b` and `a |= b`: the causal join, the operator matrix of [`Version::join`] over owned and borrowed operands.",
    BitOr::bitor, BitOrAssign::bitor_assign, Higher;
    // value operator: an owned left operand is updated in place, a borrowed
    // one is read to build a new value
    Version,  Version,  own;
    Version,  &Version, own;
    &Version, Version,  read;
    &Version, &Version, read;
    // assign: right operand folded into the left operand in place
    Version,  Version,  assign;
}

// The remaining meet (`&`, `&=`) cells are its dual.
binop_matrix! {
    "version_meet",
    "`O(n)` in total input bytes; `O(|self| + |other|)`",
    "`a & b` and `a &= b`: the causal meet, the operator matrix of [`Version::meet`] over owned and borrowed operands.",
    BitAnd::bitand, BitAndAssign::bitand_assign, Lower;
    // value operator: an owned left operand is updated in place, a borrowed
    // one is read to build a new value
    Version,  Version,  own;
    Version,  &Version, own;
    &Version, Version,  read;
    &Version, &Version, read;
    // assign: right operand folded into the left operand in place
    Version,  Version,  assign;
}

/// The pointwise extreme of two versions, named by the existing buffer it can
/// reuse.
///
/// [`Outcome::of`] holds the one short-circuit ladder of the join and the meet,
/// and their one call into the lattice kernel. Every join and meet entry point
/// maps the outcome to its result, cloning a borrowed operand only when the
/// result *is* that operand, and moving an owned right operand instead of
/// cloning it. That rule matters for allocation, not only for time: a buffer
/// adopted from an exactly filled `Vec` allocates its shared reference count on
/// its first clone, so a clone taken before the ladder would allocate even on
/// paths that discard it, and a clone of an owned operand allocates only to
/// drop the original.
enum Outcome {
    /// The result is the left operand.
    Left,
    /// The result is the right operand.
    Right,
    /// The result is the empty version, built as [`Version::new`]'s static
    /// buffer rather than shared with the right operand.
    Empty,
    /// The result is a new version the kernel emitted.
    Emitted(Version),
}

impl Outcome {
    /// Decide the pointwise `extreme` of `lhs` and `rhs`, running the kernel
    /// only when neither operand is already the result.
    ///
    /// [`Extreme::Higher`] is the join and [`Extreme::Lower`] the meet. The
    /// empty version is the join's identity and the meet's absorbing element,
    /// and both operations are idempotent, so only distinct nonempty operands
    /// reach the kernel.
    fn of(extreme: Extreme, lhs: &Version, rhs: &Version) -> Outcome {
        match (extreme, lhs.is_empty(), rhs.is_empty()) {
            (Extreme::Higher, _, true) | (Extreme::Lower, true, _) => Outcome::Left,
            (Extreme::Higher, true, false) => Outcome::Right,
            (Extreme::Lower, false, true) => Outcome::Empty,
            (_, false, false) if lhs == rhs => Outcome::Left,
            (_, false, false) => Outcome::Emitted(extreme.emit(lhs, rhs)),
        }
    }
}

impl Version {
    /// Return the pointwise `extreme` of `self` and `other` as a new value,
    /// sharing an operand's buffer when that operand is the result.
    ///
    /// `other` arrives in the caller's form: when it is the result, an owned
    /// operand moves into the result and a borrowed one is cloned.
    fn extreme<'r>(&self, extreme: Extreme, other: impl Into<Cow<'r, Version>>) -> Version {
        let other = other.into();
        match Outcome::of(extreme, self, &other) {
            Outcome::Left => self.clone(),
            Outcome::Right => other.into_owned(),
            Outcome::Empty => Version::new(),
            Outcome::Emitted(version) => version,
        }
    }

    /// Replace `self` with the pointwise `extreme` of `self` and `other`,
    /// leaving it untouched when it is already the result.
    ///
    /// `other` arrives in the caller's form, as in [`extreme`](Self::extreme).
    fn assign_extreme<'r>(&mut self, extreme: Extreme, other: impl Into<Cow<'r, Version>>) {
        let other = other.into();
        match Outcome::of(extreme, self, &other) {
            Outcome::Left => {}
            Outcome::Right => *self = other.into_owned(),
            Outcome::Empty => *self = Version::new(),
            Outcome::Emitted(version) => *self = version,
        }
    }
}

// ───────────────────────── the pair hull (`^`) ─────────────────────────
//
// `a ^ b` is `a.span(&b)`: the tightest `Span` containing both operands, `[a &
// b, a | b]`. Unlike the join and meet matrices above, the result leaves the
// operand type — a `Span`, not a `Version` — so the family has no assigning
// form (nothing of the receiver's type to assign back) and no owned-operand
// strategy: every cell reads both operands in place and constructs the endpoints
// owned, exactly as the named method does.

/// Generates the span (`^`) matrix over owned and borrowed `Version`
/// operands.
///
/// Every cell delegates to [`Version::span`]; `Borrow::borrow` coerces an owned
/// or borrowed operand uniformly to `&Version`, so one arm covers all four
/// cells.
macro_rules! span_matrix {
    ($island:literal, $contract:literal, $opdoc:literal, $($lhs:ty, $rhs:ty);* $(;)?) => {
        $(
            #[doc = $opdoc]
            #[doc = ""]
            #[doc = "# Complexity"]
            #[doc = ""]
            #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html")))]
            #[cfg_attr(not(doc), doc = $contract)]
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
    "`O(n)` in total input bytes; `O(|self| + |other|)`",
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
// [`OwnVersion`] in O(1); comparisons operate on it directly, and only the
// explicit [`OwnVersion::to_version`] pays the
// projection's product-growth materialization.
//
// Projection preserves causal order and distributes over join and meet. It is
// also additive across a party split: projecting onto two disjoint children
// and joining the results recovers the projection onto their parent. These
// properties make the lower and upper projections of a Span remain ordered.
// Projection may nevertheless raise `min_ticks`, because carving one broad
// event into disjoint peaks can require more events to explain.

/// `&v / &p`: the part of the [`Version`] `v` contributed within
/// the region owned by [`Party`] `p` (zero everywhere else), as a lazy
/// [`OwnVersion`].
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
        self.project(party)
    }
}

/// Compares canonical version bytes, which uniquely encode causal equality.
impl PartialEq<Version> for Version {
    fn eq(&self, other: &Version) -> bool {
        self.0 == other.0
    }
}

/// Compares an owned version with a borrowed version by canonical bytes.
impl PartialEq<&Version> for Version {
    fn eq(&self, other: &&Version) -> bool {
        self == *other
    }
}

/// Compares a borrowed version with an owned version by canonical bytes.
impl PartialEq<Version> for &Version {
    fn eq(&self, other: &Version) -> bool {
        *self == other
    }
}

/// Applies the canonical causal comparison to a borrowed right operand.
impl PartialOrd<&Version> for Version {
    fn partial_cmp(&self, other: &&Version) -> Option<Ordering> {
        self.partial_cmp(*other)
    }

    fn lt(&self, other: &&Version) -> bool {
        self.causal_lt(other)
    }

    fn le(&self, other: &&Version) -> bool {
        self.causal_le(other)
    }

    fn gt(&self, other: &&Version) -> bool {
        other.causal_lt(self)
    }

    fn ge(&self, other: &&Version) -> bool {
        other.causal_le(self)
    }
}

/// Applies the canonical causal comparison to a borrowed left operand.
impl PartialOrd<Version> for &Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        (*self).partial_cmp(other)
    }

    fn lt(&self, other: &Version) -> bool {
        self.causal_lt(other)
    }

    fn le(&self, other: &Version) -> bool {
        self.causal_le(other)
    }

    fn gt(&self, other: &Version) -> bool {
        other.causal_lt(self)
    }

    fn ge(&self, other: &Version) -> bool {
        other.causal_le(self)
    }
}
