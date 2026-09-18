//! The measurement engine: run one prepared cell under every meter and capture
//! its counter readings and settled denominators as a [`Sample`].

use crate::meter;

use super::cell::{Cell, Denom, ModelSpec, Units};
use super::currency::{ByCurrency, Currency, Floors};

/// The peak-heap meter the board reads, supplied by the binary that runs it.
///
/// A counting global allocator is per-binary state the library cannot own, so
/// the runner (the `amp_board` example, the smoke test) installs one and passes
/// readers in. All three read the runner's allocator: `reset_peak` clears the
/// peak high-water mark, `peak` reads it, `current` reads live bytes (the
/// baseline subtracted from the peak).
pub struct HeapMeter {
    /// Clear the peak high-water mark down to current usage.
    pub reset_peak: fn(),
    /// The peak live bytes since the last reset.
    pub peak: fn() -> usize,
    /// The currently live bytes.
    pub current: fn() -> usize,
}

/// A resource model resolved to this sample's concrete units.
#[derive(Clone, Copy)]
pub(super) struct Model {
    /// Units against which the growth trend is fitted.
    pub(super) trend_units: usize,
    /// Units against which the proportional constant is checked.
    pub(super) constant_units: usize,
    /// Optional replacement for the currency's global proportional ceiling.
    pub(super) ceiling: Option<f64>,
}

/// One measured run of a cell body: every meter and its denominators.
pub(super) struct Sample {
    /// The default proportional units: encoded input bytes, or `n_io` for an
    /// I/O-denominated cell. Segments use one absolute unit instead.
    pub(super) denom_bytes: usize,
    /// The default growth units.
    ///
    /// `denom_bytes` everywhere except the flat-denominator shape's
    /// input-denominated cells, where it is the bundle's value content: the
    /// encoded denominator is intercept-dominated there, and a two-point
    /// power-law fit against an intercept-dominated denominator manufactures
    /// exponents out of exactly linear marginal work. A resource model may
    /// replace this axis for one currency.
    pub(super) exp_denom_bytes: usize,
    /// The cell's liveness declarations; each sample carries its own since
    /// floors scale with the sample's operands.
    pub(super) floors: Floors,
    /// Resource models that differ from the global linear defaults.
    pub(super) models: ByCurrency<Option<Model>>,
    /// Every currency's counter reading over the body; `None` where the counter
    /// is not compiled in (the feature-gated scan and touch columns
    /// render `off` and are exempt from judgment).
    pub(super) readings: ByCurrency<Option<u64>>,
}

/// Run one prepared cell under all meters.
///
/// The denominators are settled after the meters are read and before the result
/// is dropped: an I/O-denominated cell's output side comes from the actual
/// result rather than a prediction.
pub(super) fn measure(
    heap: &HeapMeter,
    _op: &'static str,
    cell: Cell,
    content: Option<usize>,
) -> Sample {
    meter::reset_stack_segments();
    reset_scan();
    reset_touch();
    (heap.reset_peak)();
    let baseline = (heap.current)();
    let result = (cell.body)();
    let peak_heap = (heap.peak)().saturating_sub(baseline);
    let segments = meter::stack_segments();
    let scan = read_scan();
    let touch = read_touch();
    let (denom_bytes, exp_denom_bytes) = match cell.denom {
        // The flat-denominator shape's content denominator carries the exponent
        // legs of its input-denominated cells alone: an I/O-denominated cell's
        // output side already scales.
        Denom::Input => {
            let exp = content.unwrap_or(cell.input_bytes);
            (cell.input_bytes, exp)
        }
        Denom::Io(spec) => {
            let output_bytes = (spec.output_bytes)(result.as_ref());
            let n_io = cell.input_bytes + output_bytes;
            (n_io, n_io)
        }
    };
    drop(result);
    let resolve = |currency: Currency, spec: Option<ModelSpec>| -> Option<Model> {
        let spec = spec?;
        let ordinary_constant = if currency == Currency::Segments {
            1
        } else {
            denom_bytes
        };
        let units = |units: Units, ordinary: usize| -> usize {
            let resolved = match units {
                Units::Default => ordinary,
                Units::Scale(factor) => ((ordinary as f64) * factor).ceil() as usize,
                Units::Explicit(units) => units,
            };
            assert!(resolved > 0, "resource-model units are positive");
            resolved
        };
        let trend_units = units(spec.trend, exp_denom_bytes);
        let constant_units = units(spec.constant, ordinary_constant);
        assert!(
            trend_units != exp_denom_bytes
                || constant_units != ordinary_constant
                || spec.ceiling.is_some(),
            "a resource-model override must change a unit axis or ceiling"
        );
        Some(Model {
            trend_units,
            constant_units,
            ceiling: spec.ceiling,
        })
    };
    let models = ByCurrency {
        heap: resolve(Currency::Heap, cell.models.heap),
        segments: resolve(Currency::Segments, cell.models.segments),
        scan: resolve(Currency::Scan, cell.models.scan),
        touch: resolve(Currency::Touch, cell.models.touch),
    };
    Sample {
        denom_bytes,
        exp_denom_bytes,
        floors: cell.floors,
        models,
        readings: ByCurrency {
            heap: Some(peak_heap as u64),
            segments: Some(segments),
            scan,
            touch,
        },
    }
}

/// Reset the touch counter when the `touch-meter` feature carries one.
#[cfg(feature = "touch-meter")]
fn reset_touch() {
    suanpan::touch_meter::reset();
}

/// Without the `touch-meter` feature there is no touch counter to reset.
#[cfg(not(feature = "touch-meter"))]
fn reset_touch() {}

/// Read the touch counter, or `None` without the `touch-meter` feature.
#[cfg(feature = "touch-meter")]
fn read_touch() -> Option<u64> {
    Some(suanpan::touch_meter::touches())
}

/// Without the `touch-meter` feature the touch column is absent.
#[cfg(not(feature = "touch-meter"))]
fn read_touch() -> Option<u64> {
    None
}

/// Reset the scan counter when the `scan-meter` feature carries one.
#[cfg(feature = "scan-meter")]
fn reset_scan() {
    meter::reset_scan_bits();
}

/// Without the `scan-meter` feature there is no counter to reset.
#[cfg(not(feature = "scan-meter"))]
fn reset_scan() {}

/// Read the scan counter, or `None` without the `scan-meter` feature.
#[cfg(feature = "scan-meter")]
fn read_scan() -> Option<u64> {
    Some(meter::scan_bits())
}

/// Without the `scan-meter` feature the scan column is absent.
#[cfg(not(feature = "scan-meter"))]
fn read_scan() -> Option<u64> {
    None
}
