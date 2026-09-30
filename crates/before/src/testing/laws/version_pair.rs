//! Laws that need two versions.
//!
//! These laws cover the pairwise lattice and causal-order identities, distance
//! and rank measures, ranked ordering, spans, and prefix-free encoding.

use super::*;

laws! {
    /// Laws over a pair of versions.
    ///
    /// Commutativity and the bound laws of the lattice operations, absorption,
    /// the partial order's pair laws and their coherence with `Eq`/`Hash` and
    /// `concurrent`, the valuation identity tying `rank` to the lattice, the
    /// `distance`/`lag` metric laws, [`Ranked`]'s total order and its
    /// lexicographic key encoding, the degenerate-span identity tying span
    /// placement back to pairwise comparison, the pair span's definitional pin,
    /// the span wire form's round-trip, and the version encoding's
    /// prefix-freedom.
    pub static VERSION_PAIR: (a: &Version, b: &Version);

    /// Commutativity: `a | b == b | a` (the LUB does not depend on operand
    /// order).
    fn merge_commutative {
        (a | b) == (b | a)
    }

    /// Commutativity: `a & b == b & a` (the GLB does not depend on operand
    /// order).
    fn meet_commutative {
        (a & b) == (b & a)
    }

    /// The join is an upper bound: `a <= a | b` and `b <= a | b` — what ties
    /// `|` to the causal order.
    fn merge_is_upper_bound {
        let ab = a | b;
        le(a, &ab) && le(b, &ab)
    }

    /// The meet is a lower bound: `a & b <= a` and `a & b <= b`, the dual of
    /// [`merge_is_upper_bound`].
    fn meet_is_lower_bound {
        let ab = a & b;
        le(&ab, a) && le(&ab, b)
    }

    /// Absorption ties `&` and `|` into a lattice: `a & (a | b) == a` and
    /// `a | (a & b) == a`.
    fn meet_join_absorption {
        (a & &(a | b)) == *a && (a | &(a & b)) == *a
    }

    /// [`Version::join`] is the spelled form of `|`: equal to the operator on
    /// every pair (equality on `Version` is canonical byte equality), so the
    /// named method inherits the operator's differential and law coverage.
    fn join_method_is_the_operator {
        a.join(b) == (a | b)
    }

    /// [`Version::meet`] is the spelled form of `&`, dual to
    /// [`join_method_is_the_operator`]: equal to the operator on every pair,
    /// so the named method inherits the operator's differential and law
    /// coverage.
    fn meet_method_is_the_operator {
        a.meet(b) == (a & b)
    }

    /// The full `^` (BitXor) matrix over owned and borrowed operands equals
    /// [`Version::span`]: every cell is the same pair hull, endpoints and all.
    ///
    /// The hull itself is pinned by [`span_is_the_pair_hull`]; this law pins
    /// each operator cell's delegation to it.
    fn span_operator_matrix_is_the_method {
        let expected = a.span(b);
        (a.clone() ^ b.clone()) == expected
            && (a ^ b.clone()) == expected
            && (a.clone() ^ b) == expected
            && (a ^ b) == expected
    }

    /// Antisymmetry: `a <= b && b <= a ⟹ a == b` (mutually dominating versions
    /// denote the same history, so their canonical bytes coincide).
    fn order_antisymmetric {
        !(le(a, b) && le(b, a)) || a == b
    }

    /// Domination absorbs: `a <= b ⟹ a | b == b && a & b == a`.
    fn order_absorbing {
        !le(a, b) || ((a | b) == *b && (a & b) == *a)
    }

    /// `Eq` and the order agree: `a == b ⟺ partial_cmp == Some(Equal)`.
    fn eq_iff_cmp_equal {
        (a == b) == (a.partial_cmp(b) == Some(Ordering::Equal))
    }

    /// The order is its own dual: `cmp(a, b)` is `cmp(b, a)` reversed
    /// (including the concurrent `None`).
    fn partial_cmp_is_dual {
        a.partial_cmp(b) == b.partial_cmp(a).map(Ordering::reverse)
    }

    /// `concurrent` is exactly incomparability, and symmetric.
    fn concurrent_iff_incomparable {
        a.concurrent(b) == a.partial_cmp(b).is_none() && a.concurrent(b) == b.concurrent(a)
    }

    /// The valuation law: `rank(a|b) + rank(a&b) == rank(a) + rank(b)` (area
    /// is a lattice valuation because `max + min == sum` holds pointwise) —
    /// the identity that makes [`Version::distance`] a metric.
    fn rank_is_a_valuation {
        (a | b).rank() + (a & b).rank() == a.rank() + b.rank()
    }

    /// `rank` is strictly monotone on the causal order: `a <= b ⟹ rank(a) <=
    /// rank(b)`, strictly when `a != b`.
    fn rank_strictly_monotone {
        !le(a, b) || (a.rank() <= b.rank() && (a == b || a.rank() < b.rank()))
    }

    /// The metric symmetry law: `d(a, b) == d(b, a)`.
    fn distance_symmetric {
        a.distance(b) == b.distance(a)
    }

    /// The metric separates points: `d(a, b) == 0 ⟺ a == b`.
    fn distance_separates {
        (a.distance(b) == Rank::ZERO) == (a == b)
    }

    /// `distance` is the valuation gap across the lattice interval:
    /// `d(a, b) == rank(a|b) - rank(a&b)` (the join dominates the meet, so the
    /// subtraction is defined).
    fn distance_is_the_rank_gap {
        (a | b).rank().checked_sub(&(a & b).rank()) == Some(a.distance(b))
    }

    /// `lag` is the directed half of `distance`: the two directions sum to it.
    fn lag_halves_sum_to_distance {
        a.lag(b) + b.lag(a) == a.distance(b)
    }

    /// `lag` vanishes exactly when there is nothing left to learn:
    /// `a.lag(b) == 0 ⟺ b <= a`.
    fn lag_zero_iff_dominated {
        (a.lag(b) == Rank::ZERO) == le(b, a)
    }

    /// `lag` is the valuation gap up to the join: `a.lag(b) == rank(a|b) -
    /// rank(a)`.
    fn lag_is_the_rank_gap {
        (a | b).rank().checked_sub(&a.rank()) == Some(a.lag(b))
    }

    /// `Eq` is canonical-byte equality: `a == b ⟺ encode(a) == encode(b)`.
    fn version_eq_iff_bytes_eq {
        (a == b) == (a.encode() == b.encode())
    }

    /// Version canonical byte encodings are prefix-free: distinct versions'
    /// `as_bytes` are never byte prefixes of one another.
    ///
    /// The property the composite [`Ranked`] key's suffix safety rests on for
    /// its version component, pinned directly rather than inferred from the
    /// stream being bit-self-delimiting: a strict bit-prefix that were itself
    /// canonical would make the longer stream carry live bits past a complete
    /// tree, which the strict decoder rejects — this pins that argument's
    /// conclusion.
    fn version_encoding_is_prefix_free {
        a == b
            || (!a.as_bytes().starts_with(b.as_bytes())
                && !b.as_bytes().starts_with(a.as_bytes()))
    }

    /// `Eq`/`Hash` coherence: equal versions hash equally.
    fn version_eq_implies_hash_eq {
        a != b || hash_of(a) == hash_of(b)
    }

    /// [`Ranked`]'s total order is rank order completed by the version-byte
    /// tiebreak, exactly.
    ///
    /// The fused co-walk equals the materialized `Rank` order wherever the
    /// ranks differ, rank ties resolve by the versions' canonical bytes,
    /// equality is version identity, the explicit spelling of the rank question
    /// (`rank`, then [`Rank`]'s own comparison) answers exactly the
    /// materialized rank order — and the order therefore extends causality
    /// (causally ordered versions compare the same way, by rank strict
    /// monotonicity; only ties fall to the causally-free tiebreak).
    fn ranked_orders_by_rank_then_bytes {
        let (ra, rb) = (Ranked::from(a), Ranked::from(b));
        let rank_want = a.rank().cmp(&b.rank());
        let want = rank_want.then_with(|| a.as_bytes().cmp(b.as_bytes()));
        let fused = ra.cmp(&rb) == want && rb.cmp(&ra) == want.reverse();
        let eq = (ra == rb) == (a == b);
        let explicit = ra.rank().cmp(&rb.rank()) == rank_want
            && (ra.rank() == b.rank()) == (rank_want == Ordering::Equal);
        let extends = match a.partial_cmp(b) {
            Some(ord) => want == ord,
            None => true, // concurrent: rank or tiebreak orders them
        };
        fused && eq && explicit && extends
    }

    /// The composite key encoding is lexicographic, totally: byte order on
    /// [`Ranked::encode`] equals [`Ord`] on the views — ties included, so byte
    /// equality on keys is exactly `Eq` (version identity).
    fn ranked_encoding_orders_like_ord {
        let (ra, rb) = (Ranked::from(a), Ranked::from(b));
        let (ea, eb) = (ra.encode(), rb.encode());
        ea.cmp(&eb) == ra.cmp(&rb) && (ea == eb) == (ra == rb)
    }

    /// [`Span::new`] admits exactly the ordered pairs, and builds exactly the
    /// pair it was given.
    ///
    /// The criterion is the causal order itself: `Span::new(a, b)` is `Ok` ⟺
    /// `a <= b` (concurrent and strictly reversed pairs alike are refused,
    /// with the payload-free [`Crossed`] as the whole verdict), and an
    /// admitted span's endpoints are byte-identical to the arguments.
    fn span_gate_admits_exactly_the_ordered {
        match Span::new(a, b) {
            Ok(span) => le(a, b) && span.lo() == a && span.hi() == b,
            Err(Crossed) => !le(a, b),
        }
    }

    /// `place` against the degenerate span `[v, v]` is pairwise comparison
    /// itself.
    ///
    /// The four verdicts reachable with coincident endpoints transcribe
    /// `partial_cmp`'s four outcomes, and the five endpoint-splitting verdicts
    /// are unreachable.
    fn degenerate_span_place_is_partial_cmp {
        let Ok(span) = Span::new(b, b) else {
            // A version is always ordered with itself.
            return false;
        };
        for probe in [a, b] {
            let expect = match probe.partial_cmp(b) {
                Some(Ordering::Less) => Placement::Before,
                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
                Some(Ordering::Greater) => Placement::After,
                None => Placement::Concurrent(Endpoint::Both),
            };
            if span.place(probe) != expect {
                return false;
            }
        }
        true
    }

    /// The pair span: endpoints the pair's meet and join, commutative,
    /// subsuming the flip repair on comparable pairs, coherent with the n-ary
    /// form at its edges, and preserved exactly by the accessors and the borrow
    /// mechanics.
    ///
    /// The n-ary edges: the empty iterator is the coincident `[self, self]`,
    /// one item is the binary span. The accessors: `meet`/`join` borrow the
    /// endpoints; `into_parts` hands them out owned, in `(meet, join)` order.
    /// The borrow mechanics: `reborrow` reads the same endpoints back
    /// byte-equal (`Version` equality *is* byte equality, pinned by
    /// `version_eq_iff_bytes_eq`), and `into_owned` settles the borrows with
    /// the endpoints byte-equal too — neither moves a value, so `lo <= hi`
    /// rides through both.
    ///
    /// On a comparable pair the hull *is* the reordered pair (either
    /// orientation yields the validated span); on a concurrent pair the
    /// meet/join bracket is the only span containing both.
    fn span_is_the_pair_hull {
        let hull = a.span(b);
        let (meet, join) = (a & b, a | b);
        let definitional = hull == Span::new(&meet, &join).unwrap();
        let accessors = *hull.lo() == meet && *hull.hi() == join && {
            let (lo, hi) = a.span(b).into_parts();
            lo == meet && hi == join
        };
        let reborrowed = {
            let view = hull.reborrow();
            view == hull && *view.lo() == meet && *view.hi() == join
        };
        let settled = {
            // The settling copy (borrowed endpoints) and the free passthrough
            // (already-owned endpoints, the derived hull's state) both preserve
            // the endpoints exactly.
            let copied: Span<'static> = Span::new(&meet, &join)
                .expect("a meet/join pair is ordered")
                .into_owned();
            let passed: Span<'static> = hull.clone().into_owned();
            copied == hull
                && *copied.lo() == meet
                && *copied.hi() == join
                && passed == hull
                && *passed.lo() == meet
                && *passed.hi() == join
        };
        let commutative = hull == b.span(a);
        let flip_subsumed = match a.partial_cmp(b) {
            Some(Ordering::Less | Ordering::Equal) => hull == Span::new(a, b).unwrap(),
            Some(Ordering::Greater) => hull == Span::new(b, a).unwrap(),
            None => true, // no reordering exists; the bracket is definitional
        };
        let empty_edge = a.span_all(core::iter::empty::<&Version>()) == Span::at(a);
        let unary_edge = a.span_all([b]) == hull;
        definitional
            && accessors
            && reborrowed
            && settled
            && commutative
            && flip_subsumed
            && empty_edge
            && unary_edge
    }

    /// The span wire form: `encode` is the meet's encoding followed by the
    /// join's, and `decode ∘ encode` is the identity exactly.
    ///
    /// Quantified over the pair's hull (every valid span is some pair's hull)
    /// and the coincident span `[a, a]`: each round-trips to an equal span, and
    /// the round-tripped span re-encodes to the same bytes — the composite is a
    /// section of canonical bytes, so byte equality on encodings is span
    /// equality.
    fn span_codec_roundtrip {
        let hull = a.span(b);
        let bytes = hull.encode();
        let framed = bytes == [hull.lo().encode(), hull.hi().encode()].concat();
        let round = Span::decode(&bytes[..])
            .is_ok_and(|decoded| decoded == hull && decoded.encode() == bytes);
        let coincident = {
            let span = a.span(a);
            Span::decode(&span.encode()[..]).is_ok_and(|decoded| decoded == span)
        };
        framed && round && coincident
    }

    /// Every causal atom's membership is exactly its order relation, and every
    /// negated atom keeps exactly the complement.
    ///
    /// The eight atomic bounds are transcribed row by row from `partial_cmp`:
    /// the four elementary atoms are the four order relations against the bound
    /// version (concurrency failing all four), and `!` on each keeps precisely
    /// the probes the atom drops — which is where "or concurrent" enters the
    /// query language. Checked in both probe/bound orientations, so the
    /// self-dual corner (`a == b`) and both strict sides are reached on every
    /// call.
    fn atom_membership_matches_relations {
        for (p, q) in [(a, b), (b, a)] {
            let rel = p.partial_cmp(q);
            let le = matches!(rel, Some(Ordering::Less | Ordering::Equal));
            let lt = rel == Some(Ordering::Less);
            let ge = matches!(rel, Some(Ordering::Greater | Ordering::Equal));
            let gt = rel == Some(Ordering::Greater);
            let atoms = causally::after(q).contains(p) == ge
                && causally::strictly_after(q).contains(p) == gt
                && causally::before(q).contains(p) == le
                && causally::strictly_before(q).contains(p) == lt;
            let complements = (!causally::before(q)).contains(p) == !le
                && (!causally::after(q)).contains(p) == !ge;
            // `or_concurrent` widens an atom's relation by incomparability,
            // which is exactly the complement of the opposite side's strict
            // relation.
            let concurrent = rel.is_none();
            let widened = causally::after(q).or_concurrent().contains(p) == (ge || concurrent)
                && causally::after(q).or_concurrent().contains(p) == !lt
                && causally::before(q).or_concurrent().contains(p) == (le || concurrent)
                && causally::before(q).or_concurrent().contains(p) == !gt;
            if !(atoms && complements && widened) {
                return false;
            }
        }
        true
    }

    /// The named query shorthands and conversions equal the expressions they
    /// abbreviate, behaviorally.
    ///
    /// `since` is `!before`, `until` is `!after`, `delta` is `since & before`,
    /// `toward` is `after & until`, `all` admits everything; a [`Span`]
    /// converts to its segment's query (`after(meet) & before(join)`, consuming
    /// and borrowing spellings agreeing) and a [`Version`] to the singleton
    /// query admitting exactly itself. Behavioral equations only: a query's
    /// observation surface is membership, deliberately not identity
    /// ([`Query`] explains why structural equality is absent).
    fn query_shorthands_are_their_expressions {
        let span = a.span(b);
        for p in [a, b] {
            let since = causally::since(a).contains(p) == (!causally::before(a)).contains(p);
            let until = causally::until(a).contains(p) == (!causally::after(a)).contains(p);
            let delta = causally::delta(a, b).contains(p)
                == (causally::since(a) & causally::before(b)).contains(p);
            let toward = causally::toward(a, b).contains(p)
                == (causally::after(a) & causally::until(b)).contains(p);
            let all = causally::all().contains(p);
            let segment = Query::from(&span).contains(p)
                == (causally::after(span.lo()) & causally::before(span.hi())).contains(p)
                && Query::from(span.clone()).contains(p) == Query::from(&span).contains(p);
            let singleton = Query::from(a).contains(p) == (p == a)
                && Query::from(a.clone()).contains(p) == (p == a);
            if !(since && until && delta && toward && all && segment && singleton) {
                return false;
            }
        }
        true
    }
}

// ───────────────────────────── Version: triples ─────────────────────────────
