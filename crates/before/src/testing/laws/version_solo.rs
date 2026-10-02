//! Laws that need one version.
//!
//! These laws cover the lattice point identities, scalar observations, codec
//! and text round trips, and the coherence of ranked views.

use super::*;

// ───────────────────────────── Version: one value ─────────────────────────────

laws! {
    /// Laws over one version.
    ///
    /// The lattice point laws at a single value (idempotence, the bottom
    /// element), observer coherence (`is_empty`, `concurrent`, `distance`,
    /// `rank`, `min_ticks`, [`Ranked`]), and the representational round-trips
    /// (codec, text, byte views).
    pub static VERSION_SOLO: (a: &Version);

    /// Idempotence: `a | a == a` (the LUB of a value and itself is that value).
    fn merge_idempotent {
        (a.clone() | a.clone()) == *a
    }

    /// Idempotence: `a & a == a` (the GLB of a value and itself is that value).
    fn meet_idempotent {
        (a.clone() & a.clone()) == *a
    }

    /// Reflexivity: `a` compares `Equal` to itself (the canonical-bit
    /// short-circuit never reports an inequality).
    fn order_reflexive {
        a.partial_cmp(a) == Some(Ordering::Equal)
    }

    /// `Version::new()` is the lattice bottom: below every version.
    fn new_is_the_bottom {
        Version::new() <= a
    }

    /// The bottom is the join identity: `new | a == a`.
    fn merge_new_is_identity {
        (Version::new() | a.clone()) == *a
    }

    /// The bottom absorbs the meet: `new & a == new`.
    fn meet_new_is_absorbing {
        (Version::new() & a.clone()) == Version::new()
    }

    /// `is_empty` recognizes exactly the bottom: `a.is_empty() ⟺ a == new`.
    fn is_empty_iff_new {
        a.is_empty() == (*a == Version::new())
    }

    /// Concurrency is irreflexive: a version is never concurrent with itself.
    fn never_concurrent_with_self {
        !a.concurrent(a)
    }

    /// The metric point law at the diagonal: `d(a, a) == 0`.
    fn distance_to_self_is_zero {
        a.distance(a) == Rank::ZERO
    }

    /// The directed metric's point law at the diagonal: `a.lag(a) == 0` —
    /// a version lags itself by nothing (`rank(a | a) == rank(a)` by
    /// idempotence).
    fn lag_to_self_is_zero {
        a.lag(a) == Rank::ZERO
    }

    /// The hull at the diagonal: a version's span with itself is the
    /// coincident span `[a, a]` — both endpoints equal `a`, and the empty
    /// n-ary hull (`span_all` of nothing) agrees with it.
    fn span_with_self_is_coincident {
        let span = a.span(a);
        span.lo() == a && span.hi() == a && a.span_all(core::iter::empty::<Version>()) == span
    }

    /// The coincident constructors are the singleton hull: `Span::at`,
    /// the consuming `From<Version>` entry point, and the lending
    /// `From<&Version>` entry point all build exactly the pair hull `a.span(&a)`.
    fn at_is_the_coincident_hull {
        let hull = a.span(a);
        Span::at(a.clone()) == hull && Span::from(a.clone()) == hull && Span::from(a) == hull
    }

    /// `rank` separates the bottom: zero area exactly for the empty version
    /// (rank is a strictly monotone valuation, so only the zero function has
    /// zero area).
    fn rank_zero_iff_empty {
        (a.rank() == Rank::ZERO) == a.is_empty()
    }

    /// `min_ticks` separates the bottom: a zero tick floor exactly for the
    /// empty version (the floor is a sum of nonnegative bases, zero only
    /// when every base is).
    fn min_ticks_zero_iff_empty {
        (a.min_ticks() == Count::ZERO) == a.is_empty()
    }

    /// The whole-interval party is the projection identity: `a / seed == a`.
    fn seed_projection_is_identity {
        (a / &Party::seed()) == *a
    }

    /// [`Ranked`] carries exactly its version's rank, and its key encoding
    /// carries exactly the view.
    ///
    /// Every entry views the same version (both `From` constructors and
    /// the `Version::ranked` method spelling), `rank` (and the `From`
    /// materialization, and the fused `encode_rank` through both its
    /// entries — the view's and `Version::encode_rank`) realize exactly
    /// `Version::rank`'s value, the composite `encode` is the rank
    /// encoding followed by the version's canonical bytes,
    /// `decode ∘ encode` is the identity exactly (same version, equal
    /// view), `Hash` is the viewed version's hash (the delegation `Eq`
    /// coherence rides on), and settling the borrow (`into_owned`)
    /// changes nothing.
    // The owned-instance comparison is the point: the law exercises the
    // `From<Ranked> for Rank` materialization itself.
    #[allow(clippy::cmp_owned)]
    fn ranked_carries_own_rank {
        let ranked = Ranked::from(a);
        ranked.version() == a
            && ranked.rank() == a.rank()
            && Rank::from(ranked.clone()) == a.rank()
            && ranked.encode_rank() == a.rank().encode()
            && a.encode_rank() == a.rank().encode()
            && ranked.encode() == [a.rank().encode(), a.as_bytes().to_vec()].concat()
            && Ranked::decode(&ranked.encode()[..])
                .is_ok_and(|decoded| decoded.version() == a && decoded == ranked)
            && hash_of(&ranked) == hash_of(a)
            && {
                let owned = Ranked::from(a.clone()).into_owned();
                owned.version() == a && owned.rank() == a.rank()
            }
            && {
                let method = a.ranked();
                method.version() == a && method.rank() == a.rank()
            }
    }

    /// `decode ∘ encode == id`, and the round-tripped value re-encodes to the
    /// same bytes (the codec is a section of canonical bytes — what byte-level
    /// `Eq`/`Hash` rest on).
    fn version_codec_roundtrip {
        let bytes = a.encode();
        Version::decode(&bytes[..]).is_ok_and(|decoded| decoded == *a && decoded.encode() == bytes)
    }

    /// The borrowed byte view is the encoding: `as_bytes == encode`.
    fn version_as_bytes_matches_encode {
        a.as_bytes() == &a.encode()[..]
    }

    /// `encoded_bits` is the pre-padding bit length of `encode`: the byte length
    /// is the bit length plus the marker, rounded up to whole bytes.
    fn version_encoded_bits_matches_encode_len {
        a.encode().len() as u64 == (a.encoded_bits() + 1).div_ceil(8)
    }
}

// ───────────────────────────── Version: pairs ─────────────────────────────
