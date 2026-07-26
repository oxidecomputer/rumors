//! The shape leg: within-case curvature judgment over bucket medians.
//!
//! The band judgment is pointwise — one step above its kernel's ceiling
//! flags — and its blind spot is a mechanism that tilts *into* the band: a
//! superlinear cost whose whole in-range excess stays inside the ceiling
//! width on a kernel whose residual cloud is legitimately wide. This leg
//! judges the complementary quantity, the *trend*: across one program's
//! floored half-decade buckets, the local log-log slope between the
//! bottom and top bucket medians must not exceed the kernel's pinned
//! slope by more than a measured allowance. Two legs with different
//! failure modes beat one leg twice as wide.
//!
//! # Why within one case
//!
//! Pooled across the corpus, bucket medians tilt for an innocent reason:
//! different families dominate different size buckets at per-family cost
//! levels differing severalfold (the family-mixture artifact the pin's
//! ground-truthing dispositions). One program is one family at one
//! parameter draw — a family-pure population — so that composition tilt
//! cannot occur here, and a rising within-case trend is the mechanism's
//! own curvature.
//!
//! # Which rows this leg can see
//!
//! Evidence requires [`MIN_BUCKETS`] populated floored buckets of
//! [`MIN_PER_BUCKET`] samples spanning [`MIN_DECADES`]: the high-mass
//! construction kernels (tick, fork, the join accumulations) meet it
//! routinely; kernels sampled once or twice per program (the folds, the
//! one-shot queries) never do, and stay the point leg's and the reach
//! family's business. The committed tripwires in `tests.rs` prove the leg
//! fires on quadratic readings and stays quiet on flat ones.

use std::collections::BTreeMap;

use crate::bands::Band;

/// Rows the shape leg abstains on: the fold kernels.
///
/// Their in-band law legitimately carries a documented, budget-bounded
/// log factor along the fold-*width* axis (`Version::join_all`'s balanced
/// reduction passes every input through O(log n) joins), so a
/// within-case width ladder trends above the pooled pinned slope for the
/// honest mechanism too — a flag here would read the factor, not a
/// regression. The point leg owns these rows instead: the generators'
/// width ladder puts a degenerate (left-fold) reduction's excess, which
/// grows as n / log n, far past the pinned ceiling inside the reachable
/// width range.
pub const SHAPE_EXEMPT: &[&str] = &["ff_version_join_all", "ff_version_meet_all"];

/// Minimum populated floored buckets a within-case trend needs.
pub const MIN_BUCKETS: usize = 3;

/// Minimum samples per bucket (a bucket median over fewer is a dart
/// throw, not a location statistic).
pub const MIN_PER_BUCKET: usize = 8;

/// Minimum denominator decades between the bottom and top bucket medians.
pub const MIN_DECADES: f64 = 1.0;

/// The measured allowance above the pinned slope for a within-case local
/// slope.
///
/// Measured 2026-07-26 over the calibration corpus of record (1536
/// programs, ~985k steps; `bin/calibrate` re-derives the evidence on
/// every re-pin): the maximum healthy within-case excess across every
/// evidence-bearing (band key, case) pair was +0.006 (`ff_clock_join`,
/// a `DenseSpine` draw). The allowance sits far above that observed
/// ceiling and a third of the +1.0 a quadratic mechanism adds over a
/// linear pin, so the gap it lives in is wide on both sides.
pub const SLOPE_ALLOWANCE: f64 = 0.3;

/// One kernel's within-case local slope, minus its pinned slope: the
/// quantity [`SLOPE_ALLOWANCE`] bounds.
///
/// `None` when the case lacks evidence (too few floored buckets, thin
/// buckets, or under a decade of span between the endpoint medians).
///
/// The local slope is taken between the bottom and top populated buckets'
/// median points (median log-denominator, median log-fuel per bucket) —
/// the same location statistic the pinned fit rests on, judged over the
/// case's own reach.
pub fn local_slope_excess(band: &Band, samples: &[(u64, u64)]) -> Option<f64> {
    let mut buckets: BTreeMap<i64, Vec<(f64, f64)>> = BTreeMap::new();
    for &(d, f) in samples {
        if d < band.min_denom {
            continue;
        }
        let x = (d as f64).log10();
        let y = (f.max(1) as f64).log10();
        buckets
            .entry((x * 2.0).floor() as i64)
            .or_default()
            .push((x, y));
    }
    buckets.retain(|_, pts| pts.len() >= MIN_PER_BUCKET);
    if buckets.len() < MIN_BUCKETS {
        return None;
    }
    let median = |pts: &mut Vec<(f64, f64)>| {
        pts.sort_by(|a, b| a.1.total_cmp(&b.1));
        let y = pts[pts.len() / 2].1;
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let x = pts[pts.len() / 2].0;
        (x, y)
    };
    let mut values: Vec<Vec<(f64, f64)>> = buckets.into_values().collect();
    let (bot_x, bot_y) = median(values.first_mut().expect("MIN_BUCKETS checked"));
    let (top_x, top_y) = median(values.last_mut().expect("MIN_BUCKETS checked"));
    if top_x - bot_x < MIN_DECADES {
        return None;
    }
    Some((top_y - bot_y) / (top_x - bot_x) - band.slope)
}

#[cfg(test)]
mod tests;
