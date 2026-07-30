//! The rank view: [`Ranked`], a [`Version`] viewed by its causal
//! [`Rank`], with fused comparisons and fused encoding. The public
//! contract lives on the type; this module is private.

use core::cmp::Ordering;
use std::borrow::Cow;

use super::rank::encode_parts;
use super::{skyline, Rank, Version};

/// A [`Version`] viewed by its causal [`Rank`]: comparisons, equality,
/// and [`encode`](Ranked::encode) all act on the rank, straight off the
/// version's packed stream, with no `Rank` value materialized.
///
/// Construction is `O(1)` and borrows (or takes) the version — no fold
/// runs until something is asked. Two `Ranked` values compare in **one
/// fused co-walk** over both packed streams (the signed instance of the
/// distance/lag integrator): cheaper than two rank folds and a compare,
/// and nothing is allocated beyond the walk's accumulators. The rank
/// materializes only on request ([`to_rank`](Self::to_rank), or the
/// [`From`] impl), and [`encode`](Self::encode) emits the rank's
/// canonical order-preserving bytes (see [`Rank::encode`] — the KV-key
/// use lives there) straight from one fold's output, never through a
/// second walk.
///
/// **Equality is rank equality, deliberately coarser than version
/// identity**: two distinct, concurrent versions of equal rank compare
/// [`Equal`](Ordering::Equal). The type is "a version *viewed by* its
/// rank", so its whole comparison family answers questions about the
/// rank, never about which version carries it — and because equal ranks
/// are never causally ordered (see [`Rank`]), an `Equal` verdict still
/// tells a scheduler everything causality can: the two sides are
/// causally interchangeable. Where version identity must separate
/// equal-rank keys, pair the rank with any deterministic tiebreak
/// ([`Version::as_bytes`], a content hash).
///
/// Rank equality is a true equivalence relation (it is equality of the
/// underlying rational values), so [`Eq`] and [`Ord`] hold honestly,
/// and the heterogeneous comparisons below — every mix of `Ranked`,
/// `&Ranked`, [`Rank`], and `&Rank` compares by rank value — satisfy
/// [`PartialEq`]'s cross-type transitivity contract: any chain of
/// verdicts across the family is a chain about one totally ordered set
/// of rationals.
///
/// There is deliberately **no `Hash`**: hashing consistent with this
/// `Eq` must hash the rank, which costs the very fold the view defers —
/// materialize with [`to_rank`](Self::to_rank) (a [`Rank`] hashes in
/// `O(‖r‖)`) where a hashable key is needed. And sorting *many* keys
/// re-walks both versions per comparison: for a sorted container or a
/// one-shot sort over `n` versions, materialize each [`Rank`] (or its
/// [`encoding`](Self::encode)) once — `n` folds — rather than paying a
/// fused walk per probe; the view's comparisons win where a handful of
/// verdicts is the whole job.
///
/// # Complexity
///
/// Construction and [`version`](Self::version) are `O(1)`.
/// [`to_rank`](Self::to_rank) and [`encode`](Self::encode) are one rank
/// fold ([`Version::rank`]'s three-part claim). A `Ranked` vs `Ranked`
/// comparison is the fused pair co-sweep at the distance/lag bound; a
/// `Ranked` vs [`Rank`] comparison is one rank fold plus a rank
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
/// assert!(rh == half.rank() && &one.rank() > &rh); // mixes compare too
/// // Equal rank is Equal, even across distinct concurrent versions:
/// let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
/// assert!(Ranked::from(&half) == Ranked::from(&peaks));
/// assert_ne!(half, peaks);
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

    /// Encodes the rank's canonical order-preserving bytes (the format,
    /// the lexicographic law, and the KV-key use are [`Rank::encode`]'s
    /// contract) straight from the rank fold: one walk over the packed
    /// version, one emission from the fold's `(numerator, exponent)`
    /// output, byte-identical to `self.to_rank().encode()` (a committed
    /// law) without a second walk anywhere. Fusing deeper than the fold
    /// is impossible in principle: the encoding's leading bits depend
    /// on the fold's final total, so no bit can be emitted until the
    /// walk completes.
    ///
    /// Decoding lands on [`Rank::decode`]: the bytes carry exactly the
    /// rank, and a rank does not determine a version, so no `Ranked`
    /// can be decoded — deliberately, since the rank is the whole
    /// content of the key.
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
    /// assert_eq!(Ranked::from(&v).encode(), v.rank().encode());
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let rank = self.version.rank();
        let (num, exp) = rank.raw_parts();
        encode_parts(num, exp)
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

/// The fused rank comparison: one signed co-sweep over both packed
/// streams, no `Rank` materialized.
fn fused_cmp(a: &Ranked<'_>, b: &Ranked<'_>) -> Ordering {
    skyline::query::rank_cmp(a.version.view(), b.version.view())
}

/// A view against a materialized rank: one rank fold, then the rank
/// comparison (`O(1)` across magnitude classes, linear on ties).
fn cmp_rank(a: &Ranked<'_>, b: &Rank) -> Ordering {
    a.to_rank().cmp(b)
}

// Rank equality is an equivalence and rank order a total order (they
// are equality and order of the underlying rationals), so `Eq` and
// `Ord` are honest, and every cross-type cell below is the same one
// question — "how do the two rank values compare?" — which is what
// makes the heterogeneous family transitive as `PartialEq` demands.
impl PartialEq<Ranked<'_>> for Ranked<'_> {
    fn eq(&self, other: &Ranked<'_>) -> bool {
        fused_cmp(self, other) == Ordering::Equal
    }
}

impl Eq for Ranked<'_> {}

impl Ord for Ranked<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        fused_cmp(self, other)
    }
}

impl PartialOrd<Ranked<'_>> for Ranked<'_> {
    fn partial_cmp(&self, other: &Ranked<'_>) -> Option<Ordering> {
        Some(fused_cmp(self, other))
    }
}

// The heterogeneous comparison matrix: `Ranked` and `Rank`, both
// directions, owned and borrowed operand mixes. The macro fans each
// (lhs, rhs) cell out over the owned×owned and owned×ref spellings
// (`&L vs &R` comes from std's blanket forwarding over
// `L: PartialEq<R>`), the `OwnVersion` matrix's idiom.
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
