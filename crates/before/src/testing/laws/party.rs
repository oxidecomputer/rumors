//! Laws of party ownership.
//!
//! These laws cover ownership-tree representation, disjoint join, splitting,
//! subtraction, and the conservation of owned regions across variadic joins.

use super::*;

laws! {
    /// Laws over one live party.
    ///
    /// The fork/join round-trip and its disjointness geometry, the balanced
    /// n-way fork's two forms, the covering order's point laws (with a
    /// constructed transitivity chain), `without` at the reflexive corner,
    /// aliasing, and the representational round-trips. `join_all`'s fold laws
    /// are [`PARTY_AND_LIST`]'s: the width-quantified family covers reunion,
    /// acceptance, and error conservation.
    pub static PARTY_SOLO: (p: &Party);

    /// `fork` then `join` round-trips: the two halves reconstruct the original
    /// region exactly.
    fn fork_join_roundtrip {
        let mut kept = p.dangerously_alias();
        let given = kept.fork();
        kept.join(given).is_ok() && kept == *p
    }

    /// The two halves a `fork` produces are disjoint, the relation is
    /// symmetric, and neither half is anonymous (both re-encode and decode as
    /// nonzero shares) — the invariant that keeps a forked population pairwise
    /// `join`-able.
    fn fork_halves_disjoint {
        let mut kept = p.dangerously_alias();
        let given = kept.fork();
        kept.is_disjoint(&given)
            && given.is_disjoint(&kept)
            && Party::decode(&kept.encode()[..]).is_ok()
            && Party::decode(&given.encode()[..]).is_ok()
    }

    /// A fork's parent covers both halves, and the halves cover neither other
    /// (they are disjoint proper subregions).
    fn fork_halves_covered_by_parent {
        let mut kept = p.dangerously_alias();
        let given = kept.fork();
        p.covers(&kept) && p.covers(&given) && !kept.covers(&given) && !given.covers(&kept)
    }

    /// The two balanced-fork forms agree: `From<Party>` for `[Party; N]` equals
    /// the residual the borrowing `forks(N - 1)` keeps, followed by the shares
    /// it yields (`[residual] ++ forks`).
    fn forks_matches_from_array {
        const N: usize = 4;
        let array: [Party; N] = p.dangerously_alias().into();
        let mut keeper = p.dangerously_alias();
        let yielded: Vec<Party> = keeper.forks(N as u64 - 1).collect();
        let reconstructed: Vec<Party> = std::iter::once(keeper).chain(yielded).collect();
        array.iter().eq(reconstructed.iter())
    }

    /// After taking 2 of 5 forks and dropping the iterator, the borrower still
    /// owns the other shares: rejoining the returned pair recovers the original
    /// region.
    fn forks_partial_drop_conserves_party {
        let mut keeper = p.dangerously_alias();
        let taken: Vec<Party> = keeper.forks(5u64).take(2).collect(); // iterator dropped after 2
        keeper.join_all(taken).is_ok() && keeper == *p
    }

    /// Joining an overlapping party errors and hands it back unchanged: a
    /// proper subregion refuses to absorb the region containing it.
    fn join_overlap_hands_back {
        let mut sub = p.dangerously_alias();
        let _ = sub.fork(); // sub is now a proper subregion of p
        match sub.join(p.dangerously_alias()) {
            Err(handed_back) => handed_back == *p,
            Ok(()) => false,
        }
    }

    /// Covering is reflexive: a party covers its own region.
    fn covers_reflexive {
        p.covers(&p.dangerously_alias())
    }

    /// Covering chains down a constructed fork tower: the whole covers its
    /// half, the half its quarter, and — transitively — the whole covers the
    /// quarter.
    fn covers_transitive_constructed {
        let mut quarter = p.dangerously_alias();
        let _ = quarter.fork(); // quarter: a half of p
        let half = quarter.dangerously_alias();
        let _ = quarter.fork(); // quarter: a quarter of p
        p.covers(&half) && half.covers(&quarter) && p.covers(&quarter)
    }

    /// The whole-interval seed covers every live party — and, owning
    /// everything, is disjoint from none.
    fn seed_covers_every_party {
        Party::seed().covers(p) && !Party::seed().is_disjoint(p) && !p.is_disjoint(&Party::seed())
    }

    /// Disjointness is irreflexive on live parties: a nonzero region overlaps
    /// itself.
    fn never_disjoint_from_self {
        !p.is_disjoint(&p.dangerously_alias())
    }

    /// A party covers itself, so removing itself leaves nothing:
    /// `p \ p == None`.
    fn without_self_is_none {
        p.dangerously_alias().without(p).is_none()
    }

    /// `without` is the partial inverse of `join` on the fork lattice: carving
    /// a forked-off share back out of the parent recovers the kept half, and
    /// removing a disjoint share is a no-op.
    fn without_inverts_fork {
        let mut keep = p.dangerously_alias();
        let give = keep.fork();
        let carved = p.dangerously_alias().without(&give);
        let noop = keep.dangerously_alias().without(&give);
        carved.is_some_and(|c| c == keep) && noop.is_some_and(|n| n == keep)
    }

    /// `dangerously_alias` yields a byte-identical, `Eq` copy aliasing the
    /// entire region: the two are *not* disjoint — the deliberate linearity
    /// violation the method documents.
    fn alias_is_byte_identical_overlap {
        let dup = p.dangerously_alias();
        dup == *p && dup.as_bytes() == p.as_bytes() && !p.is_disjoint(&dup)
    }

    /// `is_seed` recognizes exactly the whole-interval party: `p.is_seed() ⟺
    /// p == seed`.
    fn is_seed_iff_equals_seed {
        p.is_seed() == (*p == Party::seed())
    }

    /// `decode ∘ encode == id`, and the round-tripped party re-encodes to the
    /// same bytes.
    fn party_codec_roundtrip {
        let bytes = p.encode();
        Party::decode(&bytes[..]).is_ok_and(|decoded| decoded == *p && decoded.encode() == bytes)
    }

    /// The borrowed byte view is the encoding: `as_bytes == encode`.
    fn party_as_bytes_matches_encode {
        p.as_bytes() == &p.encode()[..]
    }

    /// `encoded_bits` is the pre-pad bit length of `encode`.
    fn party_encoded_bits_matches_encode_len {
        p.encode().len() as u64 == (p.encoded_bits() + 1).div_ceil(8)
    }
}

// ───────────────────────────── Party: pairs ─────────────────────────────

laws! {
    /// Laws over a pair of live parties.
    ///
    /// The covering order's antisymmetry and its exclusion by disjointness,
    /// disjointness symmetry, `join`'s outcome-quantified commutativity and its
    /// coherence with `is_disjoint`, `without`'s two characterizations, and
    /// `Eq`/`Hash` coherence.
    pub static PARTY_PAIR: (a: &Party, b: &Party);

    /// Covering is antisymmetric: two regions cover each other exactly when
    /// they are equal.
    fn covers_antisymmetric {
        (a.covers(b) && b.covers(a)) == (a == b)
    }

    /// Disjointness is symmetric.
    fn disjoint_symmetric {
        a.is_disjoint(b) == b.is_disjoint(a)
    }

    /// Disjoint live regions cover neither other (covering needs overlap, and
    /// a live party is nonempty).
    fn disjoint_excludes_covering {
        !a.is_disjoint(b) || (!a.covers(b) && !b.covers(a))
    }

    /// `join` accepts exactly the disjoint pairs: `a.join(b) is Ok ⟺
    /// a.is_disjoint(b)`.
    fn join_defined_iff_disjoint {
        a.dangerously_alias().join(b.dangerously_alias()).is_ok() == a.is_disjoint(b)
    }

    /// `join` is commutative over outcomes: both orders agree in arm, produce
    /// equal unions on `Ok`, and hand back the argument unchanged (leaving
    /// `self` unchanged) on `Err`.
    fn join_commutative_outcomes {
        let mut ab = a.dangerously_alias();
        let ab_result = ab.join(b.dangerously_alias());
        let mut ba = b.dangerously_alias();
        let ba_result = ba.join(a.dangerously_alias());
        match (ab_result, ba_result) {
            (Ok(()), Ok(())) => ab == ba,
            (Err(back_b), Err(back_a)) => back_b == *b && back_a == *a && ab == *a && ba == *b,
            _ => false,
        }
    }

    /// A disjoint join absorbs both operands, and `without` undoes it:
    /// `(a + b).covers(a)`, `(a + b).covers(b)`, and `(a + b) \ a == b`.
    fn join_covers_both_and_without_undoes {
        if !a.is_disjoint(b) {
            return true;
        }
        let mut joined = a.dangerously_alias();
        if joined.join(b.dangerously_alias()).is_err() {
            return false;
        }
        joined.covers(a) && joined.covers(b) && joined.without(a).is_some_and(|rest| rest == *b)
    }

    /// `without`'s two characterizations: the result is `None` exactly when
    /// `other` covers `self`, and a surviving remainder is a subregion of
    /// `self` disjoint from `other`.
    fn without_characterization {
        match a.dangerously_alias().without(b) {
            None => b.covers(a),
            Some(remainder) => !b.covers(a) && a.covers(&remainder) && remainder.is_disjoint(b),
        }
    }

    /// Removing a disjoint share is a no-op: `a \ b == a` when the regions
    /// share nothing.
    fn without_disjoint_is_noop {
        !a.is_disjoint(b) || a.dangerously_alias().without(b).is_some_and(|r| r == *a)
    }

    /// `Eq` is canonical-byte equality: `a == b ⟺ encode(a) == encode(b)`.
    fn party_eq_iff_bytes_eq {
        (a == b) == (a.encode() == b.encode())
    }

    /// `Eq`/`Hash` coherence: equal parties hash equally.
    fn party_eq_implies_hash_eq {
        a != b || hash_of(a) == hash_of(b)
    }
}

// ───────────────────────────── Party: triples ─────────────────────────────

laws! {
    /// Laws over a triple of live parties.
    ///
    /// The covering order's incidental transitivity and the partial monoid's
    /// associativity, outcome-quantified.
    pub static PARTY_TRIPLE: (a: &Party, b: &Party, c: &Party);

    /// Covering is transitive: whenever three arbitrary parties happen to chain
    /// (`a ⊇ b ⊇ c`), the endpoints must too.
    fn covers_transitive_incidental {
        !(a.covers(b) && b.covers(c)) || a.covers(c)
    }

    /// The partial monoid is associative over outcomes: `(a + b) + c` and
    /// `a + (b + c)` agree in definedness and, where defined, in value (both
    /// are defined exactly on pairwise-disjoint triples).
    fn join_associative_outcomes {
        let left = join3(a, b, c);
        let right = {
            let mut bc = b.dangerously_alias();
            match bc.join(c.dangerously_alias()) {
                Ok(()) => {
                    let mut acc = a.dangerously_alias();
                    acc.join(bc).ok().map(|()| acc)
                }
                Err(_) => None,
            }
        };
        match (left, right) {
            (Some(l), Some(r)) => l == r,
            (None, None) => true,
            _ => false,
        }
    }
}

/// Fold `second` then `third` into an alias of `first`, `None` at the
/// first overlap — one association order of the partial monoid's ternary
/// sum.
fn join3(first: &Party, second: &Party, third: &Party) -> Option<Party> {
    let mut acc = first.dangerously_alias();
    acc.join(second.dangerously_alias()).ok()?;
    acc.join(third.dangerously_alias()).ok()?;
    Some(acc)
}

// ───────────────────── Party: a receiver and items ─────────────────────

laws! {
    /// Laws for joining any number of parties into a receiver.
    ///
    /// They cover the acceptance condition, successful results, reunion of
    /// forked parties, and preservation of every region after an error.
    pub static PARTY_AND_LIST: (p: &Party, items: &[Party]);

    /// `join_all` succeeds exactly when all parties, including the receiver,
    /// are pairwise disjoint.
    ///
    /// On success its result equals sequential calls to [`Party::join`]. On
    /// error the receiver still covers its original region and at least one
    /// party is returned.
    fn party_join_all_accepts_iff_family_pairwise_disjoint {
        let family: Vec<&Party> = core::iter::once(p).chain(items).collect();
        let pairwise_disjoint = family
            .iter()
            .enumerate()
            .all(|(i, a)| family[i + 1..].iter().all(|b| a.is_disjoint(b)));
        let mut acc = p.dangerously_alias();
        match acc.join_all(items.iter().map(Party::dangerously_alias)) {
            Ok(()) => {
                let mut seq = p.dangerously_alias();
                let sequential = items
                    .iter()
                    .all(|item| seq.join(item.dangerously_alias()).is_ok());
                pairwise_disjoint && sequential && acc == seq
            }
            Err(returned) => !pairwise_disjoint && !returned.is_empty() && acc.covers(p),
        }
    }

    /// `join_all` reunites the result of `forks(k)` for every `k`.
    ///
    /// Joining half of the parties is also compared with sequential calls to
    /// [`Party::join`], so the test checks an intermediate result as well as
    /// the final reunion. With no inputs, the receiver is unchanged.
    fn party_join_all_reunites_forks_at_any_width {
        let width = items.len();
        let mut keeper = p.dangerously_alias();
        let shares: Vec<Party> = keeper.forks(width as u64).collect();
        let half = &shares[..width / 2];
        let mut seq = keeper.dangerously_alias();
        if !half
            .iter()
            .all(|share| seq.join(share.dangerously_alias()).is_ok())
        {
            return false;
        }
        let mut balanced = keeper.dangerously_alias();
        if balanced
            .join_all(half.iter().map(Party::dangerously_alias))
            .is_err()
            || balanced != seq
        {
            return false;
        }
        keeper.join_all(shares).is_ok() && keeper == *p
    }

    /// An unsuccessful `join_all` preserves every input region.
    ///
    /// Together, the receiver and returned parties cover the original receiver
    /// and every input. Returned parties may combine several inputs, so the law
    /// compares their union rather than individual values.
    fn party_join_all_err_conserves_the_region_union {
        let mut acc = p.dangerously_alias();
        match acc.join_all(items.iter().map(Party::dangerously_alias)) {
            Ok(()) => true,
            Err(returned) => {
                let mut union = acc;
                for back in returned {
                    if let Some(missing) = back.without(&union) {
                        if union.join(missing).is_err() {
                            return false; // the remainder is disjoint by construction
                        }
                    }
                }
                union.covers(p) && items.iter().all(|item| union.covers(item))
            }
        }
    }
}

// ───────────────────────────── Version × Party ─────────────────────────────
