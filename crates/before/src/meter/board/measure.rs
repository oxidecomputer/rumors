//! The measurement engine: run one prepared cell under every meter and
//! capture its counter readings and settled denominators as a [`Sample`].

use crate::meter;

use super::ceilings::{capacity_chain_peak, TEXT_PIPELINE_LIMB_OPS_PER_VALUE};
use super::cell::{assert_honest_text, Cell, Denom};
use super::currency::{ByCurrency, Floors};

/// The peak-heap meter the board reads, supplied by the binary that runs it.
///
/// A counting global allocator is per-binary state the library cannot own,
/// so the runner (the `amp_board` example, the smoke test) installs one and
/// passes readers in. All three read the runner's allocator: `reset_peak`
/// clears the peak high-water mark, `peak` reads it, `current` reads live
/// bytes (the baseline subtracted from the peak).
pub struct HeapMeter {
    /// Clear the peak high-water mark down to current usage.
    pub reset_peak: fn(),
    /// The peak live bytes since the last reset.
    pub peak: fn() -> usize,
    /// The currently live bytes.
    pub current: fn() -> usize,
}

/// One measured run of a cell body: every meter and its denominators.
pub(super) struct Sample {
    /// The denominator of the heap and segment constants (and, on most
    /// cells, of every exponent): packed input bytes, or `n_io` for the
    /// I/O-denominated cells.
    pub(super) denom_bytes: usize,
    /// The exponent legs' denominator.
    ///
    /// `denom_bytes` everywhere except the flat-denominator shape's
    /// input-denominated cells, where it is the bundle's value content:
    /// the packed denominator is intercept-dominated there, and a
    /// two-point power-law fit against an intercept-dominated denominator
    /// manufactures exponents out of exactly linear marginal work.
    pub(super) exp_denom_bytes: usize,
    /// The limb *constant*'s denominator: `denom_bytes`, or `R` for the
    /// text rows (the limb exponent is judged against `denom_bytes` on
    /// every row).
    pub(super) limb_denom: u64,
    /// Whether the limb column is judged at the text ceiling κ.
    pub(super) text_row: bool,
    /// The cell's liveness declarations; each sample carries its own since
    /// floors scale with the sample's operands.
    pub(super) floors: Floors,
    /// The fold rows' operand count at this sample's scale, for the
    /// declared `FoldLog` model.
    pub(super) fold_arity: Option<u64>,
    /// The party fold's declared search allowance at this sample's
    /// scale, in scan bits.
    pub(super) fold_search_bits: u64,
    /// The capacity-chain model's predicted peak heap for this sample
    /// ([`capacity_chain_peak`] over the actual input and output bytes),
    /// on the cells that declare it.
    pub(super) heap_model: Option<f64>,
    /// The family-stated flat heap ceiling, on the cells that declare
    /// one (the `ceilings` module's declared-models section).
    pub(super) declared_heap: Option<f64>,
    /// The family-stated limb model `(exponent ceiling, per-radix-unit
    /// constant ceiling)`, on the cells that declare one (the
    /// `ceilings` module's declared-models section).
    pub(super) declared_limb: Option<(f64, f64)>,
    /// Every currency's counter reading over the body; `None` where the
    /// counter is not compiled in (the feature-gated limb, scan, and
    /// touch columns render `off` and are exempt from judgment).
    pub(super) readings: ByCurrency<Option<u64>>,
}

/// Run one prepared cell under all meters.
///
/// The denominators are settled after the meters are read and before the
/// result is dropped: an I/O-denominated cell's output side comes from the
/// actual result (never from a prediction), and a text output is checked
/// against the honesty ceiling right here.
pub(super) fn measure(
    heap: &HeapMeter,
    op: &'static str,
    cell: Cell,
    content: Option<usize>,
) -> Sample {
    meter::reset_stack_segments();
    reset_limb();
    reset_scan();
    reset_touch();
    (heap.reset_peak)();
    let baseline = (heap.current)();
    let result = (cell.body)();
    let peak_heap = (heap.peak)().saturating_sub(baseline);
    let segments = meter::stack_segments();
    let limb = read_limb();
    let scan = read_scan();
    let touch = read_touch();
    let mut heap_model = None;
    let (denom_bytes, exp_denom_bytes, limb_denom, text_row) = match cell.denom {
        // The flat-denominator shape's content denominator carries the
        // exponent legs of its input-denominated cells alone: an
        // I/O-denominated cell's output side already scales.
        Denom::Input => {
            let exp = content.unwrap_or(cell.input_bytes);
            (cell.input_bytes, exp, cell.input_bytes as u64, false)
        }
        Denom::Io(spec) => {
            let output_bytes = (spec.output_bytes)(result.as_ref());
            if cell.capacity_model {
                heap_model = Some(capacity_chain_peak(cell.input_bytes, output_bytes));
            }
            let n_io = cell.input_bytes + output_bytes;
            match spec.text {
                None => (n_io, n_io, n_io as u64, false),
                Some(text) => {
                    if text.output_is_text {
                        assert_honest_text(op, output_bytes, text.radix_units);
                    }
                    let pipeline = TEXT_PIPELINE_LIMB_OPS_PER_VALUE * text.spelled_values;
                    (n_io, n_io, n_io as u64 + text.radix_units + pipeline, true)
                }
            }
        }
    };
    drop(result);
    Sample {
        denom_bytes,
        exp_denom_bytes,
        limb_denom,
        text_row,
        floors: cell.floors,
        fold_arity: cell.fold_arity,
        fold_search_bits: cell.fold_search_bits,
        heap_model,
        declared_heap: cell.declared_heap,
        declared_limb: cell.declared_limb,
        readings: ByCurrency {
            heap: Some(peak_heap as u64),
            segments: Some(segments),
            limb,
            scan,
            touch,
        },
    }
}

/// Reset the limb counter when the `limb-meter` feature carries one.
#[cfg(feature = "limb-meter")]
fn reset_limb() {
    meter::reset_limb_ops();
}

/// Without the `limb-meter` feature there is no counter to reset.
#[cfg(not(feature = "limb-meter"))]
fn reset_limb() {}

/// Read the limb counter, or `None` without the `limb-meter` feature.
#[cfg(feature = "limb-meter")]
fn read_limb() -> Option<u64> {
    Some(meter::limb_ops())
}

/// Without the `limb-meter` feature the limb column is absent.
#[cfg(not(feature = "limb-meter"))]
fn read_limb() -> Option<u64> {
    None
}

/// Reset the touch counter when the `limb-meter` feature carries one.
#[cfg(feature = "limb-meter")]
fn reset_touch() {
    suanpan::touch_meter::reset();
}

/// Without the `limb-meter` feature there is no touch counter to reset.
#[cfg(not(feature = "limb-meter"))]
fn reset_touch() {}

/// Read the touch counter, or `None` without the `limb-meter` feature.
#[cfg(feature = "limb-meter")]
fn read_touch() -> Option<u64> {
    Some(suanpan::touch_meter::touches())
}

/// Without the `limb-meter` feature the touch column is absent.
#[cfg(not(feature = "limb-meter"))]
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
