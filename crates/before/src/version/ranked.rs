//! The rank view: [`Ranked`], a [`Version`] ordered totally by its
//! causal [`Rank`], with fused comparisons and a composite key
//! encoding. The public contract lives on the type; this module is
//! private.

use core::cmp::Ordering;
use std::borrow::Cow;

use super::rank::{decode_stream, encode_parts};
use super::{skyline, Rank, Version};
use crate::error::Decode;

/// A [`Version`] as a total causal-ordering key.
///
/// This is a view on a [`Version`] which is ordered by its causal [`Rank`] with
/// a deterministic tiebreak, equal only to itself, with a canonical encoding
/// whose lexicographic order aligns with [`Ord`].
///
/// Construction is `O(1)` and borrows (or takes) the version — no fold runs
/// until something is asked. Two `Ranked` values compare in **one fused
/// co-walk** over both packed streams (the signed instance of the distance/lag
/// integrator), with no `Rank` value materialized: cheaper than two rank folds
/// and a compare, and nothing is allocated beyond the walk's accumulators. The
/// rank materializes only on request ([`to_rank`](Self::to_rank), or the
/// [`From`] impl).
///
/// # The total order
///
/// Rank first: causally ordered versions compare as causality does (rank is
/// strictly monotone — see [`Rank`]). Equal ranks are never causally ordered —
/// the two sides are the same version or concurrent — so the tiebreak that
/// completes the total order is causally free: rank-equal distinct versions are
/// ordered by a fixed, deterministic comparison of their canonical bytes. Which
/// of two rank-equal versions sorts first is deliberately **unspecified beyond
/// being stable**: the same verdict in every execution, on every replica,
/// carrying no causal meaning. **Equality is version identity** — distinct
/// concurrent versions of equal rank compare unequal (and non-`Equal` under
/// [`Ord`]), so no two distinct versions ever conflate under this order.
/// Equality never runs the walk: it is the versions' canonical byte comparison.
/// [`Hash`] delegates to the version's byte hash, so `Eq` and `Hash` agree (a
/// `Ranked` hashes exactly as the [`Version`] it views).
///
/// # The key encoding
///
/// [`encode`](Self::encode) emits the rank's self-delimiting order-preserving
/// stream ([`Rank::encode`]'s form) followed by the version's canonical bytes
/// ([`Version::as_bytes`]) — the tiebreak built into the key. Byte-wise
/// lexicographic order on these keys **equals [`Ord`] on the views, totally**:
/// byte equality is exactly [`Eq`], and the order survives any appended suffix
/// (both components are prefix-free, a committed pin). A sorted KV store keyed
/// by this composite delivers causes before effects with no rank-aware
/// comparator, needs no further tiebreak suffix, and can recover the stored
/// version from the key alone ([`decode`](Self::decode)). Where rank-*class*
/// semantics are wanted instead — all rank-equal keys collapsing to one — key
/// by [`Rank::encode`] plus a tiebreak of your own choosing;
/// [`encode_rank`](Self::encode_rank) emits exactly those bytes from the view,
/// fused.
///
/// # Comparing against a bare [`Rank`]
///
/// `Ranked` compares only with `Ranked`, and [`Rank`] only with `Rank`:
/// equality means something different on each side — version identity here,
/// rank equality there — and one rank class holds many versions, so no `==`
/// between the two types could satisfy [`PartialEq`]'s transitivity contract
/// (two rank-equal distinct views would each have to equal their shared rank
/// while comparing unequal to each other). Ask the rank question explicitly
/// instead: materialize with [`to_rank`](Self::to_rank) and compare ranks —
/// `a.to_rank() == k`, `a.to_rank().cmp(&k)` — one rank fold, then [`Rank`]'s
/// own comparison.
///
/// # Cost shape
///
/// Sorting *many* keys re-walks both versions per comparison: for a sorted
/// container or a one-shot sort over `n` versions, materialize each key
/// ([`encode`](Self::encode)) once — `n` folds — rather than paying a fused
/// walk per probe; the view's comparisons win where a handful of verdicts is
/// the whole job. Equality alone is one byte compare, no walk.
///
/// # Complexity
///
/// `O(a + b)` space; time `O(M(a + b) · log (a + b))` worst case, `O((a +
/// b) log (a + b))` with width-bounded parked drifts.
/// Construction and
/// [`version`](Self::version) are `O(1)`. [`to_rank`](Self::to_rank) and the
/// encodes are one rank fold ([`Version::rank`]'s three-part claim). A
/// comparison is the fused pair co-sweep at the distance/lag bound, plus — only
/// on rank ties — one byte comparison of the two versions. On answer-embedding
/// pairs the shipped co-sweep provably pays the backend's multiplication cost —
/// it settles the exact signed rank difference, whose value embeds an
/// input-funded product — but that is a fact about this walk, not a floor on
/// the comparison problem: [`Ord`] answers a three-valued verdict, not an exact
/// rank, so the answer-embedded-product reduction that floors [`Version::rank`]
/// cannot reach it, and whether some comparison can order such pairs below one
/// multiplication is open.
///
/// ```
/// use before::{Ranked, Version};
/// let half: Version = "(0, 1, 0)".parse().unwrap();
/// let one = Version::try_from(1).unwrap();
/// // Borrowing views: no fold has run yet.
/// let (rh, ro) = (Ranked::from(&half), Ranked::from(&one));
/// assert!(rh < ro); // one fused co-walk, no Rank built
/// // The rank question is explicit: materialize, then compare ranks.
/// assert!(rh.to_rank() < one.rank());
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

    /// Materializes the rank: one fold over the version's packed
    /// stream, exactly [`Version::rank`].
    ///
    /// # Complexity
    ///
    /// `O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)` with
    /// width-bounded parked drifts.
    /// As [`Version::rank`] (one rank fold), its proven `Ω(M(n))`
    /// lower bound included: the answer-embedded-product reduction
    /// floors any computation of the exact rank, and this method is
    /// exactly that computation.
    ///
    /// ```
    /// use before::{Ranked, Version};
    /// let v = Version::try_from(3).unwrap();
    /// assert_eq!(Ranked::from(&v).to_rank(), v.rank());
    /// ```
    pub fn to_rank(&self) -> Rank {
        self.version.rank()
    }

    /// Settles the view onto an owned [`Version`], erasing the borrow
    /// lifetime.
    ///
    /// An inherent method in [`Cow`]'s own vocabulary — a `ToOwned`
    /// impl cannot exist here, because std's blanket
    /// `impl<T: Clone> ToOwned for T` already claims every `Clone`
    /// type, `Ranked` included.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    /// A borrowed view settles by cloning the version, which shares its
    /// stored buffer; an owned one moves out.
    ///
    /// ```
    /// use before::{Ranked, Version};
    /// let owned: Ranked<'static> = {
    ///     let v = Version::try_from(2).unwrap();
    ///     Ranked::from(&v).into_owned() // outlives the borrow
    /// };
    /// assert_eq!(owned.to_rank().to_string(), "2");
    /// ```
    pub fn into_owned(self) -> Ranked<'static> {
        Ranked {
            version: Cow::Owned(self.version.into_owned()),
        }
    }

    /// Encodes the composite causal-ordering key: the rank's canonical
    /// self-delimiting stream, then the version's canonical bytes.
    ///
    /// Byte-wise lexicographic order on these keys **equals [`Ord`] on
    /// the views, ties included**, and byte equality is exactly [`Eq`]
    /// (the type docs carry the law and the KV-key use). The key
    /// determines the view: [`decode`](Self::decode) accepts exactly
    /// the byte strings this method produces and recovers the version.
    /// Both components are prefix-free, so distinct views' keys are
    /// never byte prefixes of one another and no appended suffix can
    /// flip an order (a committed pin).
    ///
    /// The rank stream is emitted straight from one rank fold's
    /// `(numerator, exponent)` output — byte-identical to
    /// `self.to_rank().encode()` (a committed law) with no second walk
    /// — and the version tail is one copy of the bytes at rest.
    ///
    /// # Complexity
    ///
    /// `O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)` with
    /// width-bounded parked drifts.
    /// One rank fold, an emission linear in the rank's numeric size
    /// (which the fold keeps linear in the packed input — the
    /// provenance pin on [`Rank::encode`]), and one byte copy of the
    /// version. The fold's proven `Ω(M(n))` lower bound applies whole:
    /// the emitted rank component determines the exact rank, so no
    /// encoder undercuts the answer-embedded-product reduction that
    /// floors [`Version::rank`].
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

    /// Encodes the composite key to an arbitrary writer: exactly
    /// [`encode`](Self::encode)'s bytes, without handing the caller the
    /// intermediate `Vec`.
    ///
    /// The key is assembled in one internal buffer and delivered in a
    /// single `write_all` — the writer sees exactly what `encode`
    /// returns.
    ///
    /// # Errors
    ///
    /// Whatever the writer itself reports; the encoding side is
    /// infallible.
    ///
    /// # Complexity
    ///
    /// `O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)` with
    /// width-bounded parked drifts.
    /// As [`encode`](Self::encode): one rank fold, a linear emission,
    /// and one byte copy of the version — the fold's proven `Ω(M(n))`
    /// lower bound included.
    ///
    /// ```
    /// use before::{Ranked, Version};
    /// let v = Version::try_from(5).unwrap();
    /// let mut buf = Vec::new();
    /// Ranked::from(&v).encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, Ranked::from(&v).encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.encode())
    }

    /// Encodes the rank's canonical order-preserving bytes alone —
    /// [`encode`](Self::encode)'s rank component without the version
    /// tail — straight from the rank fold, with no second walk
    /// anywhere.
    ///
    /// The format, the lexicographic law, and the rank-class KV-key
    /// use are [`Rank::encode`]'s contract; this spelling runs one walk
    /// over the packed version and one emission from the fold's
    /// `(numerator, exponent)` output, byte-identical to
    /// `self.to_rank().encode()` (a committed law). Fusing deeper than
    /// the fold is impossible in principle: the encoding's leading
    /// bits depend on the fold's final total, so no bit can be emitted
    /// until the walk completes.
    ///
    /// Decoding lands on [`Rank::decode`]: the bytes carry exactly the
    /// rank, so they identify a rank class, not a version — reach for
    /// them when rank-equal versions should share a key prefix under a
    /// tiebreak of your own; [`encode`](Self::encode) is the
    /// ready-made key when the version itself is the tiebreak.
    ///
    /// # Complexity
    ///
    /// `O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)` with
    /// width-bounded parked drifts.
    /// One rank fold plus an emission linear in the rank's numeric
    /// size, which the fold keeps linear in the packed input (the
    /// provenance pin on [`Rank::encode`]). The fold's proven
    /// `Ω(M(n))` lower bound applies whole: the emitted bytes
    /// determine the exact rank, so no emitter undercuts the
    /// answer-embedded-product reduction that floors
    /// [`Version::rank`].
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

    /// Encodes the rank's canonical bytes to an arbitrary writer:
    /// exactly [`encode_rank`](Self::encode_rank)'s bytes — the same
    /// one fused fold and emission — without handing the caller the
    /// intermediate `Vec`.
    ///
    /// The stream is bit-packed, so the bytes are assembled in one
    /// internal buffer and delivered in a single `write_all` (see
    /// [`Rank::encode_to`]).
    ///
    /// # Errors
    ///
    /// Whatever the writer itself reports; the encoding side is
    /// infallible.
    ///
    /// # Complexity
    ///
    /// `O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)` with
    /// width-bounded parked drifts.
    /// As [`encode_rank`](Self::encode_rank): one rank fold plus an
    /// emission linear in the rank's numeric size — the fold's proven
    /// `Ω(M(n))` lower bound included.
    ///
    /// ```
    /// use before::{Ranked, Version};
    /// let v = Version::try_from(5).unwrap();
    /// let mut buf = Vec::new();
    /// Ranked::from(&v).encode_rank_to(&mut buf).unwrap();
    /// assert_eq!(buf, Ranked::from(&v).encode_rank());
    /// ```
    pub fn encode_rank_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.encode_rank())
    }

    /// Decodes an owned view from a reader of canonical
    /// [`encode`](Self::encode) bytes, strictly rejecting everything
    /// else.
    ///
    /// Total over arbitrary input: every byte string either decodes to
    /// the one view that encodes to it, or is rejected — so byte
    /// equality on keys is [`Eq`] on views. The parse reads the
    /// self-delimiting rank stream, then the version's canonical
    /// bytes, and then **verifies the parsed rank against the decoded
    /// version's own rank fold**: the composite's two components are
    /// redundant by construction (the rank is a function of the
    /// version), and enforcing the redundancy on every decode is what
    /// keeps the key canonical — a well-formed rank stream paired with
    /// a version it does not measure is a spelling no `encode` ever
    /// produces, rejected as [`Decode::NotCanonical`] (the genre for
    /// well-formed structure that is not the canonical form of any
    /// value; it is neither a truncation nor trailing input).
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
    /// `O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)` with
    /// width-bounded parked drifts.
    /// One strict parse linear in the bytes read, plus the verifying
    /// rank fold over the decoded version ([`Version::rank`]'s
    /// three-part claim) — the fold is the decode's dominant term and
    /// the price of the verification. On answer-embedding keys the
    /// shipped fold provably pays the backend's multiplication cost —
    /// it recomputes the exact rank whole — but that is a fact about
    /// this algorithm, not a floor on the verification problem: the
    /// key hands the rank in, the obligation is to check it against
    /// the version, and the answer-embedded-product reduction that
    /// floors [`Version::rank`] proves nothing about checking a
    /// claimed answer — whether a verifying decode can go below one
    /// multiplication is open.
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
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Ranked<'static>, Decode> {
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

/// Materializes the rank, as [`to_rank`](Ranked::to_rank).
impl From<Ranked<'_>> for Rank {
    fn from(ranked: Ranked<'_>) -> Rank {
        ranked.to_rank()
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
/// materialized; the tiebreak runs only on rank ties, where the two
/// sides are never causally ordered, so it is causally free.
fn total_cmp(a: &Ranked<'_>, b: &Ranked<'_>) -> Ordering {
    // Equal versions are one value under the total order — the
    // `ranked_orders_by_rank_then_bytes` law in `crate::laws` pins the
    // whole table, equal ranks and equal bytes reading `Equal` — and
    // canonical equality answers in `O(1)` on a shared buffer (clone
    // identity) or one byte compare, where the rank co-sweep would fold
    // both streams whole only to tie and tiebreak `Equal`. Unequal
    // operands pay only the compare's early-exiting prefix.
    if crate::codec::canonical_eq(a.version.view(), b.version.view()) {
        return Ordering::Equal;
    }
    skyline::query::rank_cmp(a.version.view(), b.version.view())
        .then_with(|| a.version.as_bytes().cmp(b.version.as_bytes()))
}

// The comparison family is the total order: rank first, version bytes
// on rank ties, so `Equal` is version identity and `Eq`/`Ord`/`Hash`
// all speak about one value — the version. Equality itself never runs
// the walk: byte equality of canonical forms IS version identity, at
// one memcmp. `Ranked` compares only with `Ranked`: an `==` against a
// bare `Rank` would have to mean rank class on one side and version
// identity on the other, which cannot chain transitively — the type
// docs carry the explicit `to_rank` spelling of the rank question.
impl PartialEq<Ranked<'_>> for Ranked<'_> {
    fn eq(&self, other: &Ranked<'_>) -> bool {
        self.version() == other.version()
    }
}

impl Eq for Ranked<'_> {}

/// Delegates to the viewed version's byte hash: consistent with [`Eq`]
/// (version identity), and equal to the [`Version`]'s own hash.
impl core::hash::Hash for Ranked<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.version().hash(state);
    }
}

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
