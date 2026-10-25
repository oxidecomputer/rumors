//! Heap and allocation checks for arithmetic over adversarial version shapes.
//!
//! These families make one potential source of excess memory grow while
//! keeping the others controlled: suspended minima, buried chunks, and the
//! rank integrator's width buffers. Each check combines a semantic oracle, an
//! absolute ceiling, and a liveness floor. It then compares two sizes, so a
//! live counter must show the operation's claimed per-input scaling.

use super::min_ticks_from_big;
use before::testing::meter::registry::Shape;
use num_bigint::BigUint;

/// Slack numerator over the small-scale cost (denominator
/// [`SLACK_DEN`]): the ×1.25 flatness convention.
const SLACK_NUM: u64 = 5;

/// Slack denominator for the flatness bound.
const SLACK_DEN: u64 = 4;

/// Assert one per-unit cost stays flat (×1.25) across a doubling.
fn assert_flat(name: &str, unit: &str, small: (u64, u64), large: (u64, u64)) {
    let (m1, n1) = small;
    let (m2, n2) = large;
    eprintln!(
        "MEASURED {name}: small={m1}/{n1} large={m2}/{n2} \
         milli_per_{unit}={} -> {}",
        m1 * 1000 / n1,
        m2 * 1000 / n2,
    );
    assert!(
        u128::from(m2) * u128::from(n1) * u128::from(SLACK_DEN)
            <= u128::from(m1) * u128::from(n2) * u128::from(SLACK_NUM),
        "{name}: per-{unit} cost grew more than x1.25 across the \
         size doubling: {m1}/{n1} -> {m2}/{n2}"
    );
}

/// Heap and allocation ceilings for minimum-tick constant factors.
#[path = "version_scaling/minimum_resources.rs"]
mod minimum_resources;
/// Allocation bands for the rank integrator's width buffers.
#[path = "version_scaling/rank_allocation.rs"]
mod rank_allocation;
