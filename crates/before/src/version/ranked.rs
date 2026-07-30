//! The rank view: [`Ranked`], a [`Version`] ordered totally by its
//! causal [`Rank`], with fused comparisons and a composite key
//! encoding. The public contract lives on the type; this module is
//! private.

use core::cmp::Ordering;
use std::borrow::Cow;

use super::rank::{decode_stream, encode_parts};
use super::{skyline, Rank, Version};
use crate::error::Decode;

/// A [`Version`] as a **total causal-ordering key**: ordered by causal
/// [`Rank`] with a deterministic tiebreak, equal only to itself, and
/// encoded as a composite byte key whose lexicographic order equals
/// [`Ord`].
///
/// Construction is `O(1)` and borrows (or takes) the version — no fold
/// runs until something is asked. Two `Ranked` values compare in **one
/// fused co-walk** over both packed streams (the signed instance of the
/// distance/lag integrator), with no `Rank` value materialized:
/// cheaper than two rank folds and a compare, and nothing is allocated
/// beyond the walk's accumulators. The rank materializes only on
/// request ([`to_rank`](Self::to_rank), or the [`From`] impl).
///
/// # The total order
///
/// Rank first: causally ordered versions compare as causality does
/// (rank is strictly monotone — see [`Rank`]). Equal ranks are never
/// causally ordered — the two sides are the same version or concurrent
/// — so the tiebreak that completes the total order is causally free:
/// rank-equal distinct versions are ordered by a fixed, deterministic
/// comparison of their canonical bytes. Which of two rank-equal
/// versions sorts first is deliberately **unspecified beyond being
/// stable**: the same verdict in every execution, on every replica,
/// carrying no causal meaning. **Equality is version identity** —
/// distinct concurrent versions of equal rank compare unequal (and
/// non-`Equal` under [`Ord`]), so no two distinct versions ever
/// conflate under this order. Equality never runs the walk: it is the
/// versions' canonical byte comparison. [`Hash`] delegates to the
/// version's byte hash, so `Eq` and `Hash` agree (a `Ranked` hashes
/// exactly as the [`Version`] it views).
///
/// # The key encoding
///
/// [`encode`](Self::encode) emits the rank's self-delimiting
/// order-preserving stream ([`Rank::encode`]'s form) followed by the
/// version's canonical bytes ([`Version::as_bytes`]) — the tiebreak
/// built into the key. Byte-wise lexicographic order on these keys
/// **equals [`Ord`] on the views, totally**: byte equality is exactly
/// [`Eq`], and the order survives any appended suffix (both components
/// are prefix-free, a committed pin). A sorted KV store keyed by this
/// composite delivers causes before effects with no rank-aware
/// comparator, needs no further tiebreak suffix, and can recover the
/// stored version from the key alone ([`decode`](Self::decode)). Where
/// rank-*class* semantics are wanted instead — all rank-equal keys
/// collapsing to one — key by [`Rank::encode`] plus a tiebreak of your
/// own choosing; [`encode_rank`](Self::encode_rank) emits exactly those
/// bytes from the view, fused.
///
/// # Comparing against a bare [`Rank`]
///
/// The heterogeneous comparisons — every mix of `Ranked`, `&Ranked`,
/// [`Rank`], and `&Rank` — answer the **rank question only**: a `Rank`
/// carries no version to tiebreak with. One consequence to hold in
/// mind: two rank-equal `Ranked` views each compare equal to the same
/// [`Rank`] while comparing unequal to each other, so equality is not
/// transitive *across* the family (the one deliberate deviation from
/// [`PartialEq`]'s cross-type recommendation; the strict order `<` does
/// chain soundly, since the rank leg decides it). Read a cross-type
/// verdict as "how does the view's rank compare?", never as identity.
///
/// # Cost shape
///
/// Sorting *many* keys re-walks both versions per comparison: for a
/// sorted container or a one-shot sort over `n` versions, materialize
/// each key ([`encode`](Self::encode)) once — `n` folds — rather than
/// paying a fused walk per probe; the view's comparisons win where a
/// handful of verdicts is the whole job. Equality alone is one byte
/// compare, no walk.
///
/// # Complexity
///
/// Construction and [`version`](Self::version) are `O(1)`.
/// [`to_rank`](Self::to_rank) and the encodes are one rank fold
/// ([`Version::rank`]'s three-part claim). A `Ranked` vs `Ranked`
/// comparison is the fused pair co-sweep at the distance/lag bound,
/// plus — only on rank ties — one byte comparison of the two versions;
/// a `Ranked` vs [`Rank`] comparison is one rank fold plus a rank
/// comparison.
///
/// **Complexity**: `O(a + b)` space; time `O(M(a + b) · log (a + b))` worst case, `Ω(M(a + b))` mandatory, `O((a + b) log (a + b))` with width-bounded parked drifts.
///
/// ```
/// use before::{Ranked, Version};
/// let half: Version = "(0, 1, 0)".parse().unwrap();
/// let one = Version::try_from(1).unwrap();
/// // Borrowing views: no fold has run yet.
/// let (rh, ro) = (Ranked::from(&half), Ranked::from(&one));
/// assert!(rh < ro); // one fused co-walk, no Rank built
/// assert!(rh == half.rank() && &one.rank() > &rh); // rank-only mixes
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
    /// **Complexity**: `O(1)`.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Materializes the rank: one fold over the version's packed
    /// stream, exactly [`Version::rank`].
    ///
    /// # Complexity
    ///
    /// As [`Version::rank`] (one rank fold).
    ///
    /// **Complexity**: `O(n)` space; time `O(M(n) · log n)` worst case, `Ω(M(n))` mandatory, `O(n log n)` with width-bounded parked drifts.
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
    /// One byte copy of the version when borrowed; free when already
    /// owned.
    ///
    /// **Complexity**: `O(n)` when borrowed (one byte copy of the version); `O(1)` when owned.
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
    /// One rank fold, an emission linear in the rank's numeric size
    /// (which the fold keeps linear in the packed input — the
    /// provenance pin on [`Rank::encode`]), and one byte copy of the
    /// version.
    ///
    /// **Complexity**: `O(n)` space; time `O(M(n) · log n)` worst case, `Ω(M(n))` mandatory, `O(n log n)` with width-bounded parked drifts.
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
        let rank = self.version.rank();
        let (num, exp) = rank.raw_parts();
        let mut bytes = encode_parts(num, exp);
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
    /// As [`encode`](Self::encode): one rank fold, a linear emission,
    /// and one byte copy of the version.
    ///
    /// **Complexity**: `O(n)` space; time `O(M(n) · log n)` worst case, `Ω(M(n))` mandatory, `O(n log n)` with width-bounded parked drifts.
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
    /// One rank fold plus an emission linear in the rank's numeric
    /// size, which the fold keeps linear in the packed input (the
    /// provenance pin on [`Rank::encode`]).
    ///
    /// **Complexity**: `O(n)` space; time `O(M(n) · log n)` worst case, `Ω(M(n))` mandatory, `O(n log n)` with width-bounded parked drifts.
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
    /// As [`encode_rank`](Self::encode_rank): one rank fold plus an
    /// emission linear in the rank's numeric size.
    ///
    /// **Complexity**: `O(n)` space; time `O(M(n) · log n)` worst case, `Ω(M(n))` mandatory, `O(n log n)` with width-bounded parked drifts.
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
    /// One strict parse linear in the bytes read, plus the verifying
    /// rank fold over the decoded version ([`Version::rank`]'s
    /// three-part claim) — the fold is the decode's dominant term and
    /// the price of the verification.
    ///
    /// **Complexity**: `O(n)` space; time `O(M(n) · log n)` worst case, `Ω(M(n))` mandatory, `O(n log n)` with width-bounded parked drifts.
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
    skyline::query::rank_cmp(a.version.view(), b.version.view())
        .then_with(|| a.version.as_bytes().cmp(b.version.as_bytes()))
}

/// A view against a materialized rank: one rank fold, then the rank
/// comparison (`O(1)` across magnitude classes, linear on ties).
///
/// No version tiebreak exists on this side — a `Rank` carries no
/// version — so the verdict is rank-only.
fn cmp_rank(a: &Ranked<'_>, b: &Rank) -> Ordering {
    a.to_rank().cmp(b)
}

// The homogeneous family is the total order: rank first, version bytes
// on rank ties, so `Equal` is version identity and `Eq`/`Ord`/`Hash`
// all speak about one value — the version. Equality itself never runs
// the walk: byte equality of canonical forms IS version identity, at
// one memcmp.
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

// The heterogeneous comparison matrix: `Ranked` and `Rank`, both
// directions, owned and borrowed operand mixes, every cell the rank
// question alone (a `Rank` carries no version to tiebreak with). Rank
// equality is an equivalence and rank order a total order, so each
// cross-type cell is honest in isolation; what the family deliberately
// gives up is cross-type equality chaining THROUGH a `Rank` — two
// rank-equal views each equal the same `Rank` yet differ from each
// other (the type docs carry the hazard). The strict order still
// chains: `<` verdicts are decided on the rank leg, which is
// transitive. The macro fans each (lhs, rhs) cell out over the
// owned×owned and owned×ref spellings (`&L vs &R` comes from std's
// blanket forwarding over `L: PartialEq<R>`), the `OwnVersion`
// matrix's idiom.
macro_rules! rank_cmp_impls {
    ($($lhs:ty, $rhs:ty, $cmp:expr, ($($lt:lifetime),*));* $(;)?) => {
        $(
            impl<$($lt),*> PartialEq<$rhs> for $lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    $cmp(self, o) == Ordering::Equal
                }
            }
            impl<$($lt),*> PartialOrd<$rhs> for $lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    Some($cmp(self, o))
                }
            }
            impl<$($lt),*> PartialEq<$rhs> for &$lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    $cmp(*self, o) == Ordering::Equal
                }
            }
            impl<$($lt),*> PartialOrd<$rhs> for &$lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    Some($cmp(*self, o))
                }
            }
            impl<$($lt),*> PartialEq<&$rhs> for $lhs {
                fn eq(&self, o: &&$rhs) -> bool {
                    $cmp(self, *o) == Ordering::Equal
                }
            }
            impl<$($lt),*> PartialOrd<&$rhs> for $lhs {
                fn partial_cmp(&self, o: &&$rhs) -> Option<Ordering> {
                    Some($cmp(self, *o))
                }
            }
        )*
    };
}

rank_cmp_impls! {
    Ranked<'a>, Rank, cmp_rank, ('a);
    Rank, Ranked<'a>,
        (|r: &Rank, v: &Ranked<'_>| cmp_rank(v, r).reverse()),
        ('a);
}
