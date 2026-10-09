//! Laws of exact ranks.
//!
//! The laws cover arithmetic identities, order, summation, encoding, and the
//! precision guarantees of decimal formatting.

use super::*;

laws! {
    /// Laws over a triple of ranks.
    ///
    /// `Rank` is a totally ordered commutative monoid: commutativity,
    /// associativity, the `ZERO` identity and bottom, add-monotonicity,
    /// `checked_sub` as the partial inverse defined exactly on domination, the
    /// order's duality, and cross-path normalization (value-equal ranks built
    /// along different operation paths are one structural value, equal under
    /// `Eq` and `Hash`). The wire form's laws ride the same group: the codec
    /// round-trip, byte order equal to `Ord`, and prefix-freedom. So does
    /// `Debug`'s documented format, which is `Display`'s.
    pub static RANK_TRIPLE: (a: &Rank, b: &Rank, c: &Rank);

    /// Addition is commutative: `a + b == b + a`.
    fn rank_add_commutative(a, b, _c) {
        a + b == b + a
    }

    /// Addition is associative: `(a + b) + c == a + (b + c)`.
    fn rank_add_associative {
        &(a + b) + c == a + &(b + c)
    }

    /// `ZERO` is the additive identity.
    fn rank_zero_is_identity(a, _b, _c) {
        a + &Rank::ZERO == a.clone()
    }

    /// `ZERO` is the order's bottom: no rank sits below it.
    fn rank_zero_is_bottom(a, _b, _c) {
        Rank::ZERO <= *a
    }

    /// Addition never shrinks a rank: `a + b >= a`.
    fn rank_add_monotone(a, b, _c) {
        &(a + b) >= a
    }

    /// `checked_sub` inverts addition: `(a + b) - b == a`.
    fn rank_sub_inverts_add(a, b, _c) {
        (a + b).checked_sub(b) == Some(a.clone())
    }

    /// `checked_sub` is defined exactly on domination: `a - b` is `Some` iff
    /// `b <= a`.
    fn rank_checked_sub_iff_dominated(a, b, _c) {
        a.checked_sub(b).is_some() == (b <= a)
    }

    /// Where defined, subtraction restores: `(a - b) + b == a`.
    fn rank_sub_then_add_restores(a, b, _c) {
        match a.checked_sub(b) {
            Some(difference) => &difference + b == a.clone(),
            None => true,
        }
    }

    /// `saturating_sub` is `checked_sub` with the nonexistent difference
    /// floored: equal where the difference exists, `ZERO` exactly
    /// otherwise.
    fn rank_saturating_sub_is_checked_sub_floored(a, b, _c) {
        a.saturating_sub(b) == a.checked_sub(b).unwrap_or(Rank::ZERO)
    }

    /// Saturation reaches the floor from every deficit: `a - (a + b)` is
    /// `ZERO` (with `b == ZERO` the degenerate equal-operands arm).
    fn rank_saturating_sub_saturates_at_zero(a, b, _c) {
        a.saturating_sub(&(a + b)) == Rank::ZERO
    }

    /// The total order is its own dual: `cmp(a, b)` is `cmp(b, a)` reversed.
    fn rank_cmp_antisymmetric(a, b, _c) {
        a.cmp(b) == b.cmp(a).reverse()
    }

    /// `decode ∘ encode == id`, and the round-tripped rank re-encodes to the
    /// same bytes (the wire form is a section of canonical bytes).
    fn rank_codec_roundtrip(a, _b, _c) {
        let bytes = a.encode();
        Rank::decode(&bytes[..]).is_ok_and(|decoded| decoded == *a && decoded.encode() == bytes)
    }

    /// THE LAW of the rank wire form: byte-wise lexicographic order on
    /// canonical encodings equals `Ord` on the ranks — ties included, so byte
    /// equality on encodings is exactly `Eq`.
    fn rank_lex_order(a, b, _c) {
        a.encode().cmp(&b.encode()) == a.cmp(b)
    }

    /// Rank encodings are prefix-free: distinct ranks' encodings are never
    /// byte prefixes of one another — what lets one encoding self-delimit
    /// inside a composite key, and byte order stay rank order under any
    /// appended tiebreak.
    fn rank_encoding_prefix_free(a, b, _c) {
        a == b || {
            let (ea, eb) = (a.encode(), b.encode());
            !ea.starts_with(&eb) && !eb.starts_with(&ea)
        }
    }

    /// Value-equal ranks built along different operation paths — pairwise
    /// addition, `Sum`, and add-then-subtract — are one structural value.
    ///
    /// Equal under `Eq` and under `Hash`: the normalization invariant `Ord`'s
    /// class-first fast path and every container key rest on.
    fn rank_cross_path_normalization {
        let via_add = a + b;
        let via_sum = [a.clone(), b.clone()].into_iter().sum::<Rank>();
        let via_sub = (&(a + b) + c).checked_sub(c);
        via_add == via_sum
            && via_sub == Some(via_add.clone())
            && hash_of(&via_add) == hash_of(&via_sum)
    }

    /// `Debug` renders a rank exactly as `Display` does, as its documentation
    /// states, both in the default format and when width, precision, and
    /// alignment flags apply.
    ///
    /// A `Debug` that formats through a fresh formatter, such as
    /// `write!(f, "{self}")`, matches only the default format.
    fn rank_debug_is_display(a, _b, _c) {
        format!("{a:?}") == a.to_string() && format!("{a:>40.8?}") == format!("{a:>40.8}")
    }
}

// ───────────────────────────── Clock: one value ─────────────────────────────
