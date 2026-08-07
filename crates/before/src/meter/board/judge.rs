//! The judgment: score a cell's measured samples against the exponent trend
//! bound, the ceilings, the declared models, and the committed liveness
//! floors, per currency.

use super::ceilings::{
    fold_exponent_ceiling, CAPACITY_MODEL_CEILING, CAPACITY_MODEL_FLOOR,
    FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL, HEAP_FLAT_ALLOWANCE_BYTES, MAX_GROWN_STACK_SEGMENTS,
    MAX_HEAP_BYTES_PER_INPUT_BYTE, MAX_LIMB_OPS_PER_INPUT_BYTE, MAX_SCALING_EXPONENT,
    MAX_SCAN_BITS_PER_INPUT_BYTE, MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT, MAX_TOUCHES_PER_INPUT_BYTE,
    MIN_EXPONENT_DENOM_GROWTH,
};
use super::currency::{ByCurrency, Currency, Liveness};
use super::measure::Sample;

/// The fitted scaling exponent over every point a run measured for a cell:
/// the log-log least-squares slope of counter reading against denominator
/// bytes.
///
/// This is the board's one exponent estimator — **an exponent is a trend
/// over all measured points, never a per-window ratio** (owner-ratified
/// measurement policy). Through two points the slope is exactly their log
/// ratio; the acceptance judgment fits one trend across the cell's whole
/// measurement ladder — four points spanning both sampling scales — so a
/// single generator lump at one point cannot define the estimate, while a
/// genuine super-linearity bends every point and still reads red. Densifying
/// the ladder (measuring more points) is not part of this policy: it remains
/// a case-by-case adjudication tool for a future disputed cell,
/// owner-invoked.
///
/// Readings are clamped through `max(m, 1)` so a zero at some points keeps
/// the fit defined; all-zero readings and degenerate spans (no denominator
/// variance) score 0.
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
/// The segments column's floor-trip message (unreachable while segments is
/// ceiling-only by policy; the judgment loop still carries it so a future
/// segments floor binds without a code change).
pub(super) const SEG_FLOOR_TRIP: &str =
    "segments floor: counter reads below floor: the meter is not watching this work";
/// The limb column's floor-trip message.
pub(super) const LIMB_FLOOR_TRIP: &str =
    "limb floor: counter reads below floor: the meter is not watching this work";
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

/// Fit one currency's exponent trend over a run's measured samples, in
/// measurement order, applying the judgment guards:
///
/// - the denominator span must scale ([`MIN_EXPONENT_DENOM_GROWTH`] from the
///   first used point to the last), or the fit divides by a vanishing log;
/// - a capacity-model cell's heap trend is honestly unjudgeable — the
///   doubling chain quantizes the peak by powers of two, so points straddling
///   a `k` step manufacture an exponent and points inside a step read
///   sublinear; the band judgment binds instead;
/// - every other cell's heap trend is fitted only over the points that clear
///   the flat allowance the constant leg already forgives (a point inside the
///   forgiven flat zone deflates the fit and manufactures an exponent at the
///   allowance boundary), and judged only when at least two such points
///   remain and they span a scaling denominator.
fn fit_currency(c: Currency, samples: &[&Sample]) -> Fit {
    let points: Option<Vec<(usize, u64)>> = samples
        .iter()
        .map(|s| s.readings.get(c).map(|m| (s.exp_denom_bytes, m)))
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
    if c == Currency::Heap {
        if samples.iter().any(|s| s.heap_model.is_some()) {
            return Fit {
                exp: Some(trend(&points)),
                judged: false,
            };
        }
        let cleared: Vec<(usize, u64)> = points
            .iter()
            .copied()
            .filter(|&(_, m)| m > HEAP_FLAT_ALLOWANCE_BYTES as u64)
            .collect();
        return if cleared.len() >= 2 && spans(&cleared) {
            Fit {
                exp: Some(trend(&cleared)),
                judged: true,
            }
        } else {
            Fit {
                exp: Some(trend(&points)),
                judged: false,
            }
        };
    }
    Fit {
        exp: Some(trend(&points)),
        judged: spans(&points),
    }
}

/// Every currency's fitted trend over a run's measured samples.
fn fit_exponents(samples: &[&Sample]) -> ByCurrency<Fit> {
    ByCurrency {
        heap: fit_currency(Currency::Heap, samples),
        segments: fit_currency(Currency::Segments, samples),
        limb: fit_currency(Currency::Limb, samples),
        scan: fit_currency(Currency::Scan, samples),
        touch: fit_currency(Currency::Touch, samples),
    }
}

/// The per-currency exponent ceilings over the fitted span.
///
/// Resolved from the declared models (the `ceilings` module's
/// declared-models section): the fold rows' predicted-marginal ceiling on
/// the fold currencies, a family-stated limb exponent where one is
/// declared, the global bound everywhere else.
fn exp_ceilings(first: &Sample, last: &Sample) -> ByCurrency<f64> {
    let spans =
        last.exp_denom_bytes as f64 >= first.exp_denom_bytes as f64 * MIN_EXPONENT_DENOM_GROWTH;
    let fold = match (first.fold_arity, last.fold_arity) {
        (Some(k1), Some(k2)) if spans => Some(fold_exponent_ceiling(
            k1,
            k2,
            first.exp_denom_bytes,
            last.exp_denom_bytes,
        )),
        _ => None,
    };
    ByCurrency {
        heap: MAX_SCALING_EXPONENT,
        segments: MAX_SCALING_EXPONENT,
        limb: last
            .declared_limb
            .map(|(exponent, _)| exponent)
            .or(fold)
            .unwrap_or(MAX_SCALING_EXPONENT),
        scan: fold.unwrap_or(MAX_SCALING_EXPONENT),
        touch: fold.unwrap_or(MAX_SCALING_EXPONENT),
    }
}

/// One judged column's derived scores: the fitted exponent trend and the
/// larger scale's per-unit constant (`None` where the counter is off).
#[derive(Clone, Copy)]
pub(super) struct Score {
    pub(super) exp: Option<f64>,
    /// Whether the exponent leg is judged ([`fit_currency`]'s guards).
    pub(super) exp_judged: bool,
    pub(super) per_unit: Option<f64>,
}

/// One evaluated cell: both samples of its window, per-currency scores, and
/// the verdict.
pub(super) struct CellResult {
    pub(super) op: &'static str,
    pub(super) family: &'static str,
    pub(super) s1: Sample,
    pub(super) s2: Sample,
    pub(super) scores: ByCurrency<Score>,
    /// The meters over their bounds; empty means green.
    pub(super) red: Vec<&'static str>,
}

/// Score one window (a cell's two samples at one scale) against the
/// ceilings, the declared models, and the liveness floors.
///
/// The exponent legs are judged at the supplied fits — the trend over every
/// point the run measured, which for a single-scale run is exactly this
/// window and for the acceptance judgment spans the whole ladder.
///
/// Every exponent — the limb column's included — is judged against the
/// denominator bytes (packed input, or `n_io` on the I/O-denominated cells),
/// never against `R`: `R` is the schoolbook cost law, so a limb exponent
/// against it reads a flat ~1 on exactly the quadratic converters the bound
/// exists to catch. Constants are judged per denominator byte, except segments
/// (an absolute count: the target is walks that never grow the stack) and the
/// text rows' limb constant, which is per `R` unit under the κ ceiling. The
/// loops run over the currency axis itself ([`ByCurrency::each`]), so a
/// currency added to the axis is judged on every cell or the destructuring
/// fails to compile.
fn judge_window(
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
    fits: ByCurrency<Fit>,
    ceilings: ByCurrency<f64>,
) -> CellResult {
    let capacity_model = s1.heap_model.is_some() && s2.heap_model.is_some();
    let score = |c: Currency| -> Score {
        let fit = *fits.get(c);
        let (Some(_), Some(m2)) = (*s1.readings.get(c), *s2.readings.get(c)) else {
            return Score {
                exp: None,
                exp_judged: false,
                per_unit: None,
            };
        };
        let per_unit = match c {
            Currency::Heap => {
                m2.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64) as f64 / s2.denom_bytes as f64
            }
            Currency::Segments => m2 as f64,
            Currency::Limb => m2 as f64 / s2.limb_denom as f64,
            Currency::Scan | Currency::Touch => m2 as f64 / s2.denom_bytes as f64,
        };
        Score {
            exp: fit.exp,
            exp_judged: fit.judged,
            per_unit: Some(per_unit),
        }
    };
    let scores = ByCurrency {
        heap: score(Currency::Heap),
        segments: score(Currency::Segments),
        limb: score(Currency::Limb),
        scan: score(Currency::Scan),
        touch: score(Currency::Touch),
    };

    let mut red = Vec::new();
    for (c, s) in scores.each() {
        let (mut ceiling, exp_label, const_label) = match c {
            Currency::Heap => (
                MAX_HEAP_BYTES_PER_INPUT_BYTE,
                "heap exponent",
                "heap constant",
            ),
            Currency::Segments => (
                MAX_GROWN_STACK_SEGMENTS as f64,
                "segments exponent",
                "segments count",
            ),
            Currency::Limb => (
                if s2.text_row {
                    MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT
                } else {
                    MAX_LIMB_OPS_PER_INPUT_BYTE
                },
                "limb exponent",
                "limb constant",
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
        // The capacity-model heap leg: both samples' readings must sit inside
        // the declared band around the model — over the ceiling is the
        // regression the model prices, under the floor is a stale model that
        // must be re-declared against the improved kernel.
        if c == Currency::Heap && capacity_model {
            let banded = |sample: &Sample, edge: f64| -> Option<bool> {
                let reading = (*sample.readings.get(c))? as f64;
                let model = sample.heap_model?;
                Some(if edge > 1.0 {
                    reading > model * edge
                } else {
                    reading < model * edge
                })
            };
            if [&s1, &s2]
                .iter()
                .any(|s| banded(s, CAPACITY_MODEL_CEILING).is_some_and(|over| over))
            {
                red.push("heap capacity-model ceiling");
            }
            if [&s1, &s2]
                .iter()
                .any(|s| banded(s, CAPACITY_MODEL_FLOOR).is_some_and(|under| under))
            {
                red.push("heap capacity-model floor (stale model)");
            }
            continue;
        }
        // A family-stated flat heap ceiling replaces the global heap constant
        // on the cells that declare one; the exponent leg is untouched.
        if c == Currency::Heap {
            if let Some(declared) = s2.declared_heap {
                ceiling = declared;
            }
        }
        // A family-stated limb model replaces both limb legs on the cells that
        // declare one (the ceilings module's declared-models section): the
        // stated constant in place of the global (or text) ceiling, the stated
        // exponent in place of the global bound (resolved in the ceilings
        // argument).
        if c == Currency::Limb {
            if let Some((_, per_radix_unit)) = s2.declared_limb {
                ceiling = per_radix_unit;
            }
        }
        // The fold rows' declared scan-constant model at this window's arity.
        if c == Currency::Scan {
            if let Some(k2) = s2.fold_arity {
                ceiling = FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL * (2.0 * k2 as f64).log2()
                    + s2.fold_search_bits as f64 / s2.denom_bytes as f64;
            }
        }
        if s.exp_judged && s.exp.is_some_and(|e| e > *ceilings.get(c)) {
            red.push(exp_label);
        }
        if s.per_unit.is_some_and(|v| v > ceiling) {
            red.push(const_label);
        }
    }
    // The liveness floors bind in this same pass, at both sizes: a counter
    // reading below the least a watching meter could honestly read means the
    // meter is not watching the work the ceilings claim to bound.
    for (c, _) in scores.each() {
        let trip = match c {
            Currency::Heap => HEAP_FLOOR_TRIP,
            Currency::Segments => SEG_FLOOR_TRIP,
            Currency::Limb => LIMB_FLOOR_TRIP,
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

/// Score one single-scale window: the exponent legs are the trend over the
/// window's own two points (all this run measured), the ceilings, models, and
/// floors exactly [`judge_window`]'s.
pub(super) fn evaluate(
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
) -> CellResult {
    let fits = fit_exponents(&[&s1, &s2]);
    let ceilings = exp_ceilings(&s1, &s2);
    judge_window(op, family, s1, s2, fits, ceilings)
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
    let ceilings = exp_ceilings(&l1, &h2);
    (
        judge_window(op, family, l1, l2, fits, ceilings),
        judge_window(op, family, h1, h2, fits, ceilings),
    )
}
