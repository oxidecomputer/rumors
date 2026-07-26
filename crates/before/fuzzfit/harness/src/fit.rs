//! Log-log regression over (denominator, fuel) samples: the calibration
//! leg's fitter.
//!
//! Each public operation gets one fitted line of `log₁₀ fuel` against
//! `log₁₀ denom_bits`, with two *residual widths* — the maximum positive
//! residual (the ceiling's distance from the line) and the maximum
//! negative residual magnitude (the floor's) — recorded alongside. The
//! widths are what make the line a *band*: enforcement asserts membership
//! in `line − (width_below + margin) ..= line + (width_above + margin)`,
//! so the fit's looseness is explicit and committed, never implicit in a
//! tolerance constant chosen after the fact.
//!
//! # The two widths do different jobs
//!
//! The ceiling carries the asymptotic claim (above it is a regression
//! flag); the floor carries liveness (below it is a dead meter or an
//! unmeasured path). Pricing them separately matters because the residual
//! cloud is one-sidedly heavy: at every size, fast-path steps (an early
//! comparison exit, a join whose operand shapes coalesce) undercut the
//! median law by decades, while honest work sits near it. A single
//! symmetric max-|residual| width would let that cheap mass inflate the
//! *ceiling* — pricing the regression flag off the fast paths — until a
//! superlinear mechanism's whole in-range excess fits inside it.
//!
//! # Medians in, spikes out
//!
//! Per-step fuel is heteroscedastic: at every size, a cheap-mass of
//! fast-path steps coexists with amortization spikes (a tick that pays a
//! deferred normalization), and ordinary least squares over the raw cloud
//! lets that mass drag the slope far from the median law (readings up to
//! +0.4 slope on flat-median kernels). The slope is therefore fitted over
//! *half-decade bucket medians* — the stable location statistic per size —
//! while the widths are still taken over *all* samples against that line,
//! so bounded amortization spikes land inside the committed band and only
//! unbounded (asymptotic) departures escape it.
//!
//! Operations whose sampled denominators span less than a decade, or fewer
//! than three buckets, cannot support a slope estimate; they classify as
//! *constant* bands — slope 0, centered on the mean log-fuel — which is
//! also the honest reading for genuinely O(1) rows.

use std::collections::BTreeMap;

use crate::bands::Band;

/// One fitted band over a kernel's samples.
#[derive(Debug, Clone, Copy)]
pub struct Fit {
    /// Fitted log-log slope (0 for constant-classified bands).
    pub slope: f64,
    /// Fitted intercept: `log₁₀ fuel` at `denom_bits = 1`.
    pub intercept: f64,
    /// Maximum positive residual of the whole corpus against the line:
    /// the ceiling's distance (the regression flag's threshold rides on
    /// this side).
    pub width_above: f64,
    /// Maximum negative residual magnitude: the floor's distance (the
    /// liveness flag's threshold rides on this side).
    pub width_below: f64,
    /// Sample count behind the fit.
    pub samples: usize,
    /// Smallest denominator seen (bits).
    pub min_denom: u64,
    /// Largest denominator seen (bits).
    pub max_denom: u64,
    /// Whether the band was constant-classified (denominator span under a
    /// decade or fewer than three buckets).
    pub constant: bool,
}

/// The fit floor: samples below this denominator are excluded from both
/// the fit and the committed judgment range (`Fit::min_denom` never sits
/// below it when floored samples exist).
///
/// Below ~16 bytes of operands, per-call constant overhead (register
/// dispatch, allocator fixed costs) dominates fuel, and a log-log line
/// through that regime mixes overhead decay into the asymptotic slope —
/// the bucket medians rise from the overhead knee and then settle onto the
/// true law only above it. The floor is also the enforcement-side size
/// floor: below it the shrinker would converge on small-n noise, not
/// genuine violations.
const FIT_FLOOR_BITS: u64 = 128;

/// Minimum denominator decades a slope estimate needs; narrower clouds
/// classify constant.
const MIN_DECADES: f64 = 1.0;

/// Buckets per decade of denominator (bucket medians are the fit's inputs).
const BUCKETS_PER_DECADE: f64 = 2.0;

/// Minimum populated buckets a slope estimate needs.
const MIN_BUCKETS: usize = 3;

/// Fit one kernel's samples. `None` when fewer than 2 samples exist.
///
/// Samples below [`FIT_FLOOR_BITS`] are dropped first (when anything
/// remains above the floor); a kernel sampled only below the floor fits
/// over what it has and classifies constant.
pub fn fit(samples: &[(u64, u64)]) -> Option<Fit> {
    let floored: Vec<(u64, u64)> = samples
        .iter()
        .copied()
        .filter(|&(d, _)| d >= FIT_FLOOR_BITS)
        .collect();
    let samples: &[(u64, u64)] = if floored.len() >= 2 {
        &floored
    } else {
        samples
    };
    if samples.len() < 2 {
        return None;
    }
    let min_denom = samples.iter().map(|&(d, _)| d).min().expect("non-empty");
    let max_denom = samples.iter().map(|&(d, _)| d).max().expect("non-empty");
    let logs: Vec<(f64, f64)> = samples
        .iter()
        .map(|&(d, f)| ((d as f64).log10(), (f.max(1) as f64).log10()))
        .collect();
    let decades = (max_denom as f64 / min_denom as f64).log10();

    // Half-decade buckets; each contributes its median (x, y) as one point.
    let mut buckets: BTreeMap<i64, Vec<(f64, f64)>> = BTreeMap::new();
    for &(x, y) in &logs {
        buckets
            .entry((x * BUCKETS_PER_DECADE).floor() as i64)
            .or_default()
            .push((x, y));
    }
    let medians: Vec<(f64, f64)> = buckets
        .values_mut()
        .map(|pts| {
            pts.sort_by(|a, b| a.1.total_cmp(&b.1));
            let my = pts[pts.len() / 2].1;
            pts.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mx = pts[pts.len() / 2].0;
            (mx, my)
        })
        .collect();

    let constant = decades < MIN_DECADES || medians.len() < MIN_BUCKETS;
    let (slope, intercept) = if constant {
        let mean_y = logs.iter().map(|&(_, y)| y).sum::<f64>() / logs.len() as f64;
        (0.0, mean_y)
    } else {
        let n = medians.len() as f64;
        let mx = medians.iter().map(|&(x, _)| x).sum::<f64>() / n;
        let my = medians.iter().map(|&(_, y)| y).sum::<f64>() / n;
        let sxx: f64 = medians.iter().map(|&(x, _)| (x - mx) * (x - mx)).sum();
        let sxy: f64 = medians
            .iter()
            .map(|&(x, y)| (x - mx) * (y - my))
            .sum::<f64>();
        let slope = sxy / sxx;
        (slope, my - slope * mx)
    };
    let (width_above, width_below) = logs.iter().fold((0.0f64, 0.0f64), |(wa, wb), &(x, y)| {
        let residual = y - (intercept + slope * x);
        (wa.max(residual), wb.max(-residual))
    });
    Some(Fit {
        slope,
        intercept,
        width_above,
        width_below,
        samples: samples.len(),
        min_denom,
        max_denom,
        constant,
    })
}

/// The largest disagreement, in `log₁₀ fuel`, between a fresh fit's line
/// and a pinned band's line over the fresh fit's own denominator range.
///
/// Both lines are affine in `log₁₀ d`, so the maximum over the interval
/// is attained at an endpoint. This is the staleness cross-check's
/// distance: the pin is computable two ways — the committed constants and
/// a fresh fit of the same deterministic sample stream — and a divergence
/// beyond tolerance means the pin no longer describes the code that is
/// running (a toolchain shift, a kernel change, a generator-population
/// change) and demands a deliberate re-pin, never a silent drift.
pub fn line_divergence(f: &Fit, band: &Band) -> f64 {
    [f.min_denom, f.max_denom]
        .into_iter()
        .map(|d| {
            let x = (d as f64).log10();
            ((f.intercept + f.slope * x) - (band.intercept + band.slope * x)).abs()
        })
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests;
