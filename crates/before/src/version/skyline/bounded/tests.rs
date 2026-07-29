use proptest::prelude::*;

use super::*;
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::arb_oracle_version;
use crate::version::skyline::sweep;
use crate::{Clock, Version};

/// The composed two-sweep spelling: the walk's stream-level oracle.
fn composed(probe: &Version, start: Option<&Version>, end: Option<&Version>) -> Placement {
    if let Some(start) = start {
        match sweep::causal_cmp(probe.view(), start.view()) {
            Some(Ordering::Less) => return Placement::BelowStart,
            Some(Ordering::Equal) => return Placement::AtStart,
            Some(Ordering::Greater) | None => {}
        }
    }
    match end {
        None => Placement::Inside,
        Some(end) => match sweep::causal_cmp(probe.view(), end.view()) {
            Some(Ordering::Less) => Placement::Inside,
            Some(Ordering::Equal) => Placement::AtEnd,
            Some(Ordering::Greater) => Placement::AboveEnd,
            None => Placement::ConcurrentToEnd,
        },
    }
}

/// One fused placement, from versions.
fn placed(probe: &Version, start: Option<&Version>, end: Option<&Version>) -> Placement {
    place(
        probe.view(),
        start.map(|v| &**v.view()),
        end.map(|v| &**v.view()),
    )
}

/// Every verdict on an organic witness set, including the start-drop path
/// (probe concurrent to the start, end relation still to decide) and the
/// coincident-bounds corner (`AtStart` wins).
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
        Placement::BelowStart
    );
    assert_eq!(placed(&a1, Some(&a1), Some(&a3)), Placement::AtStart);
    assert_eq!(placed(&a2, Some(&a1), Some(&a3)), Placement::Inside);
    assert_eq!(placed(&a3, Some(&a1), Some(&a3)), Placement::AtEnd);
    assert_eq!(placed(&joined, Some(&a1), Some(&a3)), Placement::AboveEnd);
    assert_eq!(
        placed(&b1, Some(&a1), Some(&a3)),
        Placement::ConcurrentToEnd
    );
    // The start-drop path: concurrent to the start, then each end verdict.
    assert_eq!(placed(&b1, Some(&a1), Some(&joined)), Placement::Inside);
    assert_eq!(placed(&b1, Some(&a1), None), Placement::Inside);
    assert_eq!(placed(&b1, Some(&a1), Some(&b1)), Placement::AtEnd);
    // Coincident bounds: the start relation speaks first.
    assert_eq!(placed(&a1, Some(&a1), Some(&a1)), Placement::AtStart);
    // No bounds: everything is inside, at zero cost.
    assert_eq!(placed(&a1, None, None), Placement::Inside);
}

proptest! {
    /// The fused walk equals the composed two-sweep spelling.
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
}
