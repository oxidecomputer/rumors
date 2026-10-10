//! The measurement engine: run one prepared cell under every meter and capture
//! its counter readings and settled denominators as a [`Sample`].

#[cfg(feature = "scan-meter")]
use crate::testing::meter;

use super::cell::{Cell, Denom, ModelSpec, Units};
use super::currency::{ByCurrency, Floors};

/// The peak-heap meter the board reads, supplied by the binary that runs it.
///
/// A counting global allocator is per-binary state the library cannot own, so
/// the runner (the `amp_board` example, the smoke test) installs one and passes
/// readers in. The board calls `reset_peak` before each cell's body and reads
/// `peak` once the body has produced its result, so a cell's heap reading is
/// the most its body raised live bytes above their level at the reset.
///
/// The readers need only agree with each other: a runner may count
/// process-wide, provided no other thread allocates while a cell is measured,
/// or count the measuring thread alone.
pub struct HeapMeter {
    /// Begin a fresh peak reading at the current live level.
    pub reset_peak: fn(),
    /// Return the most that live bytes have risen above their level at the
    /// last `reset_peak`, or zero if they never rose above it.
    pub peak: fn() -> usize,
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
    /// The default proportional units: input bytes, or `n_io` for an
    /// I/O-denominated cell.
    pub(super) denom_bytes: usize,
    /// The default growth units.
    ///
    /// `denom_bytes` everywhere except the flat-denominator shape's
    /// input-denominated cells, where it is the bundle's value content: the
    /// byte denominator is intercept-dominated there, and a two-point
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
/// result rather than a prediction. The peak includes result allocations as
/// well as scratch; releasing an input during the operation can reduce the
/// extra live heap relative to its level at the reset.
pub(super) fn measure(heap: &HeapMeter, mut cell: Cell, content: Option<usize>) -> Sample {
    reset_scan();
    reset_touch();
    (heap.reset_peak)();
    let mut observation = None;
    (cell.body)(&mut |result| {
        let readings = ByCurrency {
            heap: Some((heap.peak)() as u64),
            scan: read_scan(),
            touch: read_touch(),
        };
        let (denom_bytes, exp_denom_bytes) = match &cell.denom {
            // The flat-denominator shape's content denominator carries the exponent
            // legs of its input-denominated cells alone: an I/O-denominated cell's
            // output side already scales.
            Denom::Input => {
                let exp = content.unwrap_or(cell.input_bytes);
                (cell.input_bytes, exp)
            }
            Denom::Io(spec) => {
                let output_bytes = (spec.output_bytes)(result);
                let n_io = cell.input_bytes + output_bytes;
                (n_io, n_io)
            }
        };
        observation = Some((readings, denom_bytes, exp_denom_bytes));
    });
    let (readings, denom_bytes, exp_denom_bytes) =
        observation.expect("a cell observes its result once");
    let resolve = |spec: Option<ModelSpec>| -> Option<Model> {
        let spec = spec?;
        let ordinary_constant = denom_bytes;
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
        heap: resolve(cell.models.heap),
        scan: resolve(cell.models.scan),
        touch: resolve(cell.models.touch),
    };
    Sample {
        denom_bytes,
        exp_denom_bytes,
        floors: cell.floors,
        models,
        readings,
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
