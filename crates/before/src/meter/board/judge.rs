//! The judgment: score a cell's measured samples against the exponent trend
//! bound, the ceilings, the declared models, and the committed liveness
//! floors, per currency.

use super::ceilings::{
    HEAP_INTERCEPT_BYTES, HEAP_TREND_RESOLUTION_BYTES, MAX_HEAP_BYTES_PER_INPUT_BYTE,
    MAX_SCALING_EXPONENT, MAX_SCAN_BITS_PER_INPUT_BYTE, MAX_TOUCHES_PER_INPUT_BYTE,
    MIN_EXPONENT_DENOM_GROWTH,
};
use super::currency::{ByCurrency, Currency, Liveness};
use super::measure::Sample;

/// Log-log least-squares slope of readings against model units.
///
/// Acceptance uses all four ladder points in one fit. Zero readings become one
/// to keep logarithms defined; all-zero readings and constant denominators
/// yield zero. A finite fit can reveal scaling regressions but cannot establish
/// an asymptotic bound or the liveness of a quiet counter.
pub(super) fn trend(points: &[(usize, u64)]) -> f64 {
    if points.iter().all(|&(_, m)| m == 0) {
        return 0.0;
    }
    let xy: Vec<(f64, f64)> = points
        .iter()
        .map(|&(n, m)| ((n as f64).ln(), (m.max(1) as f64).ln()))
        .collect();
    let count = xy.len() as f64;
    let mean_x = xy.iter().map(|(x, _)| x).sum::<f64>() / count;
    let mean_y = xy.iter().map(|(_, y)| y).sum::<f64>() / count;
    let sxx: f64 = xy.iter().map(|(x, _)| (x - mean_x) * (x - mean_x)).sum();
    if sxx <= f64::EPSILON {
        return 0.0;
    }
    let sxy: f64 = xy.iter().map(|(x, y)| (x - mean_x) * (y - mean_y)).sum();
    sxy / sxx
}

/// A liveness-floor trip's rendered message: the column and the vacuity
/// mechanism.
pub(super) const HEAP_FLOOR_TRIP: &str =
    "heap floor: counter reads below floor: the meter is not watching this work";
/// The scan column's floor-trip message.
pub(super) const SCAN_FLOOR_TRIP: &str =
    "scan floor: counter reads below floor: the meter is not watching this work";
/// The touch column's floor-trip message.
pub(super) const TOUCH_FLOOR_TRIP: &str =
    "touch floor: counter reads below floor: the meter is not watching this work";

/// Whether `count` sits below a committed floor (an NA declaration never
/// trips).
fn below_floor(liveness: Liveness, count: u64) -> bool {
    match liveness {
        Liveness::Floor { min, .. } => count < min,
        Liveness::NotApplicable { .. } => false,
    }
}

/// One currency's fitted exponent trend and whether the exponent leg is
/// judged.
#[derive(Clone, Copy)]
struct Fit {
    /// The fitted slope over the points the judgment uses, `None` where the
    /// counter is not compiled in.
    exp: Option<f64>,
    /// Whether the exponent leg is judged (the guards below).
    judged: bool,
}

/// Fit a scaling trend where the denominator spans enough sizes to distinguish
/// growth. Heap uses its declared resolution; other readings are unchanged.
fn fit_currency(c: Currency, samples: &[&Sample]) -> Fit {
    let points: Option<Vec<(usize, u64)>> = samples
        .iter()
        .map(|s| {
            s.readings.get(c).map(|m| {
                let units = s
                    .models
                    .get(c)
                    .map_or(s.exp_denom_bytes, |model| model.trend_units);
                let reading = if c == Currency::Heap {
                    m.max(HEAP_TREND_RESOLUTION_BYTES as u64)
                } else {
                    m
                };
                (units, reading)
            })
        })
        .collect();
    let Some(points) = points else {
        return Fit {
            exp: None,
            judged: false,
        };
    };
    let spans = |points: &[(usize, u64)]| -> bool {
        let first = points.first().map_or(0, |&(n, _)| n);
        let last = points.last().map_or(0, |&(n, _)| n);
        last as f64 >= first as f64 * MIN_EXPONENT_DENOM_GROWTH
    };
    Fit {
        exp: Some(trend(&points)),
        judged: spans(&points),
    }
}

/// Verify that one cell uses one coherent model across its measurement ladder.
///
/// Units may grow with the operands. Applicability and proportional ceilings
/// may not: changing either between samples would splice two different claims
/// into one trend.
fn validate_models(samples: &[&Sample]) {
    for (currency, first) in samples[0].models.each() {
        if let Some(first) = first {
            assert!(
                first.trend_units > 0 && first.constant_units > 0,
                "resource-model units are positive"
            );
            assert!(
                first
                    .ceiling
                    .is_none_or(|ceiling| ceiling.is_finite() && ceiling > 0.0),
                "a resource-model ceiling is positive"
            );
        }
        for sample in &samples[1..] {
            let next = sample.models.get(currency);
            assert_eq!(
                first.is_some(),
                next.is_some(),
                "a resource model must apply at every sample size"
            );
            if let (Some(first), Some(next)) = (first, next) {
                assert!(
                    next.trend_units > 0 && next.constant_units > 0,
                    "resource-model units are positive"
                );
                assert!(
                    next.ceiling
                        .is_none_or(|ceiling| ceiling.is_finite() && ceiling > 0.0),
                    "a resource-model ceiling is positive"
                );
                assert_eq!(
                    first.ceiling.map(f64::to_bits),
                    next.ceiling.map(f64::to_bits),
                    "a resource model's ceiling is independent of sample size"
                );
            }
        }
    }
}

/// Every currency's fitted trend over a run's measured samples.
fn fit_exponents(samples: &[&Sample]) -> ByCurrency<Fit> {
    validate_models(samples);
    ByCurrency {
        heap: fit_currency(Currency::Heap, samples),
        scan: fit_currency(Currency::Scan, samples),
        touch: fit_currency(Currency::Touch, samples),
    }
}

/// A column's growth fit and largest raw per-unit cost across its two samples.
#[derive(Clone, Copy)]
pub(super) struct Score {
    /// The fitted growth exponent, or `None` when the meter is absent.
    pub(super) exp: Option<f64>,
    /// Whether the exponent leg is judged ([`fit_currency`]'s guards).
    pub(super) exp_judged: bool,
    /// The larger raw cost per constant unit across the two samples.
    pub(super) per_unit: Option<f64>,
}

/// One evaluated cell: both samples of its window, per-currency scores, and
/// the verdict.
pub(super) struct CellResult {
    /// The operation row.
    pub(super) op: &'static str,
    /// The input-family column.
    pub(super) family: &'static str,
    /// The smaller measured sample.
    pub(super) s1: Sample,
    /// The larger measured sample.
    pub(super) s2: Sample,
    /// Each meter's derived judgment values.
    pub(super) scores: ByCurrency<Score>,
    /// The meters over their bounds; empty means green.
    pub(super) red: Vec<&'static str>,
}

/// Score one window (a cell's two samples — two sizes — at one sampling
/// scale) against the ceilings, the declared models, and the liveness
/// floors.
///
/// The exponent legs are judged at the supplied fits — the trend over every
/// point the run measured, which for a single-scale run is exactly this
/// window and for the acceptance judgment spans the whole ladder.
///
/// By default, exponents and constants use the cell's byte denominators.
/// A resource model replaces either unit axis
/// for one currency. The loops run over the currency axis itself
/// ([`ByCurrency::each`]), so adding a currency fails to compile until every
/// cell judges it.
fn judge_window(
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
    fits: ByCurrency<Fit>,
) -> CellResult {
    let score = |c: Currency| -> Score {
        let fit = *fits.get(c);
        let (Some(m1), Some(m2)) = (*s1.readings.get(c), *s2.readings.get(c)) else {
            return Score {
                exp: None,
                exp_judged: false,
                per_unit: None,
            };
        };
        let per_unit = [(m1, &s1), (m2, &s2)]
            .map(|(reading, sample)| reading as f64 / constant_units(sample, c) as f64)
            .into_iter()
            .fold(0.0, f64::max);
        Score {
            exp: fit.exp,
            exp_judged: fit.judged,
            per_unit: Some(per_unit),
        }
    };
    let scores = ByCurrency {
        heap: score(Currency::Heap),
        scan: score(Currency::Scan),
        touch: score(Currency::Touch),
    };

    let mut red = Vec::new();
    for (c, s) in scores.each() {
        let (global_ceiling, exp_label, const_label) = match c {
            Currency::Heap => (
                MAX_HEAP_BYTES_PER_INPUT_BYTE,
                "heap exponent",
                "heap constant",
            ),
            Currency::Scan => (
                MAX_SCAN_BITS_PER_INPUT_BYTE,
                "scan exponent",
                "scan constant",
            ),
            Currency::Touch => (
                MAX_TOUCHES_PER_INPUT_BYTE,
                "touch exponent",
                "touch constant",
            ),
        };
        let ceiling = s2
            .models
            .get(c)
            .and_then(|model| model.ceiling)
            .unwrap_or(global_ceiling);
        if s.exp_judged && s.exp.is_some_and(|e| e > MAX_SCALING_EXPONENT) {
            red.push(exp_label);
        }
        let intercept = if c == Currency::Heap {
            HEAP_INTERCEPT_BYTES as f64
        } else {
            0.0
        };
        if [&s1, &s2].iter().any(|sample| {
            sample.readings.get(c).is_some_and(|reading| {
                reading as f64 > intercept + ceiling * constant_units(sample, c) as f64
            })
        }) {
            red.push(const_label);
        }
    }
    // The liveness floors bind in this same pass, at both sizes: a counter
    // reading below the least a watching meter could honestly read means the
    // meter is not watching the work the ceilings claim to bound.
    for (c, _) in scores.each() {
        let trip = match c {
            Currency::Heap => HEAP_FLOOR_TRIP,
            Currency::Scan => SCAN_FLOOR_TRIP,
            Currency::Touch => TOUCH_FLOOR_TRIP,
        };
        if [&s1, &s2].iter().any(|s| {
            s.readings
                .get(c)
                .is_some_and(|r| below_floor(*s.floors.get(c), r))
        }) {
            red.push(trip);
        }
    }

    CellResult {
        op,
        family,
        s1,
        s2,
        scores,
        red,
    }
}

/// Resolve one sample's units for its proportional ceiling and displayed cost.
fn constant_units(sample: &Sample, currency: Currency) -> usize {
    sample
        .models
        .get(currency)
        .map_or(sample.denom_bytes, |model| model.constant_units)
}

/// Check small inputs against ceilings and liveness without fitting growth
/// across allocation minima and inline-storage thresholds.
pub(super) fn evaluate_small(
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
) -> CellResult {
    let mut fits = fit_exponents(&[&s1, &s2]);
    fits.heap.judged = false;
    fits.scan.judged = false;
    fits.touch.judged = false;
    judge_window(op, family, s1, s2, fits)
}

/// Score one window at a single sampling scale: the exponent legs are the
/// trend over the window's own two points (all this run measured), the
/// ceilings, models, and floors exactly [`judge_window`]'s.
pub(super) fn evaluate(
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
) -> CellResult {
    let fits = fit_exponents(&[&s1, &s2]);
    judge_window(op, family, s1, s2, fits)
}

/// Score one cell across its whole measurement ladder.
///
/// One exponent trend over all four measured points (the two sizes at each
/// sampling scale), with every constant, declared-model band, and liveness
/// floor still judged per window.
///
/// Returns the two windows' results in ladder order; an exponent-leg red
/// binds both windows (the trend is one judgment), so it renders on both
/// matrices.
pub(super) fn evaluate_acceptance(
    op: &'static str,
    family: &'static str,
    lo: (Sample, Sample),
    hi: (Sample, Sample),
) -> (CellResult, CellResult) {
    let (l1, l2) = lo;
    let (h1, h2) = hi;
    let fits = fit_exponents(&[&l1, &l2, &h1, &h2]);
    (
        judge_window(op, family, l1, l2, fits),
        judge_window(op, family, h1, h2, fits),
    )
}
