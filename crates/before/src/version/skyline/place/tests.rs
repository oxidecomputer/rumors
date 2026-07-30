use proptest::prelude::*;

use super::*;
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::arb_oracle_version;
use crate::version::skyline::sweep;
use crate::{Clock, Version};

/// The composed two-sweep spelling of the range mode: the walk's
/// stream-level oracle.
fn composed(probe: &Version, start: Option<&Version>, end: Option<&Version>) -> Ranged {
    if let Some(start) = start {
        match sweep::causal_cmp(probe.view(), start.view()) {
            Some(Ordering::Less) => return Ranged::BelowStart,
            Some(Ordering::Equal) => return Ranged::AtStart,
            Some(Ordering::Greater) | None => {}
        }
    }
    match end {
        None => Ranged::Inside,
        Some(end) => match sweep::causal_cmp(probe.view(), end.view()) {
            Some(Ordering::Less) => Ranged::Inside,
            Some(Ordering::Equal) => Ranged::AtEnd,
            Some(Ordering::Greater) => Ranged::AboveEnd,
            None => Ranged::ConcurrentToEnd,
        },
    }
}

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

/// One fused range placement, from versions.
fn placed(probe: &Version, start: Option<&Version>, end: Option<&Version>) -> Ranged {
    range(
        probe.view(),
        start.map(|v| &**v.view()),
        end.map(|v| &**v.view()),
    )
}

/// Every range verdict on an organic witness set, including the
/// start-drop path (probe concurrent to the start, end relation still to
/// decide) and the coincident-bounds corner (`AtStart` wins).
#[test]
fn walk_places_organic_witnesses() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let a3 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let joined = &a3 | &b1;

    assert_eq!(
        placed(&Version::new(), Some(&a1), Some(&a3)),
        Ranged::BelowStart
    );
    assert_eq!(placed(&a1, Some(&a1), Some(&a3)), Ranged::AtStart);
    assert_eq!(placed(&a2, Some(&a1), Some(&a3)), Ranged::Inside);
    assert_eq!(placed(&a3, Some(&a1), Some(&a3)), Ranged::AtEnd);
    assert_eq!(placed(&joined, Some(&a1), Some(&a3)), Ranged::AboveEnd);
    assert_eq!(placed(&b1, Some(&a1), Some(&a3)), Ranged::ConcurrentToEnd);
    // The start-drop path: concurrent to the start, then each end verdict.
    assert_eq!(placed(&b1, Some(&a1), Some(&joined)), Ranged::Inside);
    assert_eq!(placed(&b1, Some(&a1), None), Ranged::Inside);
    assert_eq!(placed(&b1, Some(&a1), Some(&b1)), Ranged::AtEnd);
    // Coincident bounds: the start relation speaks first.
    assert_eq!(placed(&a1, Some(&a1), Some(&a1)), Ranged::AtStart);
    // No bounds: everything is inside, at zero cost.
    assert_eq!(placed(&a1, None, None), Ranged::Inside);
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

proptest! {
    /// The fused range walk equals the composed two-sweep spelling.
    ///
    /// Checked on every stream triple whose bounds a validated range
    /// could hold (either side absent, or `start <= end` — constructed
    /// via meet/join when the generated pair does not already compose),
    /// for probes spanning all three operands and their lattice corners.
    #[test]
    fn walk_matches_the_composed_sweeps(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&a);
        let b = from_oracle_version(&b);
        let c = from_oracle_version(&c);
        let (meet, join) = (&b & &c, &b | &c);

        let mut pairs: Vec<(Option<&Version>, Option<&Version>)> = vec![
            (None, None),
            (Some(&b), None),
            (None, Some(&c)),
            (Some(&meet), Some(&join)),
            (Some(&meet), Some(&meet)),
        ];
        // The generated pair itself, when it happens to compose.
        if matches!(
            b.partial_cmp(&c),
            Some(Ordering::Less | Ordering::Equal)
        ) {
            pairs.push((Some(&b), Some(&c)));
        }

        for (start, end) in pairs {
            for probe in [&a, &b, &c, &meet, &join] {
                prop_assert_eq!(
                    placed(probe, start, end),
                    composed(probe, start, end),
                    "fused walk vs composed sweeps at start={:?} end={:?}",
                    start.is_some(),
                    end.is_some(),
                );
            }
        }
    }

    /// The fused span and dominance walks equal their composed
    /// two-sweep spellings.
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
            }
        }
    }
}
