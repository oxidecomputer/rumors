//! The rank view: [`Ranked`], a [`Version`] ordered totally by its causal
//! [`Rank`], with fused comparisons and a composite key encoding. The public
//! contract lives on the type; this module is private.

use core::cmp::Ordering;
use std::borrow::Cow;
use std::io::{self, Read, Write};

use super::rank::{decode_stream, encode_parts};
use super::{skyline, Rank, Version};
use crate::error::Decode;

/// A [`Version`] as a total causal-ordering key.
///
/// This is a view on a [`Version`] which is ordered by its causal [`Rank`] with
/// a deterministic tiebreak, equal only to itself, with a canonical encoding
/// whose lexicographic order aligns with its [`Ord`] implementation.
///
/// Construction is `O(1)` and borrows (or takes) the version. Two `Ranked`
/// values compare with no intermediate [`Rank`] value materialized, which is
/// cheaper than two rank folds and a compare, because nothing is allocated
/// beyond the walk's accumulators.
///
/// # The total order
///
/// Rank first: causally ordered versions compare as causality does, since
/// [`Rank`] is strictly monotone. Equal ranks never derive from [`Version`]s
/// which are causally ordered, so the tiebreak to achieve a total order is a
/// free choice. In this case, we say that [`Rank`]-equal distinct versions are
/// ordered by a fixed, deterministic comparison of the canonical bytes of their
/// contained [`Version`]s.
///
/// # Using it as a lexicographic causal key
///
/// [`encode`](Self::encode) emits the rank's self-delimiting order-preserving
/// serialization, followed by the version's canonical bytes
/// ([`Version::as_bytes`]), which builds the tiebreak directly into the output.
/// Byte-wise lexicographic order on these keys **equals [`Ord`] on the views**:
/// byte equality is exactly [`Eq`], and the order survives any appended suffix,
/// because both components are prefix-free.
///
/// A sorted KV store keyed by this composite delivers causes before effects
/// with no rank-aware comparator, needs no further tiebreak suffix, and can
/// recover the stored version from the key alone via [`decode`](Self::decode).
///
/// If you want to group by rank-*class* instead, with all rank-equal keys
/// collapsing to one, key by [`Rank::encode`] plus a tiebreak of your own
/// choosing; [`encode_rank`](Self::encode_rank) emits exactly the corresponding
/// [`Rank`]'s encoded bytes without materializing the intermediate [`Rank`].
///
/// # Cost shape
///
/// Sorting *many* keys re-walks both versions per comparison: for a sorted
/// container or a one-shot sort over `n` versions, materialize each key once
/// rather than paying a traversal per probe; the view's comparisons win where a
/// handful of verdicts is all you need.
///
/// # Example
///
/// ```
/// use before::{Ranked, Version};
/// let half: Version = "(0, 1, 0)".parse().unwrap();
/// let one = Version::try_from(1).unwrap();
/// // Borrowing views: no fold has run yet.
/// let (rh, ro) = (Ranked::from(&half), Ranked::from(&one));
/// assert!(rh < ro); // one fused co-walk, no Rank built
/// // The rank question is explicit: materialize, then compare ranks.
/// assert!(rh.rank() < one.rank());
/// // Equal rank, distinct concurrent versions: ordered, never equal.
/// let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
/// assert_eq!(half.rank(), peaks.rank());
/// assert_ne!(Ranked::from(&half), Ranked::from(&peaks));
/// // The composite key sorts exactly as the views compare.
/// let (kh, ko) = (rh.encode(), ro.encode());
/// assert!(kh < ko);
/// assert_eq!(Ranked::decode(&kh[..]).unwrap().version(), &half);
/// ```
#[derive(Clone)]
pub struct Ranked<'a> {
    /// The version whose rank this view denotes; borrowed or owned
    /// ([`into_owned`](Self::into_owned) settles it to owned).
    version: Cow<'a, Version>,
}

impl<'a> Ranked<'a> {
    /// The version itself.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Materializes the represented rank.
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
    /// use before::{Ranked, Version};
    /// let v = Version::try_from(3).unwrap();
    /// assert_eq!(Ranked::from(&v).rank(), v.rank());
    /// ```
    pub fn rank(&self) -> Rank {
        self.version.rank()
    }

    /// Settles the view onto an owned [`Version`], erasing the borrow
    /// lifetime.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Ranked, Version};
    /// let owned: Ranked<'static> = {
    ///     let v = Version::try_from(2).unwrap();
    ///     Ranked::from(&v).into_owned() // outlives the borrow
    /// };
    /// assert_eq!(owned.rank().to_string(), "2");
    /// ```
    pub fn into_owned(self) -> Ranked<'static> {
        Ranked {
            version: Cow::Owned(self.version.into_owned()),
        }
    }

    /// Encodes the composite causal-ordering key: the rank's canonical
    /// encoding, then the version's own canonical bytes.
    ///
    /// Byte-wise lexicographic order on these keys **equals [`Ord`] on the
    /// views, ties included**, and byte equality is exactly [`Eq`]; see the
    /// [`Rank`] documentation for full detail.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_encode.html"))]
    ///
    /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Ranked, Version};
    /// let v = Version::try_from(5).unwrap();
    /// let key = Ranked::from(&v).encode();
    /// // The composite is the rank stream, then the version's bytes.
    /// let mut expect = v.rank().encode();
    /// expect.extend_from_slice(v.as_bytes());
    /// assert_eq!(key, expect);
    /// assert_eq!(Ranked::decode(&key[..]).unwrap().version(), &v);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = self.encode_rank();
        bytes.extend_from_slice(self.version.as_bytes());
        bytes
    }

    /// Encodes the composite key to an arbitrary writer.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_encode.html"))]
    ///
    /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Ranked, Version};
    /// let v = Version::try_from(5).unwrap();
    /// let mut buf = Vec::new();
    /// Ranked::from(&v).encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Ranked::from(&v).encode());
    /// ```
    pub fn encode_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.encode())
    }

    /// Encodes the rank's canonical order-preserving bytes alone, without the
    /// trailing version component.
    ///
    /// In other words, for some version `v`, these are all equivalent:
    ///
    /// - `v.ranked().encode_rank()`
    /// - `v.rank().encode()` (this one is less efficient)
    /// - `v.encode_rank()`
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
    /// use before::{Ranked, Version};
    /// let v = Version::try_from(5).unwrap();
    /// assert_eq!(Ranked::from(&v).encode_rank(), v.rank().encode());
    /// ```
    pub fn encode_rank(&self) -> Vec<u8> {
        let rank = self.version.rank();
        let (num, exp) = rank.raw_parts();
        encode_parts(num, exp)
    }

    /// Encodes the rank's canonical bytes to an arbitrary writer.
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
    /// use before::{Ranked, Version};
    /// let v = Version::try_from(5).unwrap();
    /// let mut buf = Vec::new();
    /// Ranked::from(&v).encode_rank_to(&mut buf).unwrap();
    /// assert_eq!(buf, Ranked::from(&v).encode_rank());
    /// ```
    pub fn encode_rank_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.encode_rank())
    }

    /// Decodes an owned view from a reader of canonical
    /// [`encode`](Self::encode)d bytes, strictly rejecting everything else.
    ///
    /// # Errors
    ///
    /// Each component's own genres ([`Rank::decode`]'s and
    /// [`Version::decode`]'s: [`Decode::Truncated`],
    /// [`Decode::TrailingBits`], [`Decode::NotCanonical`]);
    /// [`Decode::NotCanonical`] when the rank stream and the version
    /// disagree; [`Decode::Io`] when the reader itself fails.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_decode.html"))]
    ///
    /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{error::Decode, Ranked, Version};
    /// let v = Version::try_from(5).unwrap();
    /// let key = Ranked::from(&v).encode();
    /// let decoded = Ranked::decode(&key[..]).unwrap();
    /// assert_eq!(decoded.version(), &v);
    /// // A rank prefix the version does not measure is non-canonical.
    /// let mut forged = Version::try_from(6).unwrap().rank().encode();
    /// forged.extend_from_slice(v.as_bytes());
    /// assert!(matches!(
    ///     Ranked::decode(&forged[..]),
    ///     Err(Decode::NotCanonical)
    /// ));
    /// ```
    pub fn decode<R: Read>(mut reader: R) -> Result<Ranked<'static>, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        // The rank stream is self-delimiting: consume exactly its bytes.
        let mut consumed = 0usize;
        let rank = decode_stream(|| {
            let byte = buf.get(consumed).copied().ok_or(Decode::Truncated)?;
            consumed += 1;
            Ok(byte)
        })?;
        // The rest of the input is exactly the version (whole-input
        // strictness is Version::decode's own contract).
        let version = Version::decode(&buf[consumed..])?;
        // The two-ways pin on the wire: the key's rank component must
        // be the rank the version itself measures.
        if version.rank() != rank {
            return Err(Decode::NotCanonical);
        }
        Ok(Ranked::from(version))
    }
}

/// Views a borrowed version by its rank: `O(1)`, no fold, no copy.
impl<'a> From<&'a Version> for Ranked<'a> {
    fn from(version: &'a Version) -> Ranked<'a> {
        Ranked {
            version: Cow::Borrowed(version),
        }
    }
}

/// Views an owned version by its rank: `O(1)`, no fold.
impl From<Version> for Ranked<'static> {
    fn from(version: Version) -> Ranked<'static> {
        Ranked {
            version: Cow::Owned(version),
        }
    }
}

/// Materializes the rank, as [`rank`](Ranked::rank).
impl From<Ranked<'_>> for Rank {
    fn from(ranked: Ranked<'_>) -> Rank {
        ranked.rank()
    }
}

/// Renders the viewed version, tagged with the type's name; the rank is
/// derived state, so it is not (re)computed for a debug dump.
impl core::fmt::Debug for Ranked<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Ranked")
            .field("version", &self.version)
            .finish()
    }
}

/// The total comparison: the fused rank co-sweep, then the version-byte
/// tiebreak on rank ties.
///
/// The co-sweep is one signed walk over both packed streams, no `Rank`
/// materialized; the tiebreak runs only on rank ties, where the two sides are
/// never causally ordered, so it is causally free.
fn total_cmp(a: &Ranked<'_>, b: &Ranked<'_>) -> Ordering {
    // Equal versions are one value under the total order — the
    // `ranked_orders_by_rank_then_bytes` law in `crate::laws` pins the whole
    // table, equal ranks and equal bytes reading `Equal` — and canonical
    // equality answers in `O(1)` on a shared buffer (clone identity) or one
    // byte compare, where the rank co-sweep would fold both streams whole only
    // to tie and tiebreak `Equal`. Unequal operands pay only the compare's
    // early-exiting prefix.
    if crate::codec::canonical_eq(a.version.view(), b.version.view()) {
        return Ordering::Equal;
    }
    skyline::query::rank_cmp(a.version.view(), b.version.view())
        .then_with(|| a.version.as_bytes().cmp(b.version.as_bytes()))
}

// The comparison family is the total order: rank first, version bytes on rank
// ties, so `Equal` is version identity and `Eq`/`Ord`/`Hash` all speak about
// one value — the version. Equality itself never runs the walk: byte equality
// of canonical forms IS version identity, at one memcmp. `Ranked` compares only
// with `Ranked`: an `==` against a bare `Rank` would have to mean rank class on
// one side and version identity on the other, which cannot chain transitively —
// the type docs carry the explicit `rank` spelling of the rank question.
impl PartialEq<Ranked<'_>> for Ranked<'_> {
    fn eq(&self, other: &Ranked<'_>) -> bool {
        self.version() == other.version()
    }
}

impl Eq for Ranked<'_> {}

/// Delegates to the viewed version's byte hash: consistent with [`Eq`] (version
/// identity), and equal to the [`Version`]'s own hash.
impl core::hash::Hash for Ranked<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.version().hash(state);
    }
}

/// The total order: rank first, canonical bytes on rank ties.
///
/// # Complexity
///
/// One fused signed rank co-sweep over the two viewed versions:
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_cmp.html"))]
impl Ord for Ranked<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        total_cmp(self, other)
    }
}

impl PartialOrd<Ranked<'_>> for Ranked<'_> {
    fn partial_cmp(&self, other: &Ranked<'_>) -> Option<Ordering> {
        Some(total_cmp(self, other))
    }
}
