//! Scaling checks for arithmetic over adversarial version shapes.
//!
//! These families make one potential source of excess work grow while keeping
//! the others controlled: carry boundaries, wide height changes, retained
//! minima, or paired-version crossings. Each check combines a semantic oracle,
//! an absolute ceiling, and a liveness floor. It then compares two sizes, so a
//! live counter must show the operation's claimed per-input scaling.

use super::{min_ticks_from_big, JUMP_PAIR_MAGNITUDE_BITS};
use before::testing::meter;
use before::testing::meter::registry::Shape;
use num_bigint::BigUint;
use suanpan::touch_meter;

/// Slack numerator over the small-scale cost (denominator
/// [`SLACK_DEN`]): the ×1.25 flatness convention.
const SLACK_NUM: u64 = 5;

/// Slack denominator for the flatness bound.
const SLACK_DEN: u64 = 4;

/// One comb validation run and its touch denominator.
struct Run {
    deltas: u64,
    bytes: u64,
    touches: u64,
}

/// Assert one per-unit cost stays flat (×1.25) across a doubling.
fn assert_flat(name: &str, unit: &str, small: (u64, u64), large: (u64, u64)) {
    let (m1, n1) = small;
    let (m2, n2) = large;
    eprintln!(
        "MEASURED skyline_comb_{name}: small={m1}/{n1} large={m2}/{n2} \
         milli_per_{unit}={} -> {}",
        m1 * 1000 / n1,
        m2 * 1000 / n2,
    );
    assert!(
        u128::from(m2) * u128::from(n1) * u128::from(SLACK_DEN)
            <= u128::from(m1) * u128::from(n2) * u128::from(SLACK_NUM),
        "skyline_comb_{name}: per-{unit} cost grew more than x1.25 across the \
         size doubling: {m1}/{n1} -> {m2}/{n2}"
    );
}

/// Cliff-freedom and rank-freeze checks for version folds.
#[path = "version_scaling/cliff_and_freeze.rs"]
mod cliff_and_freeze;
/// One public fold run over a generated family shape.
struct QueryRun {
    bytes: u64,
    touches: u64,
}

/// One `Version::min_ticks` run over a generated family shape, with
/// the family's closed-form tick total as the semantic leg and the
/// one-touch-per-operand-byte liveness floor.
fn min_ticks_family_run(encoded: before::testing::meter::Encoding, expected: &BigUint) -> QueryRun {
    let v = encoded.version();
    let bytes = v.encode().len() as u64;
    touch_meter::reset();
    let ticks = v.min_ticks();
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };
    assert_eq!(
        ticks,
        min_ticks_from_big(expected),
        "min_ticks disagrees with the family's closed form"
    );
    assert!(
        run.touches >= run.bytes,
        "min_ticks at {bytes} operand bytes: {} digit touches under the \
         one-per-byte floor: the fold's accumulator work is not metered",
        run.touches,
    );
    run
}

/// Assert one two-scale reading against its absolute pinned
/// ceilings, printing the measured line re-pins read from.
fn assert_ceilings(name: &str, small: &QueryRun, large: &QueryRun, ceilings: [u64; 2]) {
    for (run, touch_ceiling, scale) in
        [(small, ceilings[0], "small"), (large, ceilings[1], "large")]
    {
        eprintln!(
            "MEASURED {name}_{scale}: bytes={} touches={}",
            run.bytes, run.touches,
        );
        assert!(
            run.touches <= touch_ceiling,
            "{name}_{scale}: {} touches exceed the pinned ceiling {touch_ceiling}",
            run.touches,
        );
    }
}

/// Flatness checks for rank and minimum-tick freeze schedules.
#[path = "version_scaling/freeze_schedules.rs"]
mod freeze_schedules;
/// Boundary and latent-state checks for minimum-tick accounting.
#[path = "version_scaling/minimum_boundaries.rs"]
mod minimum_boundaries;
/// Flatness checks for minimum-tick comb families.
#[path = "version_scaling/minimum_combs.rs"]
mod minimum_combs;
/// Flatness checks for pairwise distance and masked comparison crossings.
#[path = "version_scaling/pair_crossings.rs"]
mod pair_crossings;
/// Bands that prove accumulator skip summaries remain sufficient.
#[path = "version_scaling/skip_adequacy.rs"]
mod skip_adequacy;
