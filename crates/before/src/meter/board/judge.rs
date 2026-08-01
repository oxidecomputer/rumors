//! The judgment: score a cell's two samples against the exponent bounds,
//! the ceilings, the declared models, and the committed liveness floors,
//! per currency.

use super::ceilings::{
    fold_exponent_ceiling, CAPACITY_MODEL_CEILING, CAPACITY_MODEL_FLOOR,
    FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL, HEAP_FLAT_ALLOWANCE_BYTES, MAX_GROWN_STACK_SEGMENTS,
    MAX_HEAP_BYTES_PER_INPUT_BYTE, MAX_LIMB_OPS_PER_INPUT_BYTE, MAX_SCALING_EXPONENT,
    MAX_SCAN_BITS_PER_INPUT_BYTE, MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT, MAX_TOUCHES_PER_INPUT_BYTE,
    MIN_EXPONENT_DENOM_GROWTH,
};
use super::currency::{ByCurrency, Currency, Liveness};
use super::measure::Sample;

/// The scaling exponent `log(m2/m1) / log(n2/n1)`, clamped finite.
///
/// A meter that reads zero at both scales scores 0; a zero at one scale is
/// clamped through `max(m, 1)` so the ratio stays defined. Degenerate input
/// sizes (`n2 <= n1`, possible only at extreme scale-down) score 0 rather
/// than dividing by a vanishing log.
pub(super) fn exponent(m1: u64, m2: u64, n1: usize, n2: usize) -> f64 {
    if (m1 == 0 && m2 == 0) || n2 <= n1 {
        return 0.0;
    }
    let growth = (m2.max(1) as f64) / (m1.max(1) as f64);
    growth.ln() / ((n2 as f64) / (n1 as f64)).ln()
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

/// Panic unless two in-process measurements of one cell agree on every
/// counter reading and denominator.
///
/// The in-process leg of the board's determinism tripwire ([`run`](super::render::run)'s
/// self-verification); the cross-process leg is the `amp-board-determinism`
/// recipe, which byte-compares two whole renders.
pub(super) fn assert_deterministic(op: &str, family: &str, a: &Sample, b: &Sample) {
    assert_eq!(
        (a.denom_bytes, a.exp_denom_bytes, a.limb_denom),
        (b.denom_bytes, b.exp_denom_bytes, b.limb_denom),
        "determinism: {op} x {family}: two in-process measurements disagree on denominators"
    );
    for ((currency, first), (_, second)) in a.readings.each().into_iter().zip(b.readings.each()) {
        assert_eq!(
            first,
            second,
            "determinism: {op} x {family}: two in-process measurements disagree on the {} \
             counter",
            currency.label()
        );
    }
}

/// One judged column's derived scores: the fitted exponent and the larger
/// scale's per-unit constant (`None` where the counter is off).
#[derive(Clone, Copy)]
pub(super) struct Score {
    pub(super) exp: Option<f64>,
    /// Whether the exponent leg is judged.
    ///
    /// False where the denominator pair does not scale
    /// ([`MIN_EXPONENT_DENOM_GROWTH`]) or, on the heap column, where
    /// either reading sits inside the flat allowance the constant leg
    /// already forgives (a sub-allowance exponent is allocator size-class
    /// noise, and a fit from a sub-allowance base manufactures an
    /// exponent at the boundary).
    pub(super) exp_judged: bool,
    pub(super) per_unit: Option<f64>,
}

/// One evaluated cell: both samples, per-currency scores, and the verdict.
pub(super) struct CellResult {
    pub(super) op: &'static str,
    pub(super) family: &'static str,
    pub(super) s1: Sample,
    pub(super) s2: Sample,
    pub(super) scores: ByCurrency<Score>,
    /// The meters over their bounds; empty means green.
    pub(super) red: Vec<&'static str>,
}

/// Score a cell's two samples against the exponent bound, the ceilings,
/// and the liveness floors, per currency.
///
/// Every exponent — the limb column's included — is judged against the
/// denominator bytes (packed input, or `n_io` on the I/O-denominated
/// cells), never against `R`: `R` is the schoolbook cost law, so a limb
/// exponent against it reads a flat ~1 on exactly the quadratic converters
/// the bound exists to catch. Constants are judged per denominator byte,
/// except segments (an absolute count: the target is walks that never grow
/// the stack) and the text rows' limb constant, which is per `R` unit
/// under the κ ceiling. The loops run over the currency axis itself
/// ([`ByCurrency::each`]), so a currency added to the axis is judged on
/// every cell or the destructuring fails to compile.
pub(super) fn evaluate(
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
) -> CellResult {
    let denom_scales =
        s2.exp_denom_bytes as f64 >= s1.exp_denom_bytes as f64 * MIN_EXPONENT_DENOM_GROWTH;
    // The declared models, resolved for this cell (the declared-models
    // section): the fold exponent ceiling needs both samples' arities,
    // and the capacity-chain judgment both samples' predictions.
    let fold_exp_ceiling = match (s1.fold_arity, s2.fold_arity) {
        (Some(k1), Some(k2)) if denom_scales => Some(fold_exponent_ceiling(
            k1,
            k2,
            s1.exp_denom_bytes,
            s2.exp_denom_bytes,
        )),
        _ => None,
    };
    let capacity_model = s1.heap_model.is_some() && s2.heap_model.is_some();
    let score = |c: Currency| -> Score {
        let (Some(m1), Some(m2)) = (*s1.readings.get(c), *s2.readings.get(c)) else {
            return Score {
                exp: None,
                exp_judged: false,
                per_unit: None,
            };
        };
        let exp = exponent(m1, m2, s1.exp_denom_bytes, s2.exp_denom_bytes);
        // A capacity-model cell's heap exponent is honestly unjudgeable:
        // the doubling chain quantizes the peak by powers of two, so a
        // probe pair straddling a k step manufactures an exponent and one
        // inside a step reads sublinear; the model judgment below is what
        // binds instead. Every other cell's heap exponent is fitted only
        // where BOTH probes clear the flat allowance the constant leg
        // already forgives: a base inside the forgiven flat zone deflates
        // the fit and manufactures an exponent at the allowance boundary
        // (the flat term masks the scaling part at the small probe and
        // releases it at the large one), so a straddling pair stays
        // unjudged and the class is judged at the next doubling, where
        // both probes sit in the scaling regime.
        let exp_judged = denom_scales
            && (c != Currency::Heap
                || (!capacity_model && m1.min(m2) > HEAP_FLAT_ALLOWANCE_BYTES as u64));
        let per_unit = match c {
            Currency::Heap => {
                m2.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64) as f64 / s2.denom_bytes as f64
            }
            Currency::Segments => m2 as f64,
            Currency::Limb => m2 as f64 / s2.limb_denom as f64,
            Currency::Scan | Currency::Touch => m2 as f64 / s2.denom_bytes as f64,
        };
        Score {
            exp: Some(exp),
            exp_judged,
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
        // The capacity-model heap leg: both samples' readings must sit
        // inside the declared band around the model — over the ceiling is
        // the regression the model prices, under the floor is a stale
        // model that must be re-declared against the improved kernel.
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
        // A family-stated flat heap ceiling replaces the global heap
        // constant on the cells that declare one; the exponent leg is
        // untouched.
        if c == Currency::Heap {
            if let Some(declared) = s2.declared_heap {
                ceiling = declared;
            }
        }
        // A family-stated limb model replaces both limb legs on the
        // cells that declare one (the ceilings module's declared-models
        // section): the
        // stated constant in place of the global (or text) ceiling,
        // the stated exponent below in place of the global bound.
        if c == Currency::Limb {
            if let Some((_, per_radix_unit)) = s2.declared_limb {
                ceiling = per_radix_unit;
            }
        }
        // The fold rows' declared exponent ceiling (limb, scan, touch)
        // and scan-constant model.
        let mut exp_ceiling = match (c, fold_exp_ceiling) {
            (Currency::Limb | Currency::Scan | Currency::Touch, Some(ceiling)) => ceiling,
            _ => MAX_SCALING_EXPONENT,
        };
        if c == Currency::Limb {
            if let Some((exponent, _)) = s2.declared_limb {
                exp_ceiling = exponent;
            }
        }
        if c == Currency::Scan {
            if let Some(k2) = s2.fold_arity {
                ceiling = FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL * (2.0 * k2 as f64).log2()
                    + s2.fold_search_bits as f64 / s2.denom_bytes as f64;
            }
        }
        if s.exp_judged && s.exp.is_some_and(|e| e > exp_ceiling) {
            red.push(exp_label);
        }
        if s.per_unit.is_some_and(|v| v > ceiling) {
            red.push(const_label);
        }
    }
    // The liveness floors bind in this same pass, at both scales: a counter
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
