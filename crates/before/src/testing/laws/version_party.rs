//! Laws connecting versions with party ownership.
//!
//! These laws check ticking, projection, and how projection behaves when a
//! party is split, joined, or compared with another party.

use super::*;

laws! {
    /// Laws over a version and a live party.
    ///
    /// The event laws (`tick` strictly advances, and only within the party's
    /// region: §4's `e' = e + f·i`), the entry points' agreement (`tick` and
    /// `ticks`, each across its two spellings), the fused multi-tick's point
    /// laws (`ticks(0)` the identity, `ticks(1)` the tick, small counts
    /// against the iterated ground truth, a fresh line realizing the tick
    /// floor at any width), the tick floor's step bounds (a tick or a run of
    /// `n` raises it by at most one or `n`), and the projection (`/`) point
    /// laws.
    pub static VERSION_PARTY: (a: &Version, p: &Party);
    roster version_party_laws;

    /// `tick` strictly advances the causal order: `a < a.tick(p)`.
    fn tick_strictly_advances {
        let mut ticked = a.clone();
        ticked.tick(p);
        a < ticked
    }

    /// `tick` inflates only within the party's region (§4: `e' = e + f·i`, zero
    /// outside `i`): projected onto the region's complement, the ticked version
    /// is unchanged. Vacuous only for the seed party, which has no complement.
    fn tick_only_inflates_the_region {
        let mut ticked = a.clone();
        ticked.tick(p);
        match Party::seed().without(p) {
            None => true, // p owns the whole interval: nothing lies outside it
            Some(rest) => (&ticked / &rest) == (a / &rest),
        }
    }

    /// `tick`'s inflation is real *within* the region (§4: `f · i ⊐ 0`): the
    /// projection onto the ticking party strictly advances.
    fn tick_advances_within_the_region {
        let mut ticked = a.clone();
        ticked.tick(p);
        (a / p) < (&ticked / p)
    }

    /// The two `tick` entry points agree: `version.tick(&party)` and
    /// `party.tick(&mut version)` produce the same advance.
    fn party_tick_matches_version_tick {
        let mut via_version = a.clone();
        via_version.tick(p);
        let mut via_party = a.clone();
        p.tick(&mut via_party);
        via_version == via_party
    }

    /// `ticks(0)` is the identity: the empty run records nothing.
    fn ticks_zero_is_identity {
        let mut run = a.clone();
        run.ticks(p, 0u64);
        run == *a
    }

    /// `ticks(1)` is exactly `tick`: the fused multi-tick degenerates to the
    /// single event.
    fn ticks_one_is_tick {
        let mut fused = a.clone();
        fused.ticks(p, 1u64);
        let mut ticked = a.clone();
        ticked.tick(p);
        fused == ticked
    }

    /// `ticks(n)` equals `n` sequential `tick`s, checked at every count a
    /// short iterated run reaches (0..=3), the ground-truth seam the wide
    /// counts compose over ([`ticks_composes`] in the pair-party group).
    fn ticks_agrees_with_iterated_ticks {
        let mut iterated = a.clone();
        (0u64..=3).all(|n| {
            let mut fused = a.clone();
            fused.ticks(p, n);
            let agrees = fused == iterated;
            iterated.tick(p);
            agrees
        })
    }

    /// The two `ticks` entry points agree: `version.ticks(&party, n)` and
    /// `party.ticks(&mut version, n)` produce the same advance.
    fn party_ticks_matches_version_ticks {
        let n = a.min_ticks();
        let mut via_version = a.clone();
        via_version.ticks(p, n.clone());
        let mut via_party = a.clone();
        p.ticks(&mut via_party, n);
        via_version == via_party
    }

    /// A fresh line realizes the tick floor exactly: `n` ticks on the empty
    /// version at any one party cost floor `n`.
    ///
    /// Quantified over the wide counts the version operand's own floor
    /// supplies, all fused (no iteration at any width).
    fn ticks_line_realizes_min_ticks {
        let n = a.min_ticks();
        let mut line = Version::new();
        line.ticks(p, n.clone());
        line.min_ticks() == n
    }

    /// A tick raises the tick floor by at most one:
    /// `a.tick(p).min_ticks() <= a.min_ticks() + 1`.
    ///
    /// The bound follows from `min_ticks`'s definition as the fewest ticks of
    /// any history yielding a version. Take a history that yields `a` at some
    /// clock in `a.min_ticks()` ticks. Each of its ticks lies in that clock's
    /// causal past, since deleting one outside it would yield `a` from fewer
    /// ticks. Versions grow only by those ticks and by joins, so every version
    /// the history produces lies at or below `a`.
    /// Joining every clock into that one therefore hands it the seed and
    /// keeps its version `a`; forks and joins then carve `p` from the seed
    /// without ticking, and one tick of `p` yields `a.tick(p)`. That history
    /// performs `a.min_ticks() + 1` ticks, which bounds the successor's
    /// floor.
    ///
    /// The floor can also fall, because a tick that fills can merge regions
    /// the floor counted separately (`tick_can_lower_min_ticks` in the
    /// version tests constructs one), so the law bounds the rise alone.
    fn tick_raises_min_ticks_by_at_most_one {
        let mut ticked = a.clone();
        ticked.tick(p);
        ticked.min_ticks() <= a.min_ticks() + Count::from(1u8)
    }

    /// `ticks(n)` raises the tick floor by at most `n`:
    /// `a.ticks(p, n).min_ticks() <= a.min_ticks() + n`.
    ///
    /// The argument of [`tick_raises_min_ticks_by_at_most_one`] carries over:
    /// carve `p` after a minimal history yielding `a`, then append `n` ticks
    /// of `p`, which `ticks(p, n)` equals. The floor can fall here as there,
    /// so this law also bounds the rise alone.
    ///
    /// Quantified over the version operand's own floor and over
    /// [`wide_tick_counts`], all fused.
    fn ticks_raise_min_ticks_by_at_most_the_count {
        let floor = a.min_ticks();
        core::iter::once(floor.clone())
            .chain(wide_tick_counts())
            .all(|n| {
                let mut run = a.clone();
                run.ticks(p, n.clone());
                run.min_ticks() <= n + &floor
            })
    }

    /// Projection keeps at most the history it is given: `a / p <= a`.
    fn projection_is_sub_version {
        (a / p) <= *a
    }

    /// Projection is idempotent: `(a / p) / p == a / p`.
    ///
    /// The inner projection is materialized (idempotence quantifies over the
    /// projected *object*), and the outer one stays a view: the equality is
    /// the fused view-vs-version comparison.
    fn projection_idempotent {
        let projected = (a / p).to_version();
        (&projected / p) == projected
    }

    /// Projection is additive across a fork: a party's contribution equals the
    /// join of its two halves' contributions, the homomorphism the join/meet
    /// distribution rests on.
    ///
    /// The halves' contributions are materialized (the join needs its
    /// operands as objects), and the whole-party side stays a view: the
    /// equality is the fused version-vs-view comparison.
    fn projection_additive_over_fork {
        let mut keeper = p.dangerously_alias();
        let child = keeper.fork();
        ((a / &keeper).to_version() | (a / &child).to_version()) == (a / p)
    }

    /// The named spelling is the operator's, exactly: `a.project(p)` is
    /// `a / p`: the same view, the same materialization.
    fn project_is_the_operator_spelling {
        a.project(p) == (a / p) && a.project(p).to_version() == (a / p).to_version()
    }
}

/// Returns tick counts occupying one, two, and three 64-bit limbs: the top of
/// `u64`, just past `2^64`, and `2^129 - 2`.
///
/// A [`Count`] has no width limit, so laws over `ticks` quantify over counts
/// past a machine word as well as within one.
fn wide_tick_counts() -> [Count; 3] {
    [
        Count::from(u64::MAX),
        Count::from((1u128 << 64) + 3),
        Count::from(u128::MAX) + Count::from(u128::MAX),
    ]
}

// ───────────────────────────── Version × Version × Party ─────────────────────────────

laws! {
    /// Laws over two versions and a live party.
    ///
    /// Projection as a lattice homomorphism, its monotonicity in the
    /// version, its metric short-map property, the view's comparisons with a
    /// version at every owned and borrowed cell against the materialized
    /// projection, and `ticks` as a monoid action at the wide counts the
    /// operands' tick floors supply.
    pub static VERSION_PAIR_PARTY: (a: &Version, b: &Version, p: &Party);
    roster version_pair_party_laws;

    /// Projection is a homomorphism of the join: `(a | b) / p == (a/p) | (b/p)`
    /// (the pointwise gate commutes with pointwise max).
    ///
    /// The right-hand side's join needs its operands as objects, so the
    /// per-operand projections materialize; the left-hand side stays a view.
    fn projection_join_homomorphism {
        let joined = a | b;
        (&joined / p) == ((a / p).to_version() | (b / p).to_version())
    }

    /// Projection is a homomorphism of the meet: `(a & b) / p == (a/p) & (b/p)`.
    fn projection_meet_homomorphism {
        let met = a & b;
        (&met / p) == ((a / p).to_version() & (b / p).to_version())
    }

    /// Projection is a short map (1-Lipschitz) for the metric quantities:
    /// masking both operands to one region can only shrink `distance` and
    /// `lag`: `d(a/p, b/p) <= d(a, b)` and `lag(a/p, b/p) <= lag(a, b)`.
    ///
    /// The projection homomorphism family's metric member: a region carries
    /// `rank(v) == rank(v/p) + rank(v/p̄)` (disjoint regions carve disjoint
    /// histories, and projection is additive over a region split), so the
    /// whole metric splits into the in-region part plus the complement's,
    /// each nonnegative, so subtracting across a mask never inflates. At the
    /// seed party both sides are equal.
    fn projection_is_a_short_map {
        let (pa, pb) = ((a / p).to_version(), (b / p).to_version());
        pa.distance(&pb) <= a.distance(b) && pa.lag(&pb) <= a.lag(b)
    }

    /// Projection is monotone in the version: on the constructed comparable
    /// pair `a <= a | b`, the projections compare the same way, and whenever
    /// the inputs happen to compare directly, so do their projections.
    fn projection_monotone_in_version {
        let ab = a | b;
        let constructed = (a / p) <= (&ab / p);
        let ordered = a <= b;
        let incidental = !ordered || (a / p) <= (b / p);
        constructed && incidental
    }

    /// `ticks` is a monoid action of the naturals: `ticks(n)` then
    /// `ticks(m)` equals `ticks(n + m)`.
    ///
    /// Quantified over the wide counts the two version operands' tick
    /// floors supply, all fused, so the law exercises counts no iterated
    /// reference could reach.
    fn ticks_composes {
        let (n, m) = (a.min_ticks(), b.min_ticks());
        let mut stepwise = a.clone();
        stepwise.ticks(p, n.clone());
        stepwise.ticks(p, m.clone());
        let mut joint = a.clone();
        joint.ticks(p, n + m);
        stepwise == joint
    }

    /// The view's heterogeneous comparisons are the materialized
    /// projection's, exactly: `(a/p) ⋚ b ≡ (a/p).to_version() ⋚ b`.
    ///
    /// Checked in both operand orders, through every comparison operator, at
    /// every owned and borrowed cell of the view/version matrix. This is the
    /// three-stream differential law that checks the fused co-walk.
    // Borrowing the `Copy` view is deliberate: `&view` selects the
    // `&OwnVersion` impls, which passing the view by value would skip.
    #[allow(clippy::needless_borrows_for_generic_args)]
    fn own_version_cmp_matches_materialized {
        let view = a / p;
        let materialized = view.to_version();
        let forward = materialized.partial_cmp(b);
        let reversed = b.partial_cmp(&materialized);
        cmp_cell_agrees(view, b.clone(), forward)
            && cmp_cell_agrees(&view, b.clone(), forward)
            && cmp_cell_agrees(view, b, forward)
            && cmp_cell_agrees(&view, b, forward)
            && cmp_cell_agrees(b.clone(), view, reversed)
            && cmp_cell_agrees(b, view, reversed)
            && cmp_cell_agrees(b.clone(), &view, reversed)
            && cmp_cell_agrees(b, &view, reversed)
    }

    /// A heterogeneous comparison is the homogeneous comparison against the
    /// seed-masked view: `(a/p) ⋚ b ≡ (a/p) ⋚ (b/seed)`.
    ///
    /// Sound because projection by the seed party is the identity
    /// (`seed_projection_is_identity`), and the coherence that makes the
    /// three-stream walk a special case of the four-stream one.
    fn own_version_seed_mask_coherence {
        let view = a / p;
        let seed = Party::seed();
        let seeded = b / &seed;
        view.partial_cmp(&seeded) == view.partial_cmp(b) && (view == seeded) == (view == *b)
    }

    /// The quotient view of a span answers every verdict, and
    /// materializes, exactly as the eagerly projected span.
    ///
    /// Endpoints, the placement verdicts, the dominance and precedence
    /// coarsenings, membership, and both materialization entry points,
    /// quantified over the pair's hull and the coincident span, with probes at
    /// the operands and the projected endpoints (reaching the at-endpoint
    /// corners). Every probe built from the operands dominates the projected
    /// start (projection only shrinks a version), so the concurrent-to-start
    /// placements are this law's negative space: the span tests'
    /// `own_span_place_reaches_every_concurrent_corner` constructs them.
    ///
    /// The eager side exists at all because projection is monotone
    /// (`projection_monotone_in_version`), which this law re-witnesses by
    /// validating the projected pair through [`Span::new`].
    fn own_span_matches_the_projected_span(a, b, party) {
        let hull = a.span(b);
        let coincident = a.span(a);
        for span in [&hull, &coincident] {
            let view = span / party;
            let lo = (span.lo() / party).to_version();
            let hi = (span.hi() / party).to_version();
            let Ok(eager) = Span::new(&lo, &hi) else {
                return false; // monotone masking never crosses a valid pair
            };
            let endpoints = view.lo() == lo && view.hi() == hi;
            let materialized = view.to_span() == eager && Span::from(view) == eager;
            if !(endpoints && materialized) {
                return false;
            }
            for probe in [a, b, &lo, &hi] {
                if view.place(probe) != eager.place(probe)
                    || view.dominance(probe) != eager.dominance(probe)
                    || view.precedence(probe) != eager.precedence(probe)
                    || view.contains(probe) != eager.contains(probe)
                {
                    return false;
                }
            }
        }
        true
    }

    /// The named spelling is the operator's, exactly: `span.project(p)` is
    /// `span / p` (the same endpoint views, the same materialization),
    /// quantified over the pair's hull and the coincident span.
    fn span_project_is_the_operator_spelling(a, b, party) {
        let hull = a.span(b);
        let coincident = a.span(a);
        for span in [&hull, &coincident] {
            let named = span.project(party);
            let operator = span / party;
            if named.lo() != operator.lo()
                || named.hi() != operator.hi()
                || named.to_span() != operator.to_span()
            {
                return false;
            }
        }
        true
    }
}

// ───────────────────────────── Version × Party × Party ─────────────────────────────

laws! {
    /// Laws over a version and two live parties: projection's interaction with
    /// the region geometry.
    pub static VERSION_PARTY_PAIR: (v: &Version, p: &Party, q: &Party);
    roster version_party_pair_laws;

    /// Successive projections commute: `(v / p) / q == (v / q) / p` (both keep
    /// exactly the history on the regions' intersection).
    ///
    /// The inner projections materialize (the outer projection needs a
    /// version to gate); the outer comparison is the fused view-vs-view walk.
    fn projection_commutes {
        let vp = (v / p).to_version();
        let vq = (v / q).to_version();
        (&vp / q) == (&vq / p)
    }

    /// Projection is monotone in the region: a constructed subregion (a fork
    /// half of `p`) keeps no more than `p` does, and whenever `p` happens to
    /// cover `q`, `v / q <= v / p`.
    fn projection_monotone_in_region {
        let mut keeper = p.dangerously_alias();
        let child = keeper.fork();
        let constructed = (v / &child) <= (v / p);
        let incidental = !p.covers(q) || (v / q) <= (v / p);
        constructed && incidental
    }

    /// Disjoint regions carve disjoint histories: `p · q = 0 ⟹ (v/p) & (v/q)`
    /// is empty (the projections' supports cannot overlap).
    fn disjoint_projections_share_nothing {
        !p.is_disjoint(q) || ((v / p).to_version() & (v / q).to_version()).is_empty()
    }

    /// Ticking two disjoint parties from the same version produces different
    /// versions: each event advances history in a region the other does not
    /// own.
    fn disjoint_ticks_produce_distinct_versions {
        if !p.is_disjoint(q) {
            return true;
        }
        let mut vp = v.clone();
        vp.tick(p);
        let mut vq = v.clone();
        vq.tick(q);
        vp != vq
    }

    /// Projection is additive over any without-carved decomposition of a
    /// region: for `r = p \ q` and `inner = p \ r`, the two remainders
    /// partition `p`'s region and `(v/r) | (v/inner) == v / p`.
    ///
    /// [`projection_additive_over_fork`] states additivity for the balanced
    /// fork geometry; the ragged region pairs `without` carves are that law's
    /// negative space, and overlapping arbitrary parties keep them inhabited
    /// under mass. When `q` carves nothing out of `p` (it covers `p`, or is
    /// disjoint from it, as every pair in a one-world population, whose parties
    /// are pairwise disjoint), the law constructs the decomposition from `p`'s
    /// own fork half through the same `without` entry points instead: the
    /// additivity equation runs on every call, so no population can leave this
    /// law vacuous.
    fn projection_additive_over_carved_regions {
        let carved = p
            .dangerously_alias()
            .without(q)
            .and_then(|r| p.dangerously_alias().without(&r).map(|inner| (r, inner)));
        let (r, inner) = match carved {
            Some(pair) => pair,
            None => {
                // The constructed decomposition: carve p by its own fork
                // half, still through the without entry points under law.
                let mut keeper = p.dangerously_alias();
                let half = keeper.fork();
                let Some(r) = p.dangerously_alias().without(&half) else {
                    return false; // a proper half never covers its parent
                };
                let Some(inner) = p.dangerously_alias().without(&r) else {
                    return false; // nor does the kept half
                };
                (r, inner)
            }
        };
        ((v / &r).to_version() | (v / &inner).to_version()) == (v / p)
    }
}

// ──────────────────── Version × Version × Party × Party ────────────────────

laws! {
    /// Laws over two versions and two live parties: the homogeneous view
    /// comparison against its materialized oracle.
    pub static VERSION_PAIR_PARTY_PAIR: (a: &Version, b: &Version, p: &Party, q: &Party);
    roster version_pair_party_pair_laws;

    /// The view's homogeneous comparisons are the materialized projections',
    /// exactly: `(a/p) ⋚ (b/q) ≡ (a/p).to_version() ⋚ (b/q).to_version()`.
    ///
    /// Checked through every comparison operator, at every owned and borrowed
    /// cell. This is the four-stream differential law that checks the fused
    /// co-walk.
    // Borrowing the `Copy` views is deliberate, as in
    // `own_version_cmp_matches_materialized`.
    #[allow(clippy::needless_borrows_for_generic_args)]
    fn own_version_pair_cmp_matches_materialized {
        let (va, vb) = (a / p, b / q);
        let expected = va.to_version().partial_cmp(&vb.to_version());
        cmp_cell_agrees(va, vb, expected)
            && cmp_cell_agrees(&va, vb, expected)
            && cmp_cell_agrees(va, &vb, expected)
            && cmp_cell_agrees(&va, &vb, expected)
    }
}

// ───────────────────────────── Rank: triples ─────────────────────────────
