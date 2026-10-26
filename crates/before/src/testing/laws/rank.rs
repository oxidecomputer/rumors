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
    /// round-trip, byte order equal to `Ord`, and prefix-freedom. So do the
    /// trait-impl laws: every spelling of addition agrees, `Debug`'s format is
    /// `Display`'s, as documented, and `Display`'s formatting flags apply to
    /// the whole rendered text.
    pub static RANK_TRIPLE: (a: &Rank, b: &Rank, c: &Rank);
    roster rank_triple_laws;

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
    /// canonical encodings equals `Ord` on the ranks, ties included, so byte
    /// equality on encodings is exactly `Eq`.
    fn rank_lex_order(a, b, _c) {
        a.encode().cmp(&b.encode()) == a.cmp(b)
    }

    /// Rank encodings are prefix-free: distinct ranks' encodings are never
    /// byte prefixes of one another, so one encoding self-delimits inside a
    /// composite key, and byte order stays rank order under any appended
    /// tiebreak.
    fn rank_encoding_prefix_free(a, b, _c) {
        a == b || {
            let (ea, eb) = (a.encode(), b.encode());
            !ea.starts_with(&eb) && !eb.starts_with(&ea)
        }
    }

    /// Value-equal ranks built along different operation paths (pairwise
    /// addition, `Sum`, and add-then-subtract) are one structural value.
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

    /// Every spelling of addition computes `a + b`: the four owned and
    /// borrowed `+` cells, `+=` with an owned or borrowed right operand, and
    /// `Sum` over owned or borrowed items.
    fn rank_add_spellings_agree(a, b, _c) {
        let sum = a + b;
        let mut by_ref = a.clone();
        by_ref += b;
        let mut by_value = a.clone();
        by_value += b.clone();
        let spellings = [
            a.clone() + b.clone(),
            a.clone() + b,
            a + b.clone(),
            by_ref,
            by_value,
            [a.clone(), b.clone()].into_iter().sum(),
            [a, b].into_iter().sum(),
        ];
        spellings.iter().all(|spelled| *spelled == sum)
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

    /// `Display` formats the whole rendered text as `str` would under the same
    /// width, fill, alignment, and precision, and ignores the sign and
    /// zero-padding flags, as its documentation states.
    ///
    /// The widths fall at zero, at the text's length, and past it, under the
    /// default, centered, and right alignments with a multibyte and an ASCII
    /// fill; the precisions cut the text to nothing, inside it, and at its
    /// middle. `std::fmt` accepts widths and precisions up to `u16::MAX` only.
    /// Padding applies only past the text's length, so a text longer than
    /// that limit can meet no padded case: there the law checks the widths it
    /// can name, at which every alignment and fill writes the text unchanged,
    /// and skips the padded formats rather than rendering a long text again
    /// to no effect.
    fn rank_display_flags_format_the_plain_text(a, _b, _c) {
        let text = a.to_string();
        let len = text.len();
        let limit = usize::from(u16::MAX);
        let at = len.min(limit);
        let unpadded = format!("{a:0$}", 0) == text && format!("{a:at$}") == format!("{text:at$}");
        let padded = len + 3 > limit || {
            let past = len + 3;
            format!("{a:é^past$}") == format!("{text:é^past$}")
                && format!("{a:*>past$}") == format!("{text:*>past$}")
        };
        let (wide, middle) = ((len + 2).min(limit), (len / 2).min(limit));
        let cut = format!("{a:.0}") == format!("{text:.0}")
            && format!("{a:*^7.2}") == format!("{text:*^7.2}")
            && format!("{a:>wide$.middle$}") == format!("{text:>wide$.middle$}");
        let unsigned = format!("{a:+}") == text && format!("{a:+08}") == format!("{text:8}");
        unpadded && padded && cut && unsigned
    }
}

// ───────────────────────────── Clock: one value ─────────────────────────────
