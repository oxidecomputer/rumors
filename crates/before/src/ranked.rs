//! A total-ordering view of a [`Version`].

use core::cmp::Ordering;
use std::borrow::Cow;
use std::io::{self, Read, Write};

use crate::error::Decode;
use crate::{Rank, Version};

/// A [`Version`] viewed through a deterministic total causal order.
///
/// [`Version`]s are ordered first by [`Rank`]. Because rank strictly increases
/// with causal order, causes always precede their effects. Distinct [`Version`]s
/// can have equal [`Rank`]s; their canonical bytes provide the tiebreak.
///
/// Construction borrows or takes the version in `O(1)`. Comparison computes
/// the rank difference directly from both versions, without first allocating
/// either [`Rank`]. For a large sort or persistent index, prefer
/// [`encode`](Self::encode): its byte-wise lexicographic order is exactly the
/// same total order, and materializing each key once avoids repeating the rank
/// traversal on every comparison.
///
/// The encoded key consists of the self-delimiting rank followed by the
/// version's canonical bytes. [`Ranked::decode`] recovers the version.
/// [`Ranked::encode_rank`] writes only the rank component when the caller wants
/// equal ranks to remain equal and will supply a different tiebreak.
///
/// # Example
///
/// ```
/// use before::{Clock, Ranked};
/// let mut half_clock = Clock::seed();
/// let _other_half = half_clock.fork();
/// let half = half_clock.tick().clone();
/// let one = Clock::seed().tick().clone();
/// // Borrowing views: no fold has run yet.
/// let (rh, ro) = (Ranked::from(&half), Ranked::from(&one));
/// assert!(rh < ro); // compares the ranks without building either Rank
/// // The rank question is explicit: materialize, then compare ranks.
/// assert!(rh.rank() < one.rank());
/// // The composite key sorts exactly as the views compare.
/// let (kh, ko) = (rh.encode(), ro.encode());
/// assert!(kh < ko);
/// assert_eq!(Ranked::decode(&kh[..]).unwrap().version(), &half);
/// ```
#[derive(Clone)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_rank.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Ranked, Version};
    /// let mut v = Version::new();
    /// Party::seed().ticks(&mut v, 3u8);
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
    /// use before::{Party, Ranked, Version};
    /// let owned: Ranked<'static> = {
    ///     let mut v = Version::new();
    ///     Party::seed().ticks(&mut v, 2u8);
    ///     Ranked::from(&v).into_owned() // outlives the borrow
    /// };
    /// assert_eq!(owned.rank().to_string(), "10");
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
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_encode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Ranked, Version};
    /// let mut v = Version::new();
    /// Party::seed().ticks(&mut v, 5u8);
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
    /// The rank prefix and version suffix are written incrementally rather than
    /// buffered as one composite key.
    ///
    /// # Errors
    ///
    /// Returns any error reported by the writer.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_encode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Ranked, Version};
    /// let mut v = Version::new();
    /// Party::seed().ticks(&mut v, 5u8);
    /// let mut buf = Vec::new();
    /// Ranked::from(&v).encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Ranked::from(&v).encode());
    /// ```
    pub fn encode_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.encode_rank_to(writer)?;
        self.version.encode_to(writer)
    }

    /// Encodes the rank's canonical order-preserving bytes alone, without the
    /// trailing version component.
    ///
    /// In other words, for some version `v`, these are all equivalent:
    ///
    /// - `v.ranked().encode_rank()`
    /// - `v.rank().encode()`
    /// - `v.encode_rank()`
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
    /// use before::{Party, Ranked, Version};
    /// let mut v = Version::new();
    /// Party::seed().ticks(&mut v, 5u8);
    /// assert_eq!(Ranked::from(&v).encode_rank(), v.rank().encode());
    /// ```
    pub fn encode_rank(&self) -> Vec<u8> {
        self.version.rank().encode()
    }

    /// Encodes the rank's canonical bytes to an arbitrary writer.
    ///
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
    /// use before::{Party, Ranked, Version};
    /// let mut v = Version::new();
    /// Party::seed().ticks(&mut v, 5u8);
    /// let mut buf = Vec::new();
    /// Ranked::from(&v).encode_rank_to(&mut buf).unwrap();
    /// assert_eq!(buf, Ranked::from(&v).encode_rank());
    /// ```
    pub fn encode_rank_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.version.rank().encode_to(writer)
    }

    /// Decodes one owned view from a reader.
    ///
    /// A successful decode requires exactly one canonical encoding; trailing
    /// bytes are an error.
    ///
    /// # Errors
    ///
    /// - [`Decode::Truncated`], [`Decode::TrailingBits`], or
    ///   [`Decode::NotCanonical`] when either component has the corresponding
    ///   defect described by [`Rank::decode`] or [`Version::decode`];
    /// - [`Decode::NotCanonical`] when the rank and version disagree;
    /// - [`Decode::Io`] when the reader fails.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_decode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{error::Decode, Party, Ranked, Version};
    /// let mut v = Version::new();
    /// Party::seed().ticks(&mut v, 5u8);
    /// let key = Ranked::from(&v).encode();
    /// let decoded = Ranked::decode(&key[..]).unwrap();
    /// assert_eq!(decoded.version(), &v);
    /// // A rank prefix the version does not measure is non-canonical.
    /// let mut other = Version::new();
    /// Party::seed().ticks(&mut other, 6u8);
    /// let mut forged = other.rank().encode();
    /// forged.extend_from_slice(v.as_bytes());
    /// assert!(matches!(
    ///     Ranked::decode(&forged[..]),
    ///     Err(Decode::NotCanonical)
    /// ));
    /// ```
    pub fn decode<R: Read>(mut reader: R) -> Result<Ranked<'static>, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        Self::decode_bytes(buf.into())
    }

    /// Validates an owned canonical key and adopts its version bytes.
    pub(crate) fn decode_bytes(buf: bytes::Bytes) -> Result<Ranked<'static>, Decode> {
        // The rank stream is self-delimiting: consume exactly its bytes.
        let mut consumed = 0usize;
        Rank::decode_stream(|| {
            let byte = buf.get(consumed).copied().ok_or(Decode::Truncated)?;
            consumed += 1;
            Ok(byte)
        })?;
        let version = Version::decode_bytes(buf.slice(consumed..))?;
        if !version.rank().encoding_matches(&buf[..consumed]) {
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

/// Compares version identity, matching [`Ord`] and [`Hash`](core::hash::Hash).
impl PartialEq<Ranked<'_>> for Ranked<'_> {
    fn eq(&self, other: &Ranked<'_>) -> bool {
        self.version() == other.version()
    }
}

/// Marks version identity as an equivalence relation.
impl Eq for Ranked<'_> {}

/// Delegates to the viewed version's byte hash: consistent with [`Eq`] (version
/// identity), and equal to the [`Version`]'s own hash.
impl core::hash::Hash for Ranked<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.version().hash(state);
    }
}

/// Orders by rank, then by canonical version bytes on rank ties.
///
/// # Complexity
///
/// With `n = |self| + |other|`, where each size is its version's size in bytes,
/// comparison uses `O(M(n))` time and `O(n)` transient space, where
/// `M(n)` is the cost of multiplying `n`-bit integers. Exact rank ties require
/// resolving the complete rank difference, so they have the same worst-case
/// arithmetic as computing a rank.
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_cmp.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is the cost of multiplying `n`-bit integers"
)]
impl Ord for Ranked<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Identity is the common cheap case. Without this check, equal versions
        // would be traversed once to establish equal rank and again for the
        // byte tiebreak.
        if self.version == other.version {
            return Ordering::Equal;
        }
        self.version
            .rank_cmp(&other.version)
            .then_with(|| self.version.as_bytes().cmp(other.version.as_bytes()))
    }
}

/// Returns the total comparison supplied by [`Ord`].
impl PartialOrd<Ranked<'_>> for Ranked<'_> {
    fn partial_cmp(&self, other: &Ranked<'_>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
