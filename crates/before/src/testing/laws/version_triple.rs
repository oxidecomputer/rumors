//! Laws that need three versions.
//!
//! Three inputs expose associativity, distributivity, transitivity, triangle
//! inequalities, query composition, and the full span algebra.

use super::*;

laws! {
    /// Laws over a triple of versions.
    ///
    /// Associativity, the least/greatest bound laws, both distributive laws,
    /// transitivity (constructed and incidental), the metric and quasi-metric
    /// triangle inequalities with `lag`'s monotonicities, and
    /// the [`causally`] query and placement laws: conjunction as pointwise
    /// intersection across every atomic operand pairing and every typed `&`
    /// form, coverage's sound arms and its point degeneracy to membership, the
    /// segment-query/span-placement tie, the nine-way [`Span::place`] verdict
    /// as a pure transcription of the two endpoint comparisons, its
    /// coarsening to `dominance`, and the span operators' associativity in
    /// both of the span algebra's lattices.
    pub static VERSION_TRIPLE: (a: &Version, b: &Version, c: &Version);

    /// Associativity: `(a | b) | c == a | (b | c)` — with commutativity and
    /// idempotence, `|` is a join-semilattice operation.
    fn merge_associative {
        (&(a | b) | c) == (a | &(b | c))
    }

    /// Associativity: `(a & b) & c == a & (b & c)`, the meet dual.
    fn meet_associative {
        (&(a & b) & c) == (a & &(b & c))
    }

    /// The join is the *least* upper bound: the constructed common upper bound
    /// `a | b | c` dominates `a | b` (any common upper bound of `a` and `b`
    /// dominates their join).
    fn merge_is_least_upper_bound {
        let ab = a | b;
        let upper = &ab | c;
        a <= upper && b <= upper && ab <= upper
    }

    /// The meet is the *greatest* lower bound: the constructed common lower
    /// bound `a & b & c` is dominated by `a & b`, the dual of
    /// [`merge_is_least_upper_bound`].
    fn meet_is_greatest_lower_bound {
        let ab = a & b;
        let lower = &ab & c;
        lower <= a && lower <= b && lower <= ab
    }

    /// Meet distributes over join: `a & (b | c) == (a & b) | (a & c)`. The
    /// version lattice embeds in a function space into the chain of naturals
    /// (pointwise min/max), so it is distributive.
    fn meet_distributes_over_join {
        (a & &(b | c)) == (&(a & b) | &(a & c))
    }

    /// Join distributes over meet: `a | (b & c) == (a | b) & (a | c)`, the dual
    /// law (in a lattice each distributive law implies the other; asserting
    /// both guards an impl that realized only one direction).
    fn join_distributes_over_meet {
        (a | &(b & c)) == (&(a | b) & &(a | c))
    }

    /// Transitivity on a constructed chain: `a <= a|b <= a|b|c` holds by the
    /// upper-bound law, so the endpoints must compare — arbitrary inputs rarely
    /// chain by chance, so the chain is built rather than awaited.
    fn order_transitive_constructed {
        let mid = a | b;
        let hi = &mid | c;
        a <= mid && mid <= hi && a <= hi
    }

    /// Transitivity, incidental: whenever three arbitrary versions happen to
    /// chain (`a <= b` and `b <= c`), the endpoints must too.
    fn order_transitive_incidental {
        !(a <= b && b <= c) || a <= c
    }

    /// The triangle inequality: `d(a, c) <= d(a, b) + d(b, c)` — the defining
    /// metric law, which holds because the strictly monotone valuation `rank`
    /// lives on a *distributive* lattice.
    fn distance_triangle_inequality {
        a.distance(c) <= a.distance(b) + b.distance(c)
    }

    /// The directed triangle inequality: `a.lag(c) <= a.lag(b) + b.lag(c)` —
    /// the quasi-metric law for the directed half of `distance`.
    ///
    /// Holds by rank's modularity (`rank_is_a_valuation`) plus
    /// monotonicity: `rank(a|b) + rank(b|c) == rank(a|b|c) + rank((a|b) &
    /// (b|c)) >= rank(a|c) + rank(b)`, and subtracting `rank(a) + rank(b)`
    /// from both sides leaves the lags.
    fn lag_triangle_inequality {
        a.lag(c) <= a.lag(b) + b.lag(c)
    }

    /// `lag` is monotone in the message: a larger message leaves at least as
    /// much to learn.
    ///
    /// Constructed on `b <= b | c` (so the comparable pair exists on every
    /// call), and incidentally whenever the operands happen to compare —
    /// `b <= c ⟹ a.lag(b) <= a.lag(c)`, by the join's monotonicity under
    /// the rank valuation.
    fn lag_monotone_in_the_message {
        let constructed = a.lag(b) <= a.lag(&(b | c));
        let ordered = b <= c;
        let incidental = !ordered || a.lag(b) <= a.lag(c);
        constructed && incidental
    }

    /// `lag` is antitone in the receiver: learning more leaves less to
    /// learn.
    ///
    /// Constructed on `a <= a | c`, and incidentally whenever the operands
    /// happen to compare — `a <= a' ⟹ a'.lag(b) <= a.lag(b)`, by rank's
    /// modularity applied to `u = a | b`, `v = a'` (the meet `(a|b) & a'`
    /// dominates `a`, so the gap can only shrink).
    fn lag_antitone_in_the_receiver {
        let constructed = (a | c).lag(b) <= a.lag(b);
        let ordered = a <= c;
        let incidental = !ordered || c.lag(b) <= a.lag(b);
        constructed && incidental
    }

    /// Conjunction is pointwise intersection, commutatively, across every
    /// atomic operand pairing.
    ///
    /// `(x & y).contains(p)` equals `x.contains(p) && y.contains(p)` for every
    /// pair drawn from the atomic queries at two versions, in both operand
    /// orders. This is the behavioral pin on the whole merge kernel: floor
    /// joins, ceiling meets, hole absorption, vacuity pruning, and the
    /// strictness normalization all sit between `&` and `contains`, so any of
    /// them changing what a query admits diverges here. Probes include the
    /// operands' meet and join, reaching the at-bound corners on every call.
    fn conjunction_is_intersection {
        let (meet, join) = (b & c, b | c);
        let probes = [a, b, c, &meet, &join];
        /// One polarity-homogeneous double loop of the pointwise check.
        macro_rules! check {
            ($xs:expr, $ys:expr) => {
                for x in &$xs {
                    for y in &$ys {
                        let xy = x.clone() & y.clone();
                        let yx = y.clone() & x.clone();
                        for p in probes {
                            let want = x.contains(p) && y.contains(p);
                            if xy.contains(p) != want || yx.contains(p) != want {
                                return false;
                            }
                        }
                    }
                }
            };
        }
        check!(neutral_queries(b, c), neutral_queries(c, b));
        check!(down_queries(b, c), down_queries(c, b));
        check!(up_queries(b, c), up_queries(c, b));
        check!(neutral_queries(b, c), down_queries(c, b));
        check!(neutral_queries(b, c), up_queries(c, b));
        true
    }

    /// The typed `&` matrix lands in one predicate: wherever a conjunction
    /// lands in the type census (atom, bound, or query), it admits exactly the
    /// intersection of its operands.
    ///
    /// One equation per distinct merge path: the two elementary same-side
    /// collapses (which stay atoms, exercising the strictness-survival rule on
    /// comparable bounds and its dissolution on concurrent ones), the two side
    /// merges, and the cross-side pairings that land in a [`Query`] — the paths
    /// every macro-generated impl delegates to.
    fn conjunction_operand_forms_agree {
        use causally::{after, before, since, strictly_after, strictly_before};
        let (meet, join) = (b & c, b | c);
        let probes = [a, b, c, &meet, &join];
        for p in probes {
            let atoms = (after(b) & after(c)).contains(p)
                == (after(b).contains(p) && after(c).contains(p))
                && (before(b) & before(c)).contains(p)
                    == (before(b).contains(p) && before(c).contains(p))
                && (after(b) & before(c)).contains(p)
                    == (after(b).contains(p) && before(c).contains(p));
            let down = (since(b) & since(c)).contains(p)
                == (since(b).contains(p) && since(c).contains(p))
                && (after(b) & since(c)).contains(p)
                    == (after(b).contains(p) && since(c).contains(p))
                && (strictly_after(b) & strictly_after(c)).contains(p)
                    == (strictly_after(b).contains(p) && strictly_after(c).contains(p));
            let up = ((!after(b)) & (!after(c))).contains(p)
                == ((!after(b)).contains(p) && (!after(c)).contains(p))
                && (before(b) & (!after(c))).contains(p)
                    == (before(b).contains(p) && (!after(c)).contains(p))
                && (strictly_before(b) & strictly_before(c)).contains(p)
                    == (strictly_before(b).contains(p) && strictly_before(c).contains(p));
            if !(atoms && down && up) {
                return false;
            }
        }
        true
    }

    /// Coverage's verdicts are sound over the segment: `Full` admits the
    /// constructed in-segment probes, `Empty` rejects them.
    ///
    /// Quantified over conjunctions of an atomic query at one version with a
    /// representative bound at another, against the constructed ordered,
    /// coincident, and incidental spans; probes are the endpoints and `(lo | x)
    /// & hi` — a version within the segment by construction — so both sound
    /// arms are exercised against genuinely interior points. `Partial` promises
    /// nothing pointwise: the [`Coverage`] docs carry the precision contract,
    /// including why `Empty` cannot be complete.
    fn coverage_bounds_membership {
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                // Every candidate is ordered by construction or admission.
                return false;
            };
            let mid = &(lo | a) & hi;
            let probes = [lo, hi, &mid];
            /// One family's soundness check over the span.
            macro_rules! check {
                ($qs:expr) => {
                    for q in &$qs {
                        match q.coverage(span.reborrow()) {
                            Coverage::Full => {
                                if probes.iter().any(|p| !q.contains(p)) {
                                    return false;
                                }
                            }
                            Coverage::Empty => {
                                if probes.iter().any(|p| q.contains(p)) {
                                    return false;
                                }
                            }
                            Coverage::Partial => {}
                        }
                    }
                };
            }
            check!(neutral_queries(a, b));
            check!(down_queries(a, b));
            check!(up_queries(a, b));
        }
        true
    }

    /// Coverage of a coincident span is membership: `Full` for a member,
    /// `Empty` otherwise — `Partial` is unreachable when the segment is one
    /// version — through both the span entry point and the version entry point.
    fn coverage_matches_membership_on_points {
        /// One family's point-degeneracy check.
        macro_rules! check {
            ($qs:expr, $probes:expr) => {
                for q in &$qs {
                    for p in $probes {
                        let want = if q.contains(p) {
                            Coverage::Full
                        } else {
                            Coverage::Empty
                        };
                        if q.coverage(p) != want || q.coverage(Span::at(p)) != want {
                            return false;
                        }
                    }
                }
            };
        }
        check!(neutral_queries(b, c), [a, b, c]);
        check!(down_queries(b, c), [a, b, c]);
        check!(up_queries(b, c), [a, b, c]);
        true
    }

    /// `Span::place` is exactly the two causal comparisons against the
    /// endpoints: the nine-state verdict is a pure transcription of `(probe vs
    /// lo, probe vs hi)`.
    ///
    /// Checked over the constructed ordered, coincident, and incidental span
    /// pairs, probing each operand and the pair's meet (which reaches the
    /// at-endpoint and `lo == hi` corners on every call).
    fn span_place_matches_relations {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                // Every candidate is ordered by construction or admission.
                return false;
            };
            for probe in [a, b, c, &meet] {
                if span.place(probe) != place_from_relations(lo, hi, probe) {
                    return false;
                }
            }
        }
        true
    }

    /// `dominance` is `place` coarsened to the dominance question, on every
    /// span.
    ///
    /// `Dominance::After` collects the verdicts with `hi <= p` (`At(End)`,
    /// `At(Both)`, `After`), `Dominance::Between` those with `lo <= p` but not
    /// `hi <= p` (`At(Start)`, `Between`, `Concurrent(End)`), and
    /// `Dominance::Before` the rest (`Before`, `Concurrent(Start)`,
    /// `Concurrent(Both)`).
    fn span_dominance_coarsens_place {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                return false;
            };
            for probe in [a, b, c, &meet] {
                let coarse = match span.place(probe) {
                    Placement::At(Endpoint::End | Endpoint::Both) | Placement::After => {
                        Dominance::After
                    }
                    Placement::At(Endpoint::Start)
                    | Placement::Between
                    | Placement::Concurrent(Endpoint::End) => Dominance::Between,
                    Placement::Before | Placement::Concurrent(Endpoint::Start | Endpoint::Both) => {
                        Dominance::Before
                    }
                };
                if span.dominance(probe) != coarse {
                    return false;
                }
            }
        }
        true
    }

    /// `precedence` is `place` coarsened to the precedence question — the
    /// dominance coarsening's dual, mirrored bucket by bucket — on every span.
    ///
    /// `Precedence::Before` collects the verdicts with `p <= lo` (`Before`,
    /// `At(Start)`, `At(Both)`), `Precedence::Between` those with `p <= hi` but
    /// not `p <= lo` (`At(End)`, `Between`, `Concurrent(Start)`), and
    /// `Precedence::After` the rest (`After`, `Concurrent(End)`,
    /// `Concurrent(Both)`).
    fn span_precedence_coarsens_place {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                return false;
            };
            for probe in [a, b, c, &meet] {
                let coarse = match span.place(probe) {
                    Placement::At(Endpoint::Start | Endpoint::Both) | Placement::Before => {
                        Precedence::Before
                    }
                    Placement::At(Endpoint::End)
                    | Placement::Between
                    | Placement::Concurrent(Endpoint::Start) => Precedence::Between,
                    Placement::After | Placement::Concurrent(Endpoint::End | Endpoint::Both) => {
                        Precedence::After
                    }
                };
                if span.precedence(probe) != coarse {
                    return false;
                }
            }
        }
        true
    }

    /// `contains` is segment membership: `lo <= p <= hi`, exactly the
    /// placements at an endpoint or between them — a `Concurrent` placement is
    /// beside the segment, never within it.
    ///
    /// Every argument shape answers alike: a borrowed version, an owned
    /// version, the coincident span at the probe (the fast path), and
    /// byte-equal coincident endpoints in distinct buffers (the general
    /// arm). A span argument is contained iff both its endpoints place
    /// within — the containment order — with the borrowed and owned span
    /// entry points agreeing.
    fn span_contains_matches_place {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                return false;
            };
            for probe in [a, b, c, &meet] {
                let inside = matches!(span.place(probe), Placement::At(_) | Placement::Between);
                if span.contains(probe) != inside {
                    return false;
                }
                // The membership argument shapes: owned version, coincident
                // span, and
                // the same endpoints re-decoded into distinct buffers (which
                // routes the general endpoint comparisons, not the fused
                // walk).
                let redecoded =
                    Version::decode(&probe.encode()[..]).expect("a stored stream re-decodes");
                let consistent = span.contains(probe.clone()) == inside
                    && span.contains(Span::at(probe)) == inside
                    && span.contains(Span::new(probe, &redecoded).unwrap()) == inside;
                if !consistent {
                    return false;
                }
            }
            // A span argument is contained iff both its endpoints are:
            // every version between them rides the endpoint bounds by
            // transitivity.
            for (tlo, thi) in &span_candidates(a, c) {
                let Ok(t) = Span::new(tlo, thi) else {
                    return false;
                };
                let endpoints_within = within(&span, tlo) && within(&span, thi);
                if span.contains(&t) != endpoints_within
                    || span.contains(t.clone()) != endpoints_within
                {
                    return false;
                }
            }
        }
        true
    }

    /// A span's segment, as a query, is exactly span placement's contained
    /// region: `Query::from(&span).contains(p)` iff [`Span::place`] puts `p` at
    /// an endpoint or between them.
    ///
    /// The tie between the two constructions: a span is the concrete pair, and
    /// its segment-as-predicate is the `after(lo) & before(hi)` cell of the
    /// query language — every other placement verdict (outside either endpoint,
    /// or concurrent to one) is exactly non-membership.
    fn segment_query_matches_span_place {
        let meet = b & c;
        for (lo, hi) in &span_candidates(b, c) {
            let Ok(span) = Span::new(lo, hi) else {
                // Every candidate is ordered by construction or admission.
                return false;
            };
            let q = Query::from(&span);
            for probe in [a, b, c, &meet] {
                let inside = matches!(span.place(probe), Placement::At(_) | Placement::Between);
                if q.contains(probe) != inside {
                    return false;
                }
            }
        }
        true
    }

    /// Span composite encodings are prefix-free: distinct spans' encodings
    /// are never byte prefixes of one another.
    ///
    /// Pinned directly on the composite (it rides the components'
    /// prefix-freedom, but the pin is on the composite itself, never
    /// inferred). Prefix-freedom is what lets one composite self-delimit
    /// inside a larger stream: the borsh leg reads exactly one span and
    /// leaves the next field's bytes unread. The quantified spans share
    /// endpoints across the two operand families, so byte-prefix-adjacent
    /// composites (equal meets under differing joins) arise on every call.
    fn span_encoding_is_prefix_free {
        let mut spans = operand_spans(a, b);
        spans.extend(operand_spans(b, c));
        for (i, x) in spans.iter().enumerate() {
            for y in &spans[i + 1..] {
                if x == y {
                    continue;
                }
                let (ex, ey) = (x.encode(), y.encode());
                if ex.starts_with(&ey) || ey.starts_with(&ex) {
                    return false;
                }
            }
        }
        true
    }

    /// `+` is the containment join: endpoints definitionally the meet of the
    /// meets and the join of the joins.
    ///
    /// The same span from either operand order, from every owned/borrowed
    /// cell (`+=` included: assigning is the value operator written back),
    /// and from the `union` method spelling; idempotent, and covering both
    /// operands' whole segments. A version operand is its coincident point
    /// span, in the binary, assigning, and variadic forms alike.
    // The idempotence probes repeat an operand on purpose: `s + s == s` is
    // the law itself, not a typo the lint should flag.
    #[allow(clippy::eq_op)]
    fn span_union_is_the_containment_join {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                let union = s + t;
                let definitional =
                    union == Span::new(&(s.lo() & t.lo()), &(s.hi() | t.hi())).unwrap();
                let commutative = union == t + s;
                let idempotent = (s + s) == *s;
                let cells = (s.clone() + t.clone()) == union
                    && (s.clone() + t) == union
                    && (s + t.clone()) == union;
                let assigns = {
                    let mut by_ref = s.clone();
                    by_ref += t;
                    let mut by_value = s.clone();
                    by_value += t.clone();
                    by_ref == union && by_value == union
                };
                let method = s.union(t) == union;
                let covering = [s.lo(), s.hi(), t.lo(), t.hi()]
                    .into_iter()
                    .all(|v| within(&union, v));
                if !(definitional
                    && commutative
                    && idempotent
                    && cells
                    && assigns
                    && method
                    && covering)
                {
                    return false;
                }
            }
            // A version operand is its coincident point span, on either
            // side of the symbol, in the binary, assigning, and variadic
            // forms alike.
            let expected = s + &Span::at(a);
            let mut assigned = s.clone();
            assigned += a;
            let point = (s + a) == expected
                && (a + s) == expected
                && (a.clone() + s.clone()) == expected
                && s.union(a) == expected
                && s.union_all([a]) == expected
                && assigned == expected;
            if !point {
                return false;
            }
        }
        true
    }

    /// `*` is the containment meet: the joined meets under the met joins
    /// when that pair orders, [`None`] exactly otherwise.
    ///
    /// Commutative, idempotent, absorbing with `+`, the `intersect` method
    /// spelling exactly, and containing every version both operands contain
    /// (so two overlapping operands always intersect).
    // The idempotence probe repeats an operand on purpose: `s * s` is the law.
    #[allow(clippy::eq_op)]
    fn span_intersect_is_the_shared_segment {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                let inter = s * t;
                let definitional = inter
                    == Span::new(&(s.lo() | t.lo()), &(s.hi() & t.hi()))
                        .ok()
                        .map(Span::into_owned);
                let commutative = inter == (t * s);
                let idempotent = (s * s) == Some(s.clone());
                let cells = (s.clone() * t.clone()) == inter
                    && (s.clone() * t) == inter
                    && (s * t.clone()) == inter;
                let method = s.intersect(t) == inter;
                let absorbing = {
                    let u = s + t;
                    (s * &u) == Some(s.clone())
                };
                let membership = [a, b, c].into_iter().all(|probe| {
                    !(within(s, probe) && within(t, probe))
                        || inter.as_ref().is_some_and(|i| within(i, probe))
                });
                if !(definitional
                    && commutative
                    && idempotent
                    && cells
                    && method
                    && absorbing
                    && membership)
                {
                    return false;
                }
            }
        }
        true
    }

    /// `|` is the pointwise join: endpoints definitionally the joins of the
    /// corresponding endpoints.
    ///
    /// Commutative, idempotent, with the coincident empty span as identity,
    /// every owned/borrowed cell (`|=` included: assigning is the value
    /// operator written back), the `join` method spelling exactly — and on
    /// coincident operands it restricts to the version join exactly (the
    /// lifting is a lattice homomorphism on points, where `+` yields the
    /// hull instead). A version operand is its coincident point span, in
    /// the binary, assigning, and variadic forms alike.
    // The idempotence probe repeats an operand on purpose: `s | s == s` is
    // the law itself.
    #[allow(clippy::eq_op)]
    fn span_join_is_the_pointwise_join {
        let empty = Version::new();
        let identity = empty.span(&empty);
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                let join = s | t;
                let definitional =
                    join == Span::new(&(s.lo() | t.lo()), &(s.hi() | t.hi())).unwrap();
                let commutative = join == (t | s);
                let idempotent = (s | s) == *s;
                let cells = (s.clone() | t.clone()) == join
                    && (s.clone() | t) == join
                    && (s | t.clone()) == join;
                let assigns = {
                    let mut by_ref = s.clone();
                    by_ref |= t;
                    let mut by_value = s.clone();
                    by_value |= t.clone();
                    by_ref == join && by_value == join
                };
                let method = s.join(t) == join;
                let identity_holds = (s | &identity) == *s;
                if !(definitional
                    && commutative
                    && idempotent
                    && cells
                    && assigns
                    && method
                    && identity_holds)
                {
                    return false;
                }
            }
            // A version operand is its coincident point span, on either
            // side of the symbol, in the binary, assigning, and variadic
            // forms alike.
            let expected = s | &Span::at(a);
            let mut assigned = s.clone();
            assigned |= a;
            let point = (s | a) == expected
                && (a | s) == expected
                && (a.clone() | s.clone()) == expected
                && s.join(a) == expected
                && s.join_all([a]) == expected
                && assigned == expected;
            if !point {
                return false;
            }
        }
        // The point identity: two coincident spans join to the coincident
        // span at their versions' join.
        let bc = b | c;
        (b.span(b) | c.span(c)) == bc.span(&bc)
    }

    /// `&` is the pointwise meet: endpoints definitionally the meets of the
    /// corresponding endpoints.
    ///
    /// Commutative, idempotent, absorbing with `|` in the pointwise lattice,
    /// every owned/borrowed cell (`&=` included: assigning is the value
    /// operator written back), the `meet` method spelling exactly — and on
    /// coincident operands it restricts to the version meet exactly. A
    /// version operand is its coincident point span, in the binary,
    /// assigning, and variadic forms alike.
    // The idempotence probe repeats an operand on purpose: `s & s == s` is
    // the law itself.
    #[allow(clippy::eq_op)]
    fn span_meet_is_the_pointwise_meet {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                let meet = s & t;
                let definitional =
                    meet == Span::new(&(s.lo() & t.lo()), &(s.hi() & t.hi())).unwrap();
                let commutative = meet == (t & s);
                let idempotent = (s & s) == *s;
                let cells = (s.clone() & t.clone()) == meet
                    && (s.clone() & t) == meet
                    && (s & t.clone()) == meet;
                let assigns = {
                    let mut by_ref = s.clone();
                    by_ref &= t;
                    let mut by_value = s.clone();
                    by_value &= t.clone();
                    by_ref == meet && by_value == meet
                };
                let method = s.meet(t) == meet;
                let absorbing = {
                    let u = s | t;
                    (&u & s) == *s
                };
                if !(definitional
                    && commutative
                    && idempotent
                    && cells
                    && assigns
                    && method
                    && absorbing)
                {
                    return false;
                }
            }
            // A version operand is its coincident point span, on either
            // side of the symbol, in the binary, assigning, and variadic
            // forms alike.
            let expected = s & &Span::at(a);
            let mut assigned = s.clone();
            assigned &= a;
            let point = (s & a) == expected
                && (a & s) == expected
                && (a.clone() & s.clone()) == expected
                && s.meet(a) == expected
                && s.meet_all([a]) == expected
                && assigned == expected;
            if !point {
                return false;
            }
        }
        // The point identity: two coincident spans meet to the
        // coincident span at their versions' meet.
        let bc = b & c;
        (b.span(b) & c.span(c)) == bc.span(&bc)
    }

    /// `+` (the containment join) is associative: `(s + t) + u == s + (t + u)`.
    ///
    /// Componentwise the version lattice's meet on the lows and join on the
    /// his, each associative, so both association orders build the same span
    /// — the associativity half of the containment lattice the span docs
    /// claim.
    fn span_union_associative {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                for u in &operand_spans(a, c) {
                    if (&(s + t) + u) != (s + &(t + u)) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// `*` (the containment meet) is associative over outcomes: both
    /// association orders agree in definedness and, where defined, in value.
    ///
    /// Both orders are defined exactly when the joint condition
    /// `lo_s | lo_t | lo_u <= hi_s & hi_t & hi_u` holds — the joint condition
    /// implies every pairwise one (the pairwise join is below the joint join,
    /// the pairwise meet above the joint meet), so neither order can fail an
    /// inner intersection where the other survives; the same shape as
    /// `join_associative_outcomes` on the partial monoid.
    fn span_intersect_associative_outcomes {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                for u in &operand_spans(a, c) {
                    let left = (s * t).and_then(|st| &st * u);
                    let right = (t * u).and_then(|tu| s * &tu);
                    if left != right {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// `|` (the pointwise join) is associative: `(s | t) | u == s | (t | u)`.
    ///
    /// Componentwise the version join on both endpoint pairs, so span `|`
    /// inherits its associativity — the associativity half of the pointwise
    /// lattice the span docs claim.
    fn span_join_associative {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                for u in &operand_spans(a, c) {
                    if (&(s | t) | u) != (s | &(t | u)) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// `&` (the pointwise meet) is associative: `(s & t) & u == s & (t & u)`,
    /// the dual of [`span_join_associative`].
    fn span_meet_associative {
        for s in &operand_spans(a, b) {
            for t in &operand_spans(b, c) {
                for u in &operand_spans(a, c) {
                    if (&(s & t) & u) != (s & &(t & u)) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// The hole-free queries over a version pair: the unbounded query, each atom
/// alone, and the interval — the [`causally::Neutral`] fragment's shapes.
fn neutral_queries<'a>(b: &'a Version, c: &'a Version) -> Vec<Query<'a>> {
    vec![
        causally::all(),
        causally::after(b).into(),
        causally::before(c).into(),
        causally::after(b) & causally::before(c),
    ]
}

/// Down-polar queries over a version pair: every down-hole spelling — negation,
/// widening, strict floor — alone and conjoined, so the merge kernel's
/// absorption and pruning arms are all reached.
fn down_queries<'a>(b: &'a Version, c: &'a Version) -> Vec<Query<'a, causally::Down>> {
    vec![
        causally::since(b),
        causally::since(c),
        causally::strictly_after(b),
        causally::after(b).or_concurrent(),
        causally::delta(b, c),
        causally::since(b) & causally::since(c),
        causally::after(b) & causally::since(c),
    ]
}

/// Up-polar queries over a version pair, dually to [`down_queries`].
fn up_queries<'a>(b: &'a Version, c: &'a Version) -> Vec<Query<'a, causally::Up>> {
    vec![
        !causally::after(b),
        !causally::after(c),
        causally::strictly_before(b),
        causally::before(b).or_concurrent(),
        (!causally::after(b)) & (!causally::after(c)),
        causally::before(b) & (!causally::after(c)),
        causally::after(b) & (!causally::after(c)),
    ]
}

/// The span pairs the placement laws quantify over, from a version pair.
///
/// The constructed always-ordered pair (`meet <= join`), the coincident pair
/// (reaching `lo == hi` on every call), and the raw pair whenever it happens to
/// order.
fn span_candidates(b: &Version, c: &Version) -> Vec<(Version, Version)> {
    let (meet, join) = (b & c, b | c);
    let mut out = vec![(meet.clone(), join), (meet.clone(), meet)];
    if b <= c {
        out.push((b.clone(), c.clone()));
    }
    out
}

/// [`Placement`], transcribed from the two raw causal comparisons against the
/// endpoints — the nine-state table stated relation by relation, with the start
/// relation examined first.
fn place_from_relations(lo: &Version, hi: &Version, p: &Version) -> Placement {
    match p.partial_cmp(lo) {
        Some(Ordering::Less) => Placement::Before,
        Some(Ordering::Equal) => match p.partial_cmp(hi) {
            Some(Ordering::Equal) => Placement::At(Endpoint::Both),
            _ => Placement::At(Endpoint::Start),
        },
        Some(Ordering::Greater) => match p.partial_cmp(hi) {
            Some(Ordering::Less) => Placement::Between,
            Some(Ordering::Equal) => Placement::At(Endpoint::End),
            Some(Ordering::Greater) => Placement::After,
            None => Placement::Concurrent(Endpoint::End),
        },
        None => match p.partial_cmp(hi) {
            None => Placement::Concurrent(Endpoint::Both),
            _ => Placement::Concurrent(Endpoint::Start),
        },
    }
}

// ─────────────────────────── the span algebra ───────────────────────────

/// The valid spans the operator laws quantify over, from a version pair: the
/// pair's hull, the coincident span at the meet, and the raw pair whenever it
/// happens to order — settled owned so the operand cells can consume them.
fn operand_spans(b: &Version, c: &Version) -> Vec<Span<'static>> {
    span_candidates(b, c)
        .into_iter()
        .map(|(lo, hi)| {
            Span::new(&lo, &hi)
                .expect("every candidate is ordered")
                .into_owned()
        })
        .collect()
}

/// Whether `probe` lies on the span's chain segment (`lo <= p <= hi`): the
/// order-sense membership the containment operators speak about. A `Concurrent`
/// placement is *beside* the span, never within it.
fn within(span: &Span<'_>, probe: &Version) -> bool {
    matches!(span.place(probe), Placement::At(_) | Placement::Between)
}

// ───────────────────────────── Version: lists ─────────────────────────────
