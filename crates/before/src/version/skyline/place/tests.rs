use proptest::prelude::*;

use super::filter::{self, Demand};
use super::*;
use crate::causally::Coverage;
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::arb_oracle_version;
use crate::version::skyline::sweep;
use crate::{Clock, Version};

/// The composed two-sweep spelling of the span mode: the nine-state
/// verdict transcribed from the raw relations.
fn composed_span(probe: &Version, lo: &Version, hi: &Version) -> Placement {
    let lo_rel = sweep::causal_cmp(probe.view(), lo.view());
    let hi_rel = sweep::causal_cmp(probe.view(), hi.view());
    match lo_rel {
        Some(Ordering::Less) => Placement::Before,
        Some(Ordering::Equal) => match hi_rel {
            Some(Ordering::Equal) => Placement::At(Endpoint::Both),
            _ => Placement::At(Endpoint::Start),
        },
        Some(Ordering::Greater) => match hi_rel {
            Some(Ordering::Less) => Placement::Between,
            Some(Ordering::Equal) => Placement::At(Endpoint::End),
            Some(Ordering::Greater) => Placement::After,
            None => Placement::Concurrent(Endpoint::End),
        },
        None => match hi_rel {
            None => Placement::Concurrent(Endpoint::Both),
            _ => Placement::Concurrent(Endpoint::Start),
        },
    }
}

/// The dominance coarsening of [`composed_span`]: the mode's
/// stream-level oracle.
fn composed_dominance(probe: &Version, lo: &Version, hi: &Version) -> Dominance {
    match composed_span(probe, lo, hi) {
        Placement::At(Endpoint::End | Endpoint::Both) | Placement::After => Dominance::After,
        Placement::At(Endpoint::Start)
        | Placement::Between
        | Placement::Concurrent(Endpoint::End) => Dominance::Between,
        Placement::Before | Placement::Concurrent(Endpoint::Start | Endpoint::Both) => {
            Dominance::Before
        }
    }
}

/// The precedence coarsening of [`composed_span`]: the mode's
/// stream-level oracle, [`composed_dominance`] mirrored.
fn composed_precedence(probe: &Version, lo: &Version, hi: &Version) -> Precedence {
    match composed_span(probe, lo, hi) {
        Placement::At(Endpoint::Start | Endpoint::Both) | Placement::Before => Precedence::Before,
        Placement::At(Endpoint::End)
        | Placement::Between
        | Placement::Concurrent(Endpoint::Start) => Precedence::Between,
        Placement::After | Placement::Concurrent(Endpoint::End | Endpoint::Both) => {
            Precedence::After
        }
    }
}

/// The membership coarsening of [`composed_span`]: the mode's
/// stream-level oracle — exactly the at-endpoint and between regions.
fn composed_contains(probe: &Version, lo: &Version, hi: &Version) -> bool {
    matches!(
        composed_span(probe, lo, hi),
        Placement::At(_) | Placement::Between
    )
}

/// The composed pairwise spelling of one demand's verdict: the filter
/// walks' stream-level oracle, per bound.
fn demand_admits(probe: &Version, bound: &Version, demand: Demand) -> bool {
    let rel = sweep::causal_cmp(probe.view(), bound.view());
    let le = matches!(rel, Some(Ordering::Less | Ordering::Equal));
    let lt = rel == Some(Ordering::Less);
    let ge = matches!(rel, Some(Ordering::Greater | Ordering::Equal));
    let gt = rel == Some(Ordering::Greater);
    match demand {
        Demand::After => ge,
        Demand::Before => le,
        Demand::NotBefore => !le,
        Demand::NotStrictlyBefore => !lt,
        Demand::NotAfter => !ge,
        Demand::NotStrictlyAfter => !gt,
    }
}

/// The composed pairwise spelling of the coverage fold: per bound, the
/// segment-emptying and segment-filling conditions from the endpoint
/// relations, folded Empty-first.
fn composed_coverage(lo: &Version, hi: &Version, bounds: &[(&Version, Demand)]) -> Coverage {
    let mut full = true;
    for &(bound, demand) in bounds {
        let le = |p: &Version| {
            matches!(
                sweep::causal_cmp(p.view(), bound.view()),
                Some(Ordering::Less | Ordering::Equal)
            )
        };
        let lt = |p: &Version| sweep::causal_cmp(p.view(), bound.view()) == Some(Ordering::Less);
        let ge = |p: &Version| {
            matches!(
                sweep::causal_cmp(p.view(), bound.view()),
                Some(Ordering::Greater | Ordering::Equal)
            )
        };
        let gt = |p: &Version| sweep::causal_cmp(p.view(), bound.view()) == Some(Ordering::Greater);
        let (empties, admits_all) = match demand {
            Demand::After => (!ge(hi), ge(lo)),
            Demand::Before => (!le(lo), le(hi)),
            Demand::NotBefore => (le(hi), !le(lo)),
            Demand::NotStrictlyBefore => (lt(hi), !lt(lo)),
            Demand::NotAfter => (ge(lo), !ge(hi)),
            Demand::NotStrictlyAfter => (gt(lo), !gt(hi)),
        };
        if empties {
            return Coverage::Empty;
        }
        full &= admits_all;
    }
    if full {
        Coverage::Full
    } else {
        Coverage::Partial
    }
}

/// The demand lists the filter proptests sweep: every demand alone at
/// either bound, required-plus-hole pairs, and one mixed list carrying
/// every demand kind at once.
fn demand_lists<'a>(b: &'a Version, c: &'a Version) -> Vec<Vec<(&'a Version, Demand)>> {
    const DEMANDS: [Demand; 6] = [
        Demand::After,
        Demand::Before,
        Demand::NotBefore,
        Demand::NotStrictlyBefore,
        Demand::NotAfter,
        Demand::NotStrictlyAfter,
    ];
    let mut lists = vec![Vec::new()];
    for demand in DEMANDS {
        lists.push(vec![(b, demand)]);
        lists.push(vec![(c, demand)]);
        lists.push(vec![(b, Demand::After), (c, demand)]);
        lists.push(vec![(b, Demand::Before), (c, demand)]);
    }
    lists.push(DEMANDS.map(|demand| (b, demand)).to_vec());
    lists.push(vec![
        (b, Demand::After),
        (c, Demand::Before),
        (b, Demand::NotBefore),
        (c, Demand::NotAfter),
        (b, Demand::NotStrictlyBefore),
        (c, Demand::NotStrictlyAfter),
    ]);
    lists
}

/// Materialize a demand list at the stream layer.
fn streams<'a>(bounds: &[(&'a Version, Demand)]) -> Vec<(&'a BitsSlice, Demand)> {
    bounds
        .iter()
        .map(|&(bound, demand)| (&**bound.view(), demand))
        .collect()
}

/// Every span-mode drop and early-return path on an organic witness
/// set: a decided endpoint concurrency drops only its own cursor, and
/// the walk returns early exactly when both endpoints have refuted.
#[test]
fn span_walk_places_organic_witnesses() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let a3 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let joined = &a2 | &b1;

    let placed =
        |probe: &Version, lo: &Version, hi: &Version| span(probe.view(), lo.view(), hi.view());
    // The chain verdicts.
    assert_eq!(placed(&Version::new(), &a1, &a3), Placement::Before);
    assert_eq!(placed(&a1, &a1, &a3), Placement::At(Endpoint::Start));
    assert_eq!(placed(&a2, &a1, &a3), Placement::Between);
    assert_eq!(placed(&a3, &a1, &a3), Placement::At(Endpoint::End));
    assert_eq!(placed(&a3, &a1, &a2), Placement::After);
    assert_eq!(placed(&a1, &a1, &a1), Placement::At(Endpoint::Both));
    // The lo-drop path: concurrent to lo, still bounded by hi.
    assert_eq!(
        placed(&b1, &a2, &joined),
        Placement::Concurrent(Endpoint::Start)
    );
    // Contained under a two-line join: the lo relation still sweeps to
    // exhaustion after hi's bulk is past.
    assert_eq!(placed(&a2, &a1, &joined), Placement::Between);
    // The hi-drop path: past lo, concurrent to hi.
    let side_top = &a1 | &b1;
    assert_eq!(
        placed(&a2, &a1, &side_top),
        Placement::Concurrent(Endpoint::End)
    );
    // Both refuted: the early Concurrent(Both) return.
    assert_eq!(placed(&b1, &a1, &a3), Placement::Concurrent(Endpoint::Both));
}

/// Every precedence-walk hook path on an organic witness set: the
/// end refutation's early bail, the start refutation's drop (the
/// verdict riding the end relation alone), and the exhaustion
/// confirmations for both surviving directions.
#[test]
fn precedence_walk_verdicts_organic_witnesses() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let a3 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let joined = &a2 | &b1;

    let preceded = |probe: &Version, lo: &Version, hi: &Version| {
        precedence(probe.view(), lo.view(), hi.view())
    };
    // Exhaustion confirmations: the whole span preceded, at and below
    // the start.
    assert_eq!(preceded(&Version::new(), &a1, &a3), Precedence::Before);
    assert_eq!(preceded(&a1, &a1, &a3), Precedence::Before);
    // The start-drop path: `probe <= lo` refuted, the end relation
    // still confirming at exhaustion — comparably (`At(End)`,
    // `Between`) and concurrently (`Concurrent(Start)`).
    assert_eq!(preceded(&a2, &a1, &a3), Precedence::Between);
    assert_eq!(preceded(&a3, &a1, &a3), Precedence::Between);
    assert_eq!(preceded(&b1, &a2, &joined), Precedence::Between);
    // The early bail: `probe <= hi` refuted, comparably (above the
    // whole span) and concurrently (beside the end, and beside both).
    assert_eq!(preceded(&a3, &a1, &a2), Precedence::After);
    let side_top = &a1 | &b1;
    assert_eq!(preceded(&a2, &a1, &side_top), Precedence::After);
    assert_eq!(preceded(&b1, &a1, &a3), Precedence::After);
}

/// Every membership-walk hook path on an organic witness set: the
/// start-side and end-side bails (comparable and concurrent genres of
/// each), and the exhaustion confirmations at and between the
/// endpoints.
#[test]
fn contains_walk_verdicts_organic_witnesses() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let a3 = alice.tick().clone();
    let b1 = bob.tick().clone();

    let within =
        |probe: &Version, lo: &Version, hi: &Version| contains(probe.view(), lo.view(), hi.view());
    // Exhaustion confirmations: both endpoints and the interior.
    assert!(within(&a1, &a1, &a3));
    assert!(within(&a2, &a1, &a3));
    assert!(within(&a3, &a1, &a3));
    // The start-side bail: `lo <= probe` refuted, comparably (below
    // the span) and concurrently (beside the start, and beside both).
    assert!(!within(&Version::new(), &a1, &a3));
    let joined = &a2 | &b1;
    assert!(!within(&b1, &a2, &joined));
    assert!(!within(&b1, &a1, &a3));
    // The end-side bail: `probe <= hi` refuted, comparably (above the
    // span) and concurrently (beside the end).
    assert!(!within(&a3, &a1, &a2));
    let side_top = &a1 | &b1;
    assert!(!within(&a2, &a1, &side_top));
}

/// Every membership-walk hook path on an organic witness set.
///
/// The required-direction bail, the satisfied-hole drop, the
/// all-holes-satisfied early `true` (no required demand left), and the
/// exhaustion confirmations for inclusive and strict holes.
#[test]
fn filter_admits_organic_witnesses() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let b1 = bob.tick().clone();

    let admits = |probe: &Version, bounds: &[(&Version, Demand)]| {
        filter::admits(probe.view(), streams(bounds))
    };
    // The required-direction bail: a floor above the probe.
    assert!(!admits(&a1, &[(&a2, Demand::After)]));
    // The satisfied-hole drop, alone (early true) and beside a
    // required demand that sweeps to exhaustion.
    assert!(admits(&a2, &[(&a1, Demand::NotBefore)]));
    assert!(admits(&b1, &[(&a1, Demand::NotBefore)]));
    assert!(admits(
        &a2,
        &[(&a2, Demand::Before), (&a1, Demand::NotBefore)]
    ));
    // Exhaustion confirmations: an inclusive hole holds at its bound,
    // a strict hole admits it.
    assert!(!admits(&a1, &[(&a1, Demand::NotBefore)]));
    assert!(admits(&a1, &[(&a1, Demand::NotStrictlyBefore)]));
    assert!(!admits(&a1, &[(&a1, Demand::NotAfter)]));
    assert!(admits(&a1, &[(&a1, Demand::NotStrictlyAfter)]));
    // The empty demand list is vacuously true.
    assert!(admits(&a1, &[]));
}

/// The coverage walk's verdict arms on an organic witness set: the
/// early floor/ceiling Empty, hole-driven Empty at exhaustion, Full,
/// and the conservative Partial.
#[test]
fn filter_coverage_organic_witnesses() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let a3 = alice.tick().clone();
    let b1 = bob.tick().clone();

    let coverage = |lo: &Version, hi: &Version, bounds: &[(&Version, Demand)]| {
        filter::coverage(lo.view(), hi.view(), streams(bounds))
    };
    // A floor refuting `floor <= hi`: the early Empty, whether above
    // or concurrent to the whole segment.
    assert_eq!(coverage(&a1, &a2, &[(&a3, Demand::After)]), Coverage::Empty);
    assert_eq!(coverage(&a1, &a2, &[(&b1, Demand::After)]), Coverage::Empty);
    // A ceiling refuting `lo <= ceiling`, dually.
    assert_eq!(
        coverage(&a2, &a3, &[(&a1, Demand::Before)]),
        Coverage::Empty
    );
    // A hole covering the whole segment: Empty at exhaustion.
    assert_eq!(
        coverage(&a1, &a2, &[(&a3, Demand::NotBefore)]),
        Coverage::Empty
    );
    // Bounds admitting the whole segment: Full.
    assert_eq!(
        coverage(
            &a2,
            &a3,
            &[
                (&a1, Demand::After),
                (&a3, Demand::Before),
                (&b1, Demand::NotAfter)
            ],
        ),
        Coverage::Full
    );
    // A bound straddling the segment: Partial.
    assert_eq!(
        coverage(&a1, &a3, &[(&a2, Demand::Before)]),
        Coverage::Partial
    );
    // No bounds: Full at zero cost.
    assert_eq!(coverage(&a1, &a3, &[]), Coverage::Full);
}

proptest! {
    /// The fused span, dominance, precedence, and membership walks
    /// equal their composed two-sweep spellings.
    ///
    /// Checked on every ordered stream pair (constructed via meet/join,
    /// the coincident pair, and the generated pair when it happens to
    /// order), for probes spanning the operands and their lattice
    /// corners.
    #[test]
    fn span_walks_match_the_composed_sweeps(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&a);
        let b = from_oracle_version(&b);
        let c = from_oracle_version(&c);
        let (meet, join) = (&b & &c, &b | &c);

        let mut pairs: Vec<(&Version, &Version)> = vec![
            (&meet, &join),
            (&meet, &meet),
        ];
        if matches!(
            b.partial_cmp(&c),
            Some(Ordering::Less | Ordering::Equal)
        ) {
            pairs.push((&b, &c));
        }

        for (lo, hi) in pairs {
            for probe in [&a, &b, &c, &meet, &join] {
                prop_assert_eq!(
                    span(probe.view(), lo.view(), hi.view()),
                    composed_span(probe, lo, hi),
                    "fused span walk vs composed sweeps",
                );
                prop_assert_eq!(
                    dominance(probe.view(), lo.view(), hi.view()),
                    composed_dominance(probe, lo, hi),
                    "fused dominance walk vs composed coarsening",
                );
                prop_assert_eq!(
                    precedence(probe.view(), lo.view(), hi.view()),
                    composed_precedence(probe, lo, hi),
                    "fused precedence walk vs composed coarsening",
                );
                prop_assert_eq!(
                    contains(probe.view(), lo.view(), hi.view()),
                    composed_contains(probe, lo, hi),
                    "fused membership walk vs composed coarsening",
                );
            }
        }
    }

    /// The fused membership walk equals the composed pairwise sweeps.
    ///
    /// For every demand list — each demand kind alone at either bound,
    /// required-plus-hole pairs, and mixed lists carrying every kind —
    /// the walk's verdict is the conjunction of the per-bound
    /// verdicts, for probes spanning the operands and their lattice
    /// corners.
    #[test]
    fn filter_admits_matches_the_composed_sweeps(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&a);
        let b = from_oracle_version(&b);
        let c = from_oracle_version(&c);
        let (meet, join) = (&b & &c, &b | &c);

        for bounds in demand_lists(&b, &c) {
            for probe in [&a, &b, &c, &meet, &join] {
                let composed = bounds
                    .iter()
                    .all(|&(bound, demand)| demand_admits(probe, bound, demand));
                prop_assert_eq!(
                    filter::admits(probe.view(), streams(&bounds)),
                    composed,
                    "fused membership walk vs composed sweeps over {:?}",
                    bounds,
                );
            }
        }
    }

    /// The fused coverage walk equals the composed pairwise fold.
    ///
    /// For every demand list and every ordered segment (constructed
    /// via meet/join, the coincident pair, and the generated pair when
    /// it happens to order), the walk's verdict is the Empty-first
    /// fold of the per-bound endpoint conditions.
    #[test]
    fn filter_coverage_matches_the_composed_sweeps(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&a);
        let b = from_oracle_version(&b);
        let c = from_oracle_version(&c);
        let (meet, join) = (&a & &b, &a | &b);

        let mut segments: Vec<(&Version, &Version)> = vec![
            (&meet, &join),
            (&meet, &meet),
        ];
        if matches!(
            a.partial_cmp(&b),
            Some(Ordering::Less | Ordering::Equal)
        ) {
            segments.push((&a, &b));
        }

        for (lo, hi) in segments {
            for bounds in demand_lists(&b, &c) {
                prop_assert_eq!(
                    filter::coverage(lo.view(), hi.view(), streams(&bounds)),
                    composed_coverage(lo, hi, &bounds),
                    "fused coverage walk vs composed fold over {:?}",
                    bounds,
                );
            }
        }
    }
}
